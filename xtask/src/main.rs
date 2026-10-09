mod assurance;
#[cfg(test)]
mod tests;
use anyhow::{bail, Context, Result};
use assurance::{load_ledger, source_digest, validate_ledger, Ledger};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Debug, Deserialize, Serialize)]
struct Pins {
    rust: String,
    verus: String,
    vstd: String,
    vest: String,
    duvet: String,
    z3: String,
    vstd_sha256: String,
    vest_lib_sha256: String,
}
#[derive(Debug, Serialize)]
struct Tools {
    verus: PathBuf,
    cargo_verus: PathBuf,
    vest: PathBuf,
    duvet: PathBuf,
}

struct Outcome<'a> {
    action: &'a str,
    digest: &'a str,
    release: bool,
    tests: bool,
}

fn main() {
    if let Err(error) = entry() {
        eprintln!("xtask: {error:#}");
        std::process::exit(1);
    }
}
fn entry() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    env::set_current_dir(root)?;
    let mut args = env::args().skip(1);
    let action = args.next().unwrap_or_else(|| "help".into());
    let rest: Vec<_> = args.collect();
    if matches!(action.as_str(), "help" | "--help") {
        println!("cargo xtask <doctor|generate|build|verify|trace|trace-update|test|producer|ci> [--release]\nbuild verifies by default; check aliases ci; traceability aliases trace.\nSet VERUS, CARGO_VERUS, VEST and DUVET or source tools/env.sh.");
        return Ok(());
    }
    if rest.iter().any(|arg| arg != "--release") || rest.len() > 1 {
        bail!("unsupported arguments: {rest:?}");
    }
    let release = !rest.is_empty();
    let ledger = load_ledger(root)?;
    let pins: Pins = toml::from_str(&fs::read_to_string(root.join("tools/toolchain.toml"))?)?;
    let tools = tools(root);
    match action.as_str() {
        "doctor" => doctor(root, &tools, &pins),
        "generate" => {
            doctor(root, &tools, &pins)?;
            generate(root, &ledger, &tools, true)
        }
        "trace-update" => {
            ensure_duvet(root, &tools.duvet, &pins)?;
            run(root, &tools.duvet, &["report", "--ci=false"])
        }
        "trace" | "traceability" => {
            ensure_duvet(root, &tools.duvet, &pins)?;
            trace(root, &ledger, &tools)
        }
        "test" => run(
            root,
            Path::new("cargo"),
            &["test", "--workspace", "--locked"],
        ),
        "producer" => {
            let script = root.join("tools/producer.sh");
            if !script.is_file() {
                bail!(
                    "official-producer regeneration is pending; required fixtures are not skipped"
                );
            }
            run(
                root,
                Path::new("bash"),
                &[script.to_str().context("non-UTF8 producer path")?],
            )
        }
        "build" | "verify" | "ci" | "check" => {
            let full = matches!(action.as_str(), "ci" | "check");
            doctor(root, &tools, &pins)?;
            validate_ledger(root, &ledger)?;
            generate(root, &ledger, &tools, false)?;
            trace(root, &ledger, &tools)?;
            let before = source_digest(root)?;
            verify(root, &tools, &action, release)?;
            if full {
                run(root, Path::new("cargo"), &["fmt", "--all", "--check"])?;
                run(
                    root,
                    Path::new("cargo"),
                    &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--locked",
                        "--",
                        "-D",
                        "warnings",
                    ],
                )?;
                run(
                    root,
                    Path::new("cargo"),
                    &["test", "--workspace", "--locked"],
                )?;
                validate_named_tests(root, &ledger)?;
            }
            let after = source_digest(root)?;
            if before != after {
                bail!("sources changed during verification; no evidence recorded");
            }
            evidence(
                root,
                &ledger,
                &pins,
                &tools,
                Outcome {
                    action: &action,
                    digest: &after,
                    release,
                    tests: full,
                },
            )
        }
        _ => bail!("unknown command {action:?}; use cargo xtask help"),
    }
}
fn tools(root: &Path) -> Tools {
    let cache = env::var_os("DUPLICITY_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".tools"));
    let find = |var: &str, local: PathBuf, fallback: &str| {
        env::var_os(var).map(PathBuf::from).unwrap_or_else(|| {
            if local.exists() {
                local
            } else {
                PathBuf::from(fallback)
            }
        })
    };
    let verus = find("VERUS", cache.join("verus-x86-linux/verus"), "verus");
    let cargo_verus = find(
        "CARGO_VERUS",
        verus.parent().unwrap_or(Path::new(".")).join("cargo-verus"),
        "cargo-verus",
    );
    Tools {
        verus,
        cargo_verus,
        vest: find("VEST", cache.join("bin/vest"), "vest"),
        duvet: find("DUVET", cache.join("bin/duvet"), "duvet"),
    }
}
fn capture(root: &Path, executable: &Path, args: &[&str]) -> Result<Output> {
    let output = Command::new(executable)
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "cannot run {} (required tool missing?)",
                executable.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "{} {args:?} failed ({}):\n{}\n{}",
            executable.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}
fn run(root: &Path, executable: &Path, args: &[&str]) -> Result<()> {
    println!("Running {} {}", executable.display(), args.join(" "));
    let status = Command::new(executable)
        .current_dir(root)
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "cannot run {} (required tool missing?)",
                executable.display()
            )
        })?;
    if !status.success() {
        bail!("{} failed ({status})", executable.display());
    }
    Ok(())
}
fn executable_path(executable: &Path) -> Result<PathBuf> {
    if executable.components().count() > 1 {
        return Ok(executable.to_path_buf());
    }
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(executable))
        .find(|file| file.is_file())
        .context("cannot locate executable on PATH")
}
fn ensure_duvet(root: &Path, executable: &Path, pins: &Pins) -> Result<()> {
    capture(root, executable, &["query", "--help"])?;
    // Duvet 0.4.3 has no --version; use Cargo's installation receipt.
    let actual = executable_path(executable)?;
    let receipt = actual
        .parent()
        .and_then(Path::parent)
        .context("Duvet install root unavailable")?
        .join(".crates2.json");
    let installed: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt).with_context(|| {
            format!(
                "Duvet {} install receipt required at {}; run tools/bootstrap.sh",
                pins.duvet,
                receipt.display()
            )
        })?)?;
    let prefix = format!("duvet {} ", pins.duvet);
    if !installed["installs"]
        .as_object()
        .is_some_and(|items| items.keys().any(|id| id.starts_with(&prefix)))
    {
        bail!("Duvet receipt does not identify version {}", pins.duvet);
    }
    Ok(())
}
fn doctor(root: &Path, tools: &Tools, pins: &Pins) -> Result<()> {
    let metadata: serde_json::Value = serde_json::from_slice(
        &capture(
            root,
            Path::new("cargo"),
            &["metadata", "--locked", "--format-version", "1"],
        )?
        .stdout,
    )?;
    let lock: toml::Value = toml::from_str(&fs::read_to_string(root.join("Cargo.lock"))?)?;
    assurance::validate_dependencies(
        &metadata,
        &lock,
        &[
            ("vstd", &pins.vstd, &pins.vstd_sha256),
            ("vest_lib", &pins.vest, &pins.vest_lib_sha256),
        ],
    )?;
    let json: serde_json::Value = serde_json::from_slice(
        &capture(root, &tools.verus, &["--version", "--output-json"])?.stdout,
    )?;
    if json["verus"]["version"].as_str() != Some(&pins.verus) {
        bail!(
            "Verus {} required; found {}. Set VERUS or source tools/env.sh",
            pins.verus,
            json["verus"]["version"]
        );
    }
    let rust = String::from_utf8(capture(root, Path::new("rustc"), &["--version"])?.stdout)?;
    if !rust.starts_with(&format!("rustc {} ", pins.rust)) {
        bail!("Rust {} required; found {rust}", pins.rust);
    }
    if String::from_utf8(capture(root, &tools.vest, &["--version"])?.stdout)?.trim()
        != format!("vest {}", pins.vest)
    {
        bail!("Vest {} required", pins.vest);
    }
    capture(root, &tools.cargo_verus, &["--help"])?;
    let verus = executable_path(&tools.verus)?;
    let solver = verus.parent().unwrap().join("z3");
    if !String::from_utf8(capture(root, &solver, &["--version"])?.stdout)?
        .starts_with(&format!("Z3 version {} ", pins.z3))
    {
        bail!("Z3 {} required alongside Verus", pins.z3);
    }
    ensure_duvet(root, &tools.duvet, pins)?;
    println!(
        "Pinned tools ready: Rust {}, Verus {}, vstd {}, Vest {}, Duvet {}, Z3 {}",
        pins.rust, pins.verus, pins.vstd, pins.vest, pins.duvet, pins.z3
    );
    Ok(())
}
fn generate(root: &Path, ledger: &Ledger, tools: &Tools, update: bool) -> Result<()> {
    let temp = root.join("target/assurance/generated");
    fs::create_dir_all(&temp)?;
    for (index, item) in ledger.generation.iter().enumerate() {
        let produced = temp.join(format!("{index}.rs"));
        capture(
            root,
            &tools.vest,
            &[
                &item.input,
                "--output",
                produced.to_str().context("non-UTF8 temp path")?,
            ],
        )?;
        let expected = root.join(&item.output);
        let mut bytes = fs::read(&produced)?;
        bytes.extend(item.append.as_bytes());
        if update {
            fs::create_dir_all(expected.parent().context("output parent missing")?)?;
            fs::write(expected, bytes)?;
        } else if fs::read(&expected).ok().as_deref() != Some(bytes.as_slice()) {
            bail!(
                "stale generated parser {}; run cargo xtask generate and review",
                item.output
            );
        }
    }
    Ok(())
}
//= docs/parsing-requirements.md#assure-trace-003
//# The acceptance gate MUST compare Duvet's derived requirement snapshot with a reviewed snapshot committed to the repository.
//= docs/parsing-requirements.md#assure-trace-004
//# Each completed milestone MUST pass Duvet implementation and test coverage checks for its declared requirement sections.
fn trace(root: &Path, ledger: &Ledger, tools: &Tools) -> Result<()> {
    run(root, &tools.duvet, &["report", "--ci"])?;
    for item in ledger.milestone.iter().filter(|item| item.gate) {
        for requirement in &item.requirements {
            let section = format!(
                "{}#{}",
                ledger.specification,
                requirement.to_ascii_lowercase()
            );
            run(
                root,
                &tools.duvet,
                &[
                    "query",
                    "--check",
                    "implementation,test",
                    "--section",
                    &section,
                ],
            )?;
        }
    }
    Ok(())
}
//= docs/parsing-requirements.md#assure-build-001
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
fn verify(root: &Path, tools: &Tools, action: &str, release: bool) -> Result<()> {
    let subcommand = if action == "verify" {
        "verify"
    } else {
        "build"
    };
    let mut args = vec![
        subcommand,
        "--workspace",
        "--locked",
        "--check-toolchain",
        "--fwd-verus-args-to",
        "roots",
    ];
    if release {
        args.push("--release");
    }
    args.extend(["--", "--no-cheating"]);
    let mut cmd = Command::new(&tools.cargo_verus);
    cmd.current_dir(root).args(args);
    let verus = executable_path(&tools.verus)?;
    let mut paths = vec![verus.parent().unwrap().to_path_buf()];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    cmd.env("PATH", env::join_paths(paths)?);
    cmd.env_remove("VERUSFLAGS")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().context("cannot run pinned cargo-verus")?;
    if !status.success() {
        bail!("verified {subcommand} failed ({status}); no successful evidence recorded");
    }
    Ok(())
}
fn validate_named_tests(root: &Path, ledger: &Ledger) -> Result<()> {
    for milestone in ledger.milestone.iter().filter(|item| item.gate) {
        for test in &milestone.tests {
            let listing = String::from_utf8(
                capture(
                    root,
                    Path::new("cargo"),
                    &["test", "--locked", "-p", &test.package, "--", "--list"],
                )?
                .stdout,
            )?;
            if !listing
                .lines()
                .any(|line| line == format!("{}: test", test.name))
            {
                bail!(
                    "required test {} in {} does not exist",
                    test.name,
                    test.package
                );
            }
            let result = capture(
                root,
                Path::new("cargo"),
                &[
                    "test",
                    "--locked",
                    "-p",
                    &test.package,
                    &test.name,
                    "--",
                    "--exact",
                ],
            )?;
            let output = String::from_utf8(result.stdout)?;
            if !output
                .lines()
                .any(|line| line == format!("test {} ... ok", test.name))
            {
                bail!(
                    "required test {} did not execute successfully (ignored or filtered)",
                    test.name
                );
            }
        }
    }
    Ok(())
}
//= docs/parsing-requirements.md#assure-trace-002
//# Formal proof obligations and successful verification results MUST be recorded separately by requirement ID with the verified source identity and tool configuration.
fn evidence(
    root: &Path,
    ledger: &Ledger,
    pins: &Pins,
    tools: &Tools,
    outcome: Outcome<'_>,
) -> Result<()> {
    let Outcome {
        action,
        digest,
        release,
        tests,
    } = outcome;
    let mut hashes = serde_json::Map::new();
    for (name, executable) in [
        ("verus", &tools.verus),
        ("cargo_verus", &tools.cargo_verus),
        ("vest", &tools.vest),
        ("duvet", &tools.duvet),
    ] {
        hashes.insert(
            name.into(),
            assurance::digest_bytes(&fs::read(executable_path(executable)?)?).into(),
        );
    }
    let evidence = serde_json::json!({"schema":1,"command":action,"source_sha256":digest,"tools":pins,"executables":tools,"executable_sha256":hashes,
        "target":format!("{}-{}", env::consts::ARCH, env::consts::OS),"configuration":{"release":release,"features":"default","locked":true,"project_no_cheating":true},
        "verification":"passed","tests":if tests {"passed"} else {"not-run"},
        "milestones":ledger.milestone.iter().filter(|item|item.gate).map(|item|&item.id).collect::<Vec<_>>(),
        "proof_obligations":ledger.milestone.iter().filter(|item|item.gate).flat_map(|item|item.proofs.iter()).collect::<Vec<_>>()});
    let path = root.join(format!("target/assurance/{action}.json"));
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, serde_json::to_vec_pretty(&evidence)?)?;
    println!("Evidence: {} (source {digest})", path.display());
    Ok(())
}

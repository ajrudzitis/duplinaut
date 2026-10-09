use anyhow::{bail, Context, Result};
use proc_macro2::{TokenStream, TokenTree};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub schema: u32,
    pub baseline_enabled: bool,
    pub specification: String,
    pub algorithm_sources: Vec<String>,
    #[serde(default)]
    pub generation: Vec<Generation>,
    pub milestone: Vec<Milestone>,
    pub trusted: Vec<Trusted>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    pub input: String,
    pub output: String,
    #[serde(default)]
    pub append: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    pub id: String,
    pub gate: bool,
    pub requirements: Vec<String>,
    pub specified: bool,
    pub implemented: bool,
    pub proved: bool,
    pub tested: bool,
    pub reviewed: bool,
    pub proof_required: bool,
    #[serde(default)]
    pub proof_rationale: String,
    pub implementations: Vec<SourceReference>,
    pub proofs: Vec<ProofReference>,
    pub tests: Vec<TestReference>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReference {
    pub path: String,
    pub symbol: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProofReference {
    pub id: String,
    pub requirement: String,
    pub path: String,
    pub symbol: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestReference {
    pub package: String,
    pub name: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trusted {
    pub id: String,
    pub reviewed: bool,
    pub rationale: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VendorPin {
    pub path: String,
    pub tree_sha256: String,
}

pub fn load_ledger(root: &Path) -> Result<Ledger> {
    toml::from_str(&fs::read_to_string(root.join("docs/assurance.toml"))?)
        .context("invalid assurance ledger")
}

pub fn validate_dependencies(
    root: &Path,
    metadata: &serde_json::Value,
    lock: &toml::Value,
    pins: &[(&str, &str, &str)],
    vendor: Option<&VendorPin>,
) -> Result<()> {
    let packages = metadata["packages"]
        .as_array()
        .context("Cargo metadata has no packages")?;
    let core = packages
        .iter()
        .find(|item| item["name"] == "duplicity-core")
        .context("verified core package is missing")?;
    if core["metadata"]["verus"]["verify"] != true {
        bail!("duplicity-core must remain opted into verification");
    }
    let locked = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .context("Cargo.lock has no packages")?;
    for &(name, version, checksum) in pins {
        let matches: Vec<_> = packages
            .iter()
            .filter(|item| item["name"] == name)
            .collect();
        if matches.len() != 1 || matches[0]["version"] != version {
            bail!("unreviewed resolved dependency {name}; require crates.io {version} without path/git patches");
        }
        if name == "vest_lib" && matches[0]["source"].is_null() {
            let approved = vendor
                .context("local Vest library requires an independently reviewed vendor pin")?;
            if approved.path != "vendor/vest_lib" {
                bail!("Vest vendor path must be vendor/vest_lib");
            }
            let actual = Path::new(
                matches[0]["manifest_path"]
                    .as_str()
                    .context("vendor manifest path missing")?,
            )
            .canonicalize()?;
            if actual
                != root
                    .join(&approved.path)
                    .join("Cargo.toml")
                    .canonicalize()?
            {
                bail!("resolved Vest path does not match the reviewed vendor location");
            }
            if vendor_digest(&root.join(&approved.path))? != approved.tree_sha256 {
                bail!("vendored Vest source differs from independently reviewed tree identity");
            }
            // Its crates.io archive identity remains independently pinned, even
            // though Cargo.lock records a local source rather than an archive.
            if checksum != "f3f4d4698b6a56f5b23ae330d294a8d77fbb728fbf6d74946a18e0e46ba6e7a9" {
                bail!("vendor baseline archive identity differs from reviewed vest_lib 0.2.1");
            }
            continue;
        }
        if matches[0]["source"] != "registry+https://github.com/rust-lang/crates.io-index" {
            bail!("unreviewed dependency source for {name}; only the pinned registry or reviewed Vest vendor is accepted");
        }
        let package = locked
            .iter()
            .find(|item| {
                item.get("name").and_then(toml::Value::as_str) == Some(name)
                    && item.get("version").and_then(toml::Value::as_str) == Some(version)
            })
            .context("pinned dependency missing from Cargo.lock")?;
        if package.get("checksum").and_then(toml::Value::as_str) != Some(checksum) {
            bail!("archive identity for {name} does not match independent toolchain pin");
        }
    }
    Ok(())
}

pub fn vendor_digest(root: &Path) -> Result<String> {
    fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            bail!("vendor identity does not accept symlinks");
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                collect(root, &entry?.path(), files)?;
            }
        } else if path.strip_prefix(root)? != Path::new("PROVENANCE.json") {
            files.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let mut hash = Sha256::new();
    for path in files {
        let name = path
            .strip_prefix(root)?
            .to_str()
            .context("non-UTF8 vendor source name")?
            .as_bytes();
        let contents = fs::read(&path)?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name);
        hash.update((contents.len() as u64).to_le_bytes());
        hash.update(contents);
    }
    Ok(format!("{:x}", hash.finalize()))
}
//= docs/parsing-requirements.md#assure-trace-001
//# Every implemented requirement MUST have Duvet links from authored implementation and applicable test evidence to its specification heading and quoted normative text.
pub fn validate_ledger(root: &Path, ledger: &Ledger) -> Result<()> {
    if ledger.schema != 1 {
        bail!("unsupported assurance ledger schema {}", ledger.schema);
    }
    let spec = fs::read_to_string(root.join(&ledger.specification))?;
    let reqs: BTreeSet<_> = spec
        .lines()
        .filter_map(|line| line.strip_prefix("### "))
        .collect();
    if reqs.is_empty() {
        bail!("specification contains no requirement headings");
    }
    if !ledger
        .algorithm_sources
        .iter()
        .any(|path| path == "crates/core/src")
    {
        bail!("algorithm audit must include crates/core/src");
    }
    for required in ["tooling", "delta-wire", "manifest-token", "complete-reader"] {
        if !ledger
            .milestone
            .iter()
            .any(|milestone| milestone.id == required)
        {
            bail!("required milestone {required} is missing");
        }
    }
    if ledger.baseline_enabled && !ledger.milestone.iter().any(|milestone| milestone.gate) {
        bail!("enabled assurance baseline has no active milestone");
    }
    let mut reachable = BTreeSet::new();
    reachable_sources(&root.join("crates/core/src/lib.rs"), &mut reachable)?;
    let mut ids = BTreeSet::new();
    for item in &ledger.milestone {
        if !ids.insert(&item.id) {
            bail!("duplicate milestone {}", item.id);
        }
        for req in &item.requirements {
            if !reqs.contains(req.as_str()) {
                bail!("unknown requirement {req}");
            }
        }
        if item.gate {
            if !(item.specified && item.implemented && item.tested && item.reviewed) {
                bail!(
                    "milestone {} has incomplete specification/implementation/test/review status",
                    item.id
                );
            }
            if item.requirements.is_empty()
                || item.implementations.is_empty()
                || item.tests.is_empty()
            {
                bail!(
                    "milestone {} lacks requirement/implementation/test evidence",
                    item.id
                );
            }
            if item.proof_required && (!item.proved || item.proofs.is_empty()) {
                bail!("milestone {} lacks required proof evidence", item.id);
            }
            if !item.proof_required && item.proof_rationale.trim().is_empty() {
                bail!("milestone {} lacks proof applicability rationale", item.id);
            }
            for implementation in &item.implementations {
                check_symbol(root, &implementation.path, &implementation.symbol, false)?;
            }
            let mut proofs = BTreeSet::new();
            for proof in &item.proofs {
                if !proofs.insert(&proof.id) {
                    bail!("duplicate proof obligation {}", proof.id);
                }
                if !item.requirements.contains(&proof.requirement) {
                    bail!("proof {} targets undeclared requirement", proof.id);
                }
                check_symbol(root, &proof.path, &proof.symbol, true)?;
                let path = root.join(&proof.path).canonicalize()?;
                if !reachable.contains(&path) {
                    bail!("proof {} is not reachable from the verified core in the supported unconditional module/include profile", proof.id);
                }
            }
        } else if item.reviewed
            && item.implemented
            && item.tested
            && (!item.proof_required || item.proved)
        {
            bail!(
                "completed milestone {} omitted from acceptance gates",
                item.id
            );
        }
    }
    for trust in &ledger.trusted {
        if !trust.reviewed || trust.rationale.trim().is_empty() || trust.requirements.is_empty() {
            bail!("unreviewed or unexplained trusted boundary {}", trust.id);
        }
    }
    for path in &ledger.algorithm_sources {
        if !root.join(path).exists() {
            bail!("algorithm audit source {path} is missing");
        }
        let mut files = Vec::new();
        rust_files(&root.join(path), &mut files)?;
        for file in files {
            let tokens: TokenStream = fs::read_to_string(&file)?
                .parse()
                .map_err(|error| anyhow::anyhow!("cannot tokenize {}: {error}", file.display()))?;
            audit_tokens(tokens)
                .with_context(|| format!("project algorithm audit {}", file.display()))?;
        }
    }
    Ok(())
}
fn rust_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            rust_files(&entry?.path(), out)?;
        }
    } else if path.extension().is_some_and(|ext| ext == "rs") {
        out.push(path.into());
    }
    Ok(())
}

fn contains_ident(tokens: TokenStream, needle: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(id) => id == needle,
        TokenTree::Group(group) => contains_ident(group.stream(), needle),
        _ => false,
    })
}

// Conservative default-configuration reachability. Conditional source files and
// custom module paths need explicit future support, rather than accidental credit.
fn reachable_sources(file: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    let file = file
        .canonicalize()
        .with_context(|| format!("missing core module {}", file.display()))?;
    if out.contains(&file) {
        return Ok(());
    }
    let source = fs::read_to_string(&file)?;
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| anyhow::anyhow!("cannot tokenize core module: {error}"))?;
    if contains_ident(tokens.clone(), "cfg") || contains_ident(tokens.clone(), "cfg_attr") {
        return Ok(());
    }
    out.insert(file.clone());
    let parent = file.parent().context("core source parent missing")?;
    let module_dir = if matches!(
        file.file_name().and_then(|name| name.to_str()),
        Some("lib.rs" | "mod.rs")
    ) {
        parent.to_path_buf()
    } else {
        parent.join(file.file_stem().context("module name missing")?)
    };
    reachable_tokens(tokens, &module_dir, parent, out)
}

fn reachable_tokens(
    tokens: TokenStream,
    module_dir: &Path,
    include_dir: &Path,
    out: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            reachable_tokens(group.stream(), module_dir, include_dir, out)?;
        }
        if matches!(token, TokenTree::Ident(id) if id == "include")
            && matches!(tokens.get(index+1), Some(TokenTree::Punct(p)) if p.as_char() == '!')
        {
            if let Some(TokenTree::Group(group)) = tokens.get(index + 2) {
                let argument = group.stream().to_string();
                let path: String = serde_json::from_str(&argument)
                    .context("proof inclusion requires a literal include path")?;
                reachable_sources(&include_dir.join(path), out)?;
            }
        }
        if matches!(token, TokenTree::Ident(id) if id == "mod") {
            if let Some(TokenTree::Ident(name)) = tokens.get(index + 1) {
                if matches!(tokens.get(index+2), Some(TokenTree::Punct(p)) if p.as_char() == ';') {
                    let flat = module_dir.join(format!("{name}.rs"));
                    let file = if flat.exists() {
                        flat
                    } else {
                        module_dir.join(name.to_string()).join("mod.rs")
                    };
                    reachable_sources(&file, out)?;
                }
            }
        }
    }
    Ok(())
}
fn audit_tokens(tokens: TokenStream) -> Result<()> {
    for token in tokens {
        match token {
            TokenTree::Group(group) => audit_tokens(group.stream())?,
            TokenTree::Ident(id)
                if [
                    "assume",
                    "admit",
                    "axiom",
                    "assume_specification",
                    "external_body",
                    "external",
                    "exec_allows_no_decreases",
                    "assume_termination",
                    "trusted",
                ]
                .contains(&id.to_string().as_str()) =>
            {
                bail!("prohibited proof shortcut {id}")
            }
            _ => {}
        }
    }
    Ok(())
}
fn has_function(tokens: TokenStream, symbol: &str, proof: bool) -> bool {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            if has_function(group.stream(), symbol, proof) {
                return true;
            }
        }
        if matches!(token, TokenTree::Ident(id) if id == "fn")
            && matches!(tokens.get(index+1), Some(TokenTree::Ident(id)) if id == symbol)
            && (!proof
                || matches!(index.checked_sub(1).and_then(|i|tokens.get(i)), Some(TokenTree::Ident(id)) if id == "proof"))
        {
            return true;
        }
    }
    false
}
pub fn check_symbol(root: &Path, path: &str, symbol: &str, proof: bool) -> Result<()> {
    let source = fs::read_to_string(root.join(path))
        .with_context(|| format!("missing evidence source {path}"))?;
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| anyhow::anyhow!("cannot tokenize evidence {path}: {error}"))?;
    if !has_function(tokens, symbol, proof) {
        bail!(
            "required {}function {symbol} not declared in {path}",
            if proof { "proof " } else { "" }
        );
    }
    Ok(())
}
pub fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn source_digest(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()?;
    if !output.status.success() {
        bail!("cannot enumerate repository sources for evidence");
    }
    let mut paths: Vec<_> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .collect();
    paths.sort();
    paths.dedup();
    let mut hash = Sha256::new();
    for bytes in paths {
        let path = std::str::from_utf8(bytes).context("non-UTF8 repository source path")?;
        let file = root.join(path);
        if !file.is_file() {
            continue;
        }
        let contents = fs::read(file)?;
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
        hash.update((contents.len() as u64).to_le_bytes());
        hash.update(contents);
    }
    Ok(format!("{:x}", hash.finalize()))
}

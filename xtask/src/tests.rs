use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = env::temp_dir().join(format!(
            "duplicity-assurance-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn duvet() -> PathBuf {
    tools(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).duvet
}
fn fixture() -> Temp {
    let temp = Temp::new();
    fs::create_dir(temp.0.join(".duvet")).unwrap();
    fs::write(
        temp.0.join("spec.md"),
        "# Fixture\n\n## Required behavior\n\nThe reader MUST reject malformed input.\n",
    )
    .unwrap();
    fs::write(temp.0.join(".duvet/config.toml"), "'$schema' = \"https://awslabs.github.io/duvet/config/v0.4.0.json\"\n[[specification]]\nsource = \"spec.md\"\nformat = \"markdown\"\n[[source]]\npattern = \"*.rs\"\n[report.snapshot]\nenabled = true\npath = \".duvet/snapshot.txt\"\n[report.html]\nenabled = false\n").unwrap();
    temp
}
fn annotation(kind: &str) -> String {
    // Construct marker strings so Duvet does not mistake examples for project evidence.
    format!("{}= spec.md#required-behavior\n{}= type={kind}\n{}# The reader MUST reject malformed input.\n", "//", "//", "//")
}
fn duvet_cmd(root: &Path, args: &[&str]) -> Output {
    Command::new(duvet())
        .current_dir(root)
        .args(args)
        .output()
        .expect("pinned Duvet required for assurance tests")
}

//= docs/parsing-requirements.md#assure-trace-003
//= type=test
//# The acceptance gate MUST compare Duvet's derived requirement snapshot with a reviewed snapshot committed to the repository.
#[test]
fn duvet_detects_changed_snapshot() {
    let temp = fixture();
    fs::write(
        temp.0.join("implementation.rs"),
        format!("{}fn read() {{}}\n", annotation("implementation")),
    )
    .unwrap();
    assert!(duvet_cmd(&temp.0, &["report", "--ci=false"])
        .status
        .success());
    assert!(duvet_cmd(&temp.0, &["report", "--ci"]).status.success());
    fs::write(temp.0.join(".duvet/snapshot.txt"), "corrupt baseline\n").unwrap();
    assert!(!duvet_cmd(&temp.0, &["report", "--ci"]).status.success());
}

//= docs/parsing-requirements.md#assure-trace-001
//= type=test
//# Every implemented requirement MUST have Duvet links from authored implementation and applicable test evidence to its specification heading and quoted normative text.
//= docs/parsing-requirements.md#assure-trace-004
//= type=test
//# Each completed milestone MUST pass Duvet implementation and test coverage checks for its declared requirement sections.
#[test]
fn duvet_requires_actual_implementation_and_test_annotations() {
    let temp = fixture();
    fs::write(
        temp.0.join("implementation.rs"),
        format!("{}fn read() {{}}\n", annotation("todo")),
    )
    .unwrap();
    assert!(!duvet_cmd(&temp.0, &["query", "-c", "implementation,test"])
        .status
        .success());
    fs::write(
        temp.0.join("implementation.rs"),
        format!("{}fn read() {{}}\n", annotation("implementation")),
    )
    .unwrap();
    assert!(!duvet_cmd(&temp.0, &["query", "-c", "implementation,test"])
        .status
        .success());
    fs::write(
        temp.0.join("test.rs"),
        format!("{}#[test]\nfn test_read() {{}}\n", annotation("test")),
    )
    .unwrap();
    assert!(duvet_cmd(&temp.0, &["query", "-c", "implementation,test"])
        .status
        .success());
    fs::write(temp.0.join("test.rs"), "fn unrelated() {}\n").unwrap();
    assert!(!duvet_cmd(&temp.0, &["query", "-c", "implementation,test"])
        .status
        .success());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn missing_verification_tool_is_failure() {
    let temp = Temp::new();
    assert!(capture(&temp.0, &temp.0.join("no-such-verifier"), &["--version"]).is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn unsuccessful_child_cannot_create_success() {
    let temp = Temp::new();
    assert!(run(&temp.0, Path::new("sh"), &["-c", "exit 7"]).is_err());
    assert!(!temp.0.join("target/assurance/build.json").exists());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn stale_generated_parser_rejected() {
    let temp = Temp::new();
    fs::write(temp.0.join("simple.vest"), "!BIG_ENDIAN\nheader = u32\n").unwrap();
    let ledger: Ledger = toml::from_str("schema=1\nbaseline_enabled=false\nspecification='spec.md'\nalgorithm_sources=[]\nmilestone=[]\ntrusted=[]\n[[generation]]\ninput='simple.vest'\noutput='generated.rs'\nappend=''").unwrap();
    let tools = tools(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap());
    generate(&temp.0, &ledger, &tools, true).unwrap();
    generate(&temp.0, &ledger, &tools, false).unwrap();
    fs::write(temp.0.join("generated.rs"), "// replaced parser\n").unwrap();
    assert!(generate(&temp.0, &ledger, &tools, false).is_err());
}

//= docs/parsing-requirements.md#assure-trace-002
//= type=test
//# Formal proof obligations and successful verification results MUST be recorded separately by requirement ID with the verified source identity and tool configuration.
#[test]
fn proof_link_requires_declaration_not_a_comment_or_string() {
    let temp = Temp::new();
    fs::write(
        temp.0.join("proof.rs"),
        "// proof fn goal() {}\nconst EXAMPLE: &str = \"proof fn goal() {}\";\n",
    )
    .unwrap();
    assert!(assurance::check_symbol(&temp.0, "proof.rs", "goal", true).is_err());
    fs::write(temp.0.join("proof.rs"), "verus! { proof fn goal() {} }\n").unwrap();
    assurance::check_symbol(&temp.0, "proof.rs", "goal", true).unwrap();
}

fn ledger_fixture(root: &Path) -> Ledger {
    fs::create_dir_all(root.join("crates/core/src")).unwrap();
    fs::write(root.join("crates/core/src/lib.rs"), "mod algorithm;\n").unwrap();
    fs::write(root.join("crates/core/src/algorithm.rs"), "// fixture\n").unwrap();
    fs::write(
        root.join("spec.md"),
        "### REQ-001\n\nReader MUST meet its contract.\n",
    )
    .unwrap();
    let mut text = String::from("schema=1\nbaseline_enabled=false\nspecification='spec.md'\nalgorithm_sources=['crates/core/src']\n[[trusted]]\nid='fixture'\nreviewed=true\nrationale='Unit test fixture'\nrequirements=['REQ-001']\n");
    for id in ["tooling", "delta-wire", "manifest-token", "complete-reader"] {
        text.push_str(&format!("[[milestone]]\nid='{id}'\ngate=false\nrequirements=['REQ-001']\nspecified=true\nimplemented=false\nproved=false\ntested=false\nreviewed=false\nproof_required=true\nimplementations=[]\nproofs=[]\ntests=[]\n"));
    }
    toml::from_str(&text).unwrap()
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn unreviewed_trust_and_project_proof_shortcuts_rejected() {
    let temp = Temp::new();
    let mut ledger = ledger_fixture(&temp.0);
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { proof fn goal() {} }\n",
    )
    .unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
    ledger.trusted[0].reviewed = false;
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.trusted[0].reviewed = true;
    for shortcut in [
        "assume",
        "admit",
        "assume_specification",
        "external_body",
        "exec_allows_no_decreases",
    ] {
        fs::write(
            temp.0.join("crates/core/src/algorithm.rs"),
            format!("verus! {{ proof fn goal() {{ {shortcut}(true); }} }}\n"),
        )
        .unwrap();
        assert!(
            validate_ledger(&temp.0, &ledger).is_err(),
            "accepted {shortcut}"
        );
    }
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "// assume in an explanation\nconst TEXT: &str = \"admit\";\n",
    )
    .unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn incomplete_or_nonexistent_required_proof_rejected() {
    let temp = Temp::new();
    let mut ledger = ledger_fixture(&temp.0);
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { exec fn decode() {} proof fn valid() {} }\n",
    )
    .unwrap();
    let milestone: assurance::Milestone = toml::from_str("id='tooling'\ngate=true\nrequirements=['REQ-001']\nspecified=true\nimplemented=true\nproved=false\ntested=true\nreviewed=true\nproof_required=true\nimplementations=[{path='crates/core/src/algorithm.rs',symbol='decode'}]\nproofs=[]\ntests=[{package='xtask',name='fixture'}]\n").unwrap();
    ledger.milestone[0] = milestone;
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.milestone[0].proved = true;
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.milestone[0].proofs.push(assurance::ProofReference {
        id: "required-proof".into(),
        requirement: "REQ-001".into(),
        path: "crates/core/src/algorithm.rs".into(),
        symbol: "missing".into(),
    });
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.milestone[0].proofs[0].symbol = "valid".into();
    validate_ledger(&temp.0, &ledger).unwrap();
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn missing_milestones_empty_requirements_and_disabled_audit_rejected() {
    let temp = Temp::new();
    let mut ledger = ledger_fixture(&temp.0);
    validate_ledger(&temp.0, &ledger).unwrap();
    let removed = ledger.milestone.remove(1);
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.milestone.insert(1, removed);
    ledger.algorithm_sources.clear();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.algorithm_sources.push("crates/core/src".into());
    ledger.baseline_enabled = true;
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.baseline_enabled = false;
    fs::write(temp.0.join("spec.md"), "# Empty specification\n").unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn unreachable_and_conditional_proof_declarations_rejected() {
    let temp = Temp::new();
    let mut ledger = ledger_fixture(&temp.0);
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { exec fn decode() {} proof fn valid() {} }\n",
    )
    .unwrap();
    ledger.milestone[0] = toml::from_str("id='tooling'\ngate=true\nrequirements=['REQ-001']\nspecified=true\nimplemented=true\nproved=true\ntested=true\nreviewed=true\nproof_required=true\nimplementations=[{path='crates/core/src/algorithm.rs',symbol='decode'}]\nproofs=[{id='wire',requirement='REQ-001',path='crates/core/src/algorithm.rs',symbol='valid'}]\ntests=[{package='xtask',name='fixture'}]\n").unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "// algorithm intentionally omitted\n",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "#[cfg(any())] mod algorithm;\n",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    fs::write(temp.0.join("crates/core/src/lib.rs"), "mod algorithm;\n").unwrap();
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { exec fn decode() {} #[cfg(any())] proof fn valid() {} }\n",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn verification_opt_in_and_upstream_identity_required() {
    let mut metadata = serde_json::json!({"packages":[
        {"name":"duplicity-core","metadata":{"verus":{"verify":true}}},
        {"name":"vstd","version":"1","source":"registry+https://github.com/rust-lang/crates.io-index"},
        {"name":"vest_lib","version":"2","source":"registry+https://github.com/rust-lang/crates.io-index"}
    ]});
    let lock: toml::Value = toml::from_str("[[package]]\nname='vstd'\nversion='1'\nchecksum='a'\n[[package]]\nname='vest_lib'\nversion='2'\nchecksum='b'\n").unwrap();
    let pins = [("vstd", "1", "a"), ("vest_lib", "2", "b")];
    assurance::validate_dependencies(&metadata, &lock, &pins).unwrap();
    metadata["packages"][0]["metadata"]["verus"]["verify"] = false.into();
    assert!(assurance::validate_dependencies(&metadata, &lock, &pins).is_err());
    metadata["packages"][0]["metadata"]["verus"]["verify"] = true.into();
    metadata["packages"][1]["source"] = serde_json::Value::Null;
    assert!(assurance::validate_dependencies(&metadata, &lock, &pins).is_err());
    metadata["packages"][1]["source"] =
        "registry+https://github.com/rust-lang/crates.io-index".into();
    metadata["packages"][1]["version"] = "unreviewed".into();
    assert!(assurance::validate_dependencies(&metadata, &lock, &pins).is_err());
    metadata["packages"][1]["version"] = "1".into();
    assert!(assurance::validate_dependencies(
        &metadata,
        &lock,
        &[("vstd", "1", "wrong"), ("vest_lib", "2", "b")]
    )
    .is_err());
}

//= docs/parsing-requirements.md#assure-trace-002
//= type=test
//# Formal proof obligations and successful verification results MUST be recorded separately by requirement ID with the verified source identity and tool configuration.
#[test]
fn source_digest_changes_with_source_but_not_ignored_artifacts() {
    let temp = Temp::new();
    assert!(Command::new("git")
        .current_dir(&temp.0)
        .args(["init", "--quiet"])
        .status()
        .unwrap()
        .success());
    fs::write(temp.0.join(".gitignore"), "/target/\n").unwrap();
    fs::write(temp.0.join("source.rs"), "fn read() {}\n").unwrap();
    let before = source_digest(&temp.0).unwrap();
    fs::create_dir(temp.0.join("target")).unwrap();
    fs::write(temp.0.join("target/result"), "test result\n").unwrap();
    assert_eq!(before, source_digest(&temp.0).unwrap());
    fs::write(temp.0.join("source.rs"), "fn read() { panic!(); }\n").unwrap();
    assert_ne!(before, source_digest(&temp.0).unwrap());
}

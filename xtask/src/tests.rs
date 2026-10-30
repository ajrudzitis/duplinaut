use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = env::temp_dir()
            .canonicalize()
            .expect("temporary directory should resolve to its physical path")
            .join(format!(
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
        kind: assurance::ProofKind::Lemma,
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
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "#[cfg(any())] use missing::Ghost; verus! { exec fn decode() {} proof fn valid() {} }\n",
    )
    .unwrap();
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "#[cfg(any())] mod absent; mod algorithm;\n",
    )
    .unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "#![cfg(any())] verus! { exec fn decode() {} proof fn valid() {} }\n",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { exec fn decode() {} proof fn valid() {} }\n",
    )
    .unwrap();
    for root in [
        "#[cfg_attr(any(), path=\"algorithm.rs\")] mod algorithm;",
        "#[path=\"algorithm.rs\"] mod alias;",
        "#[cfg(any())] include!(\"algorithm.rs\");",
        "fn unused() { include!(\"algorithm.rs\"); }",
        "unknown_macro! { mod algorithm; }",
    ] {
        fs::write(temp.0.join("crates/core/src/lib.rs"), root).unwrap();
        assert!(
            validate_ledger(&temp.0, &ledger).is_err(),
            "accepted unsupported root: {root}"
        );
    }
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "include!(\"algorithm.rs\");",
    )
    .unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
    for source in [
        "verus! { #![cfg(any())] exec fn decode() {} proof fn valid() {} }",
        "verus! { #![cfg_attr(any(), allow(dead_code))] exec fn decode() {} proof fn valid() {} }",
        "#![cfg(any())] verus! { exec fn decode() {} proof fn valid() {} }",
    ] {
        fs::write(temp.0.join("crates/core/src/algorithm.rs"), source).unwrap();
        assert!(validate_ledger(&temp.0, &ledger).is_err());
        assert!(assurance::check_proof_symbol(
            &temp.0,
            "crates/core/src/algorithm.rs",
            "valid",
            assurance::ProofKind::Lemma
        )
        .is_err());
    }
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { exec fn decode() {} proof fn valid() {} }",
    )
    .unwrap();
    let inline = temp.0.join("crates/core/src/inline");
    fs::create_dir_all(inline.join("body")).unwrap();
    fs::write(inline.join("body.rs"), "mod algorithm;").unwrap();
    fs::write(inline.join("algorithm.rs"), "fn actual_unverified() {}").unwrap();
    fs::write(
        inline.join("body/algorithm.rs"),
        "verus! { proof fn valid() {} }",
    )
    .unwrap();
    ledger.milestone[0].proofs[0].path = "crates/core/src/inline/body/algorithm.rs".into();
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "include!(\"inline/body.rs\");",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    #[cfg(unix)]
    {
        let src = temp.0.join("crates/core/src");
        fs::create_dir_all(src.join("logical")).unwrap();
        fs::create_dir_all(src.join("physical/parent")).unwrap();
        fs::write(src.join("physical/parent.rs"), "pub mod child;").unwrap();
        fs::write(src.join("logical/child.rs"), "fn actual_unverified() {}").unwrap();
        fs::write(
            src.join("physical/parent/child.rs"),
            "verus! { proof fn valid() {} }",
        )
        .unwrap();
        std::os::unix::fs::symlink("physical/parent.rs", src.join("logical.rs")).unwrap();
        ledger.milestone[0].proofs[0].path = "crates/core/src/physical/parent/child.rs".into();
        fs::write(src.join("lib.rs"), "mod logical;").unwrap();
        assert!(validate_ledger(&temp.0, &ledger).is_err());
        // Directory aliases are excluded as source-path ancestors too.
        std::os::unix::fs::symlink("physical", src.join("alias")).unwrap();
        fs::write(src.join("lib.rs"), "include!(\"alias/parent.rs\");").unwrap();
        ledger.milestone[0].proofs[0].path = "crates/core/src/alias/parent.rs".into();
        fs::write(
            src.join("physical/parent.rs"),
            "verus! { proof fn valid() {} }",
        )
        .unwrap();
        assert!(validate_ledger(&temp.0, &ledger).is_err());
    }
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
    assurance::validate_dependencies(Path::new("."), &metadata, &lock, &pins, None).unwrap();
    metadata["packages"][0]["metadata"]["verus"]["verify"] = false.into();
    assert!(
        assurance::validate_dependencies(Path::new("."), &metadata, &lock, &pins, None).is_err()
    );
    metadata["packages"][0]["metadata"]["verus"]["verify"] = true.into();
    metadata["packages"][1]["source"] = serde_json::Value::Null;
    assert!(
        assurance::validate_dependencies(Path::new("."), &metadata, &lock, &pins, None).is_err()
    );
    metadata["packages"][1]["source"] =
        "registry+https://github.com/rust-lang/crates.io-index".into();
    metadata["packages"][1]["version"] = "unreviewed".into();
    assert!(
        assurance::validate_dependencies(Path::new("."), &metadata, &lock, &pins, None).is_err()
    );
    metadata["packages"][1]["version"] = "1".into();
    assert!(assurance::validate_dependencies(
        Path::new("."),
        &metadata,
        &lock,
        &[("vstd", "1", "wrong"), ("vest_lib", "2", "b")],
        None,
    )
    .is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn only_exact_reviewed_vendor_source_is_accepted() {
    let temp = Temp::new();
    let vendor = temp.0.join("vendor/vest_lib");
    fs::create_dir_all(vendor.join("src")).unwrap();
    fs::write(
        vendor.join("Cargo.toml"),
        "[package]\nname='vest_lib'\nversion='0.2.1'\n",
    )
    .unwrap();
    fs::write(vendor.join("src/lib.rs"), "// reviewed fixture bytes\n").unwrap();
    let mut metadata = serde_json::json!({"packages":[
        {"name":"duplicity-core","metadata":{"verus":{"verify":true}}},
        {"name":"vest_lib","version":"0.2.1","source":null,"manifest_path":vendor.join("Cargo.toml")}
    ]});
    let lock: toml::Value = toml::from_str("package=[]\n").unwrap();
    let pins = [(
        "vest_lib",
        "0.2.1",
        "f3f4d4698b6a56f5b23ae330d294a8d77fbb728fbf6d74946a18e0e46ba6e7a9",
    )];
    assert!(assurance::validate_dependencies(&temp.0, &metadata, &lock, &pins, None).is_err());
    let approved = assurance::VendorPin {
        path: "vendor/vest_lib".into(),
        tree_sha256: assurance::vendor_digest(&vendor).unwrap(),
    };
    assurance::validate_dependencies(&temp.0, &metadata, &lock, &pins, Some(&approved)).unwrap();
    fs::write(vendor.join("PROVENANCE.json"), "mutable self-attestation\n").unwrap();
    assurance::validate_dependencies(&temp.0, &metadata, &lock, &pins, Some(&approved)).unwrap();
    fs::write(vendor.join("src/lib.rs"), "// changed implementation\n").unwrap();
    assert!(
        assurance::validate_dependencies(&temp.0, &metadata, &lock, &pins, Some(&approved))
            .is_err()
    );
    fs::write(vendor.join("src/lib.rs"), "// reviewed fixture bytes\n").unwrap();
    metadata["packages"][1]["source"] = "git+https://example.invalid/unreviewed".into();
    assert!(
        assurance::validate_dependencies(&temp.0, &metadata, &lock, &pins, Some(&approved))
            .is_err()
    );
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

//= docs/parsing-requirements.md#assure-trace-002
//= type=test
//# Formal proof obligations and successful verification results MUST be recorded separately by requirement ID with the verified source identity and tool configuration.
#[test]
fn executable_contract_evidence_requires_verified_mode_and_real_postcondition() {
    use assurance::{check_proof_symbol, ProofKind};
    let temp = Temp::new();
    for source in [
        "verus! { pub fn goal(x: u8) -> (result: u8) ensures result == x, { x } }",
        "verus! { pub exec fn goal(x: u8) -> (result: u8) ensures match result { 0 => x == 0, _ => result == x }, { x } }",
        "verus! { struct Reader; impl Reader { fn goal(x: u8) -> (result: u8) ensures result == x, { x } } }",
        "mod checked { verus! { fn goal(x: u8) -> (result: u8) ensures result == x, { x } } }",
        "#[verus_verify] #[verus_spec(result => ensures result == x)] fn goal(x: u8) -> u8 { x }",
    ] {
        fs::write(temp.0.join("contract.rs"), source).unwrap();
        check_proof_symbol(&temp.0, "contract.rs", "goal", ProofKind::Contract).unwrap();
        assert!(check_proof_symbol(&temp.0, "contract.rs", "goal", ProofKind::Lemma).is_err());
    }
    for source in [
        "fn goal(x: u8) -> u8 { x }",
        "fn goal(x: u8) -> (result: u8) ensures result == x, { x }",
        "verus! { fn goal(x: u8) -> u8 { let ensures = x; ensures } }",
        "verus! { fn goal(x: u8) -> u8 ensures true, { x } }",
        "verus! { fn goal(x: u8) -> u8 ensures (true), true, { x } }",
        "verus! { proof fn goal(x: u8) ensures x == x, {} }",
        "verus! { spec fn goal(x: u8) -> u8 { x } }",
        "verus! { fn goal(x: u8) -> (result: u8) ensures result == x,; }",
        "#[verus_verify] fn goal(x: u8) -> u8 { x }",
        "#[verus_spec(result => ensures result == x)] fn goal(x: u8) -> u8 { x }",
        "#[verus_verify] #[verus_spec(result => ensures true)] fn goal(x: u8) -> u8 { x }",
        "// verus! { fn goal() ensures false {} }\nconst EXAMPLE: &str = \"verus! { fn goal() ensures false {} }\";",
        "quote! { verus! { fn goal(x: u8) -> (result: u8) ensures result == x, { x } } }",
        "macro_rules! template { () => { verus! { fn goal(x: u8) -> (result: u8) ensures result == x, { x } } }; }",
        "fn other() { verus! { fn goal(x: u8) -> (result: u8) ensures result == x, { x } } }",
        "#[cfg(any())] verus! { fn goal(x: u8) -> (result: u8) ensures result == x, { x } }",
    ] {
        fs::write(temp.0.join("contract.rs"), source).unwrap();
        assert!(check_proof_symbol(&temp.0, "contract.rs", "goal", ProofKind::Contract).is_err(), "accepted invalid contract: {source}");
    }
    // The production contract syntax is accepted without treating this check
    // as a substitute for the complete verifier or independent semantic review.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    check_proof_symbol(
        root,
        "crates/core/src/delta.rs",
        "apply_delta",
        ProofKind::Contract,
    )
    .unwrap();
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn executable_contract_evidence_requires_core_inclusion_and_explicit_kind() {
    let temp = Temp::new();
    let mut ledger = ledger_fixture(&temp.0);
    fs::write(
        temp.0.join("crates/core/src/algorithm.rs"),
        "verus! { fn decode(x: u8) -> (result: u8) ensures result == x, { x } }",
    )
    .unwrap();
    ledger.milestone[0] = toml::from_str("id='tooling'\ngate=true\nrequirements=['REQ-001']\nspecified=true\nimplemented=true\nproved=true\ntested=true\nreviewed=true\nproof_required=true\nimplementations=[{path='crates/core/src/algorithm.rs',symbol='decode'}]\nproofs=[{id='wire',requirement='REQ-001',path='crates/core/src/algorithm.rs',symbol='decode',kind='contract'}]\ntests=[{package='xtask',name='fixture'}]\n").unwrap();
    validate_ledger(&temp.0, &ledger).unwrap();
    ledger.milestone[0].proofs[0].kind = assurance::ProofKind::Lemma;
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    ledger.milestone[0].proofs[0].kind = assurance::ProofKind::Contract;
    fs::write(
        temp.0.join("crates/core/src/lib.rs"),
        "// source excluded\n",
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
        "verus! { #[cfg(any())] fn decode(x: u8) -> (result: u8) ensures result == x, { x } }",
    )
    .unwrap();
    assert!(validate_ledger(&temp.0, &ledger).is_err());
    let default: assurance::ProofReference =
        toml::from_str("id='lemma'\nrequirement='REQ-001'\npath='proof.rs'\nsymbol='valid'\n")
            .unwrap();
    assert_eq!(default.kind, assurance::ProofKind::Lemma);
    assert!(toml::from_str::<assurance::ProofReference>(
        "id='bad'\nrequirement='REQ-001'\npath='proof.rs'\nsymbol='valid'\nkind='unknown'\n"
    )
    .is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn generation_fact_activation_preserves_every_original_byte() {
    let item: assurance::Generation = toml::from_str("input='crates/core/formats/manifest_token.vest'\noutput='crates/core/src/generated/manifest_token.rs'\nproof_profile='manifest-token-facts-v1'\n").unwrap();
    let raw = b"// mod exec_impls { fake }\nverus! { mod derived_proofs { broadcast use { original::facts, }; proof fn lemma() {} } mod exec_impls { fn parse() -> u8 { 7 } } }\n";
    let generated = assurance::augment_generation(raw, &item).unwrap();
    let generated = String::from_utf8(generated).unwrap();
    let activation = "\nbroadcast use crate::verified::token_refinement::group_token_facts;\n";
    assert_eq!(generated.matches(activation).count(), 1);
    let list_activation = "\n        crate::verified::token_refinement::group_token_facts,";
    assert_eq!(generated.matches(list_activation).count(), 1);
    assert_eq!(
        generated
            .replace(activation, "")
            .replace(list_activation, "")
            .as_bytes(),
        raw
    );
    for bad in [
        "verus! { mod derived_proofs {} }",
        "verus! { mod derived_proofs {} mod exec_impls {} mod exec_impls {} }",
        "verus! { mod derived_proofs {} #[cfg(any())] mod exec_impls {} }",
        "quote! { verus! { mod derived_proofs {} mod exec_impls {} } }",
        "mod derived_proofs {} mod exec_impls {}",
        "verus! { mod derived_proofs {} mod exec_impls; }",
        "verus! { mod derived_proofs { broadcast use one; } mod exec_impls {} }",
        "verus! { mod derived_proofs { broadcast use {}; } mod exec_impls {} }",
        "verus! { mod derived_proofs { broadcast use { one }; broadcast use { two }; } mod exec_impls {} }",
        "verus! { mod derived_proofs { broadcast use { one }; } mod exec_impls { broadcast use { two }; } }",
    ] {
        assert!(assurance::augment_generation(bad.as_bytes(), &item).is_err());
    }
    let mut wrong = item;
    wrong.output = "other.rs".into();
    assert!(assurance::augment_generation(raw, &wrong).is_err());
    assert!(toml::from_str::<assurance::Generation>(
        "input='a'\noutput='b'\nproof_profile='arbitrary-edit'\n"
    )
    .is_err());
}

//= docs/parsing-requirements.md#assure-build-001
//= type=test
//# The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.
#[test]
fn cached_partial_verification_and_success_without_fresh_proofs_are_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let temp = Temp::new();
    fs::create_dir_all(temp.0.join("target/shared")).unwrap();
    fs::write(
        temp.0.join("target/shared/partial-cache"),
        "previous focused success",
    )
    .unwrap();
    let driver = temp.0.join("fake-cargo-verus");
    fs::write(
        &driver,
        r#"#!/bin/sh
printf '%s\n' "$@" > requested-arguments
while [ "$#" -gt 0 ]; do
    if [ "$1" = '--target-dir' ]; then shift; selected_target=$1; fi
    shift
done
[ -n "$selected_target" ] || exit 19
if [ -f "$selected_target/partial-cache" ]; then
    echo 'verification results:: 1 verified, 0 errors'
    exit 0
fi
# Model Cargo's successful no-op when nothing was actually verified.
if [ -f emit-fresh ]; then
    echo 'verification results:: 32 verified, 0 errors'
fi
exit 0
"#,
    )
    .unwrap();
    fs::set_permissions(&driver, fs::Permissions::from_mode(0o755)).unwrap();
    let tools = Tools {
        cargo_verus: driver.clone(),
        verus: driver.clone(),
        vest: driver.clone(),
        duvet: driver,
    };
    assert!(verify(&temp.0, &tools, "verify", false).is_err());
    fs::write(temp.0.join("emit-fresh"), "").unwrap();
    verify(&temp.0, &tools, "verify", false).unwrap();
    let arguments = fs::read_to_string(temp.0.join("requested-arguments")).unwrap();
    assert!(
        arguments.starts_with("verify\n--workspace\n--locked\n--check-toolchain\n--target-dir\n")
    );
    assert!(arguments.ends_with("--fwd-verus-args-to\nroots\n--\n--no-cheating\n"));
    assert!(!arguments.contains("--verify-module"));
    assert!(!arguments.contains("--verify-only-module"));
    let targets: Vec<_> = fs::read_dir(temp.0.join("target/assurance"))
        .unwrap()
        .collect();
    assert_eq!(targets.len(), 2);
    assert!(temp.0.join("target/shared/partial-cache").exists());
    assert!(verify_in_target(
        &temp.0,
        &tools,
        "verify",
        false,
        &temp.0.join("target/shared")
    )
    .is_err());
    assert!(require_fresh_verification_summary(
        b"verification results:: 0 verified, 0 errors\n",
        b""
    )
    .is_err());
    assert!(require_fresh_verification_summary(
        b"verification results:: 32 verified, 1 errors\n",
        b""
    )
    .is_err());
    assert!(require_fresh_verification_summary(
        b"verification results:: 3183 verified, 0 errors\nverification results:: 535 verified, 0 errors (partial verification with `--verify-*`)\n",
        b""
    ).is_err());
}

#[test]
fn lemma_evidence_rejects_unexpanded_macros_and_ordinary_functions() {
    let temp = Temp::new();
    for source in [
        "proof fn goal() {}",
        "quote! { verus! { proof fn goal() {} } }",
        "fn ordinary() { verus! { proof fn goal() {} } }",
        "verus! { fn goal() {} }",
        "verus! { #[cfg(any())] proof fn goal() {} }",
    ] {
        fs::write(temp.0.join("proof.rs"), source).unwrap();
        assert!(
            assurance::check_symbol(&temp.0, "proof.rs", "goal", true).is_err(),
            "accepted invalid lemma: {source}"
        );
    }
}

#[test]
fn gate_tests_are_grouped_by_package_and_deduplicated() {
    let ledger: Ledger = toml::from_str(
        r#"
schema=1
baseline_enabled=false
specification='spec.md'
algorithm_sources=[]
trusted=[]
[[milestone]]
id='first'
gate=true
requirements=[]
specified=true
implemented=true
proved=false
tested=true
reviewed=true
proof_required=false
implementations=[]
proofs=[]
tests=[{package='core',name='alpha'},{package='core',name='beta'}]
[[milestone]]
id='second'
gate=true
requirements=[]
specified=true
implemented=true
proved=false
tested=true
reviewed=true
proof_required=false
implementations=[]
proofs=[]
tests=[{package='core',name='alpha'},{package='xtask',name='gamma'}]
[[milestone]]
id='inactive'
gate=false
requirements=[]
specified=true
implemented=true
proved=false
tested=false
reviewed=false
proof_required=false
implementations=[]
proofs=[]
tests=[{package='unused',name='not_required'}]
"#,
    )
    .unwrap();

    let grouped = grouped_gate_tests(&ledger);
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped["core"].len(), 2);
    assert!(grouped["core"].contains("alpha"));
    assert!(grouped["core"].contains("beta"));
    assert_eq!(
        grouped["xtask"].iter().cloned().collect::<Vec<_>>(),
        ["gamma"]
    );
    assert!(!grouped.contains_key("unused"));
}

#[test]
fn package_gate_test_validation_rejects_missing_ignored_and_failed_tests() {
    let names = ["required".to_owned()].into_iter().collect();
    assert!(validate_package_test_output("core", &names, "other: test\n", "").is_err());
    assert!(validate_package_test_output(
        "core",
        &names,
        "required: test\n",
        "test required ... ignored\n"
    )
    .is_err());
    assert!(validate_package_test_output(
        "core",
        &names,
        "required: test\n",
        "test required ... FAILED\n"
    )
    .is_err());
    validate_package_test_output("core", &names, "required: test\n", "test required ... ok\n")
        .unwrap();
}

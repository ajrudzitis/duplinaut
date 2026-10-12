use std::process::{Command, Stdio};

#[test]
fn help_describes_capabilities_without_requiring_a_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_duplicity-tui"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("local DIRECTORY"));
    assert!(text.contains("s3 BUCKET"));
    assert!(text.contains("restoration are under construction"));
}

#[test]
fn redirected_input_fails_before_source_access_or_terminal_changes() {
    let output = Command::new(env!("CARGO_BIN_EXE_duplicity-tui"))
        .args(["local", "/this/source/does/not/exist"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("requires a terminal"));
    assert!(!error.contains('\u{1b}'));
}

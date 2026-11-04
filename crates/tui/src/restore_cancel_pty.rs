//! Isolated PTY acceptance for cancellation of a real active restore command.
//! This module is included in the library unit-test binary so the worker's
//! cfg(test)-only Execute gate is available to its ignored child driver.

use duplicity_core::{local::LocalSource, source::Cancellation};
use nix::{
    errno::Errno,
    poll::{poll, PollFd, PollFlags},
    pty::{openpty, Winsize},
    sys::termios::tcgetattr,
    unistd::{read, write},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write as _},
    os::fd::{AsFd, AsRawFd},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const CONTROL_ENV: &str = "DUPLINAUT_RESTORE_CANCEL_CONTROL";
const HISTORY_MARKER: &[u8] = b"duplicity-full.20231114T221320Z.manifest";
const CONFIG: &[u8] = b"duplinaut active restore cancel pty v1\n";
const COMPLETE: &[u8] = b"browser-returned-after-worker-join\n";
const CONTROL_LIMIT: u64 = 512;
const PHASE_TIMEOUT: Duration = Duration::from_secs(30);
const ROWS: u16 = 40;
const COLS: u16 = 240;
const DRIVER: &str = "restore_cancel_pty::gated_restore_browser_helper";

fn publish(control: &Path, name: &str, body: &[u8]) -> io::Result<()> {
    assert!(body.len() as u64 <= CONTROL_LIMIT);
    let path = control.join(name);
    if path.try_exists()? {
        return Err(io::Error::other("duplicate restore-cancel phase marker"));
    }
    let temporary = control.join(format!("{name}.pending"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(body)?;
    drop(file);
    fs::rename(temporary, path)
}

fn marker(control: &Path, name: &str) -> io::Result<Option<Vec<u8>>> {
    let file = match File::open(control.join(name)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(CONTROL_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CONTROL_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control marker exceeds cap",
        ));
    }
    Ok(Some(bytes))
}

fn fixture() -> PathBuf {
    fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .expect("official plain fixture exists")
}

fn fingerprint(root: &Path) -> Vec<(PathBuf, Option<[u8; 32]>)> {
    fn walk(root: &Path, directory: &Path, out: &mut Vec<(PathBuf, Option<[u8; 32]>)>) {
        let mut entries: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                out.push((relative, None));
                walk(root, &path, out);
            } else if kind.is_file() {
                out.push((
                    relative,
                    Some(Sha256::digest(fs::read(path).unwrap()).into()),
                ));
            } else if kind.is_symlink() {
                use std::os::unix::ffi::OsStrExt;
                let target = fs::read_link(path).unwrap();
                out.push((
                    relative,
                    Some(Sha256::digest(target.as_os_str().as_bytes()).into()),
                ));
            } else {
                panic!("unexpected fixture entry type");
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "duplinaut-{label}-pty-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn pump(master: &File, output: &mut Vec<u8>, timeout: u16) {
    let mut descriptors = [PollFd::new(master.as_fd(), PollFlags::POLLIN)];
    poll(&mut descriptors, timeout).expect("poll PTY");
    if descriptors[0]
        .revents()
        .unwrap_or_else(PollFlags::empty)
        .contains(PollFlags::POLLIN)
    {
        let mut chunk = [0u8; 8192];
        match read(master.as_raw_fd(), &mut chunk) {
            Ok(count) => output.extend_from_slice(&chunk[..count]),
            Err(Errno::EIO | Errno::EAGAIN | Errno::EINTR) => {}
            Err(error) => panic!("read PTY: {error}"),
        }
        assert!(
            output.len() <= 8 * 1024 * 1024,
            "PTY transcript exceeds cap"
        );
    }
}

fn wait_screen(
    master: &File,
    child: &mut ChildGuard,
    output: &mut Vec<u8>,
    parser: &mut vt100::Parser,
    phase: &str,
    expected: impl Fn(&str) -> bool,
    control: &Path,
) {
    let deadline = Instant::now() + PHASE_TIMEOUT;
    loop {
        let previous = output.len();
        pump(master, output, 100);
        parser.process(&output[previous..]);
        let screen = parser.screen().contents();
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited during {phase}; screen:\n{screen}; control={:?}",
            fs::read_dir(control)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>()
        );
        if expected(&screen) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out during {phase}; screen:\n{screen}"
        );
    }
}

fn wait_exit(master: &File, child: &mut ChildGuard, output: &mut Vec<u8>) -> Option<ExitStatus> {
    let deadline = Instant::now() + PHASE_TIMEOUT;
    loop {
        pump(master, output, 100);
        if let Some(status) = child.0.try_wait().unwrap() {
            pump(master, output, 0);
            return Some(status);
        }
        if Instant::now() >= deadline {
            return None;
        }
    }
}

#[test]
#[ignore = "internal isolated active-restore PTY driver, spawned by its parent cases"]
fn gated_restore_browser_helper() {
    let control = PathBuf::from(std::env::var_os(CONTROL_ENV).expect("control path required"));
    assert_eq!(
        marker(&control, "driver-config").unwrap().as_deref(),
        Some(CONFIG)
    );
    let source_path = fixture();
    let destination_parent =
        PathBuf::from(std::env::var_os("DUPLINAUT_RESTORE_CANCEL_DEST_PARENT").unwrap());
    let destination_name = b"restored".to_vec();
    let source = std::sync::Arc::new(LocalSource::new(source_path).unwrap());
    let destination = std::sync::Arc::new(duplicity_tui::cli::Destination {
        parent: destination_parent,
        name: destination_name,
    });
    duplicity_tui::browser::run_configured(
        source,
        String::new(),
        Cancellation::default(),
        Some(destination),
    )
    .expect("production browser must complete cancellation and teardown");
    let control_result = marker(&control, "execute-result")
        .unwrap()
        .expect("backend result marker");
    assert!(String::from_utf8_lossy(&control_result)
        .contains("outcome=Unpublished;cause=Cancelled;uncertain_outer=false"));
    assert!(
        !PathBuf::from(std::env::var_os("DUPLINAUT_RESTORE_CANCEL_DEST_PARENT").unwrap())
            .join("restored")
            .exists()
    );
    assert!(
        fs::read_dir(std::env::var_os("DUPLINAUT_RESTORE_CANCEL_DEST_PARENT").unwrap())
            .unwrap()
            .next()
            .is_none()
    );
    publish(&control, "browser-returned", COMPLETE).unwrap();
}

fn check_active_restore_cancel(key: &[u8], post_result: &[u8]) {
    let fixture = fixture();
    let before_source = fingerprint(&fixture);
    let working = Scratch::new("restore-cancel-work");
    let control = Scratch::new("restore-cancel-control");
    let destination_parent = working.0.join("destination-parent");
    fs::create_dir(&destination_parent).unwrap();
    publish(&control.0, "driver-config", CONFIG).unwrap();
    let pty = openpty(
        Some(&Winsize {
            ws_row: ROWS,
            ws_col: COLS,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("open restore-cancel PTY");
    let slave = File::from(pty.slave);
    let original_termios = tcgetattr(&slave).unwrap();
    let master = File::from(pty.master);
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                DRIVER,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CONTROL_ENV, &control.0)
            .env("DUPLINAUT_RESTORE_CANCEL_DEST_PARENT", &destination_parent)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", COLS.to_string())
            .env("LINES", ROWS.to_string())
            .current_dir(&working.0)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()))
            .spawn()
            .expect("spawn isolated browser unit-test driver"),
    );
    let mut output = Vec::new();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "history",
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
        },
        &control.0,
    );
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    write(&master, b"a").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "full anchor",
        |s| s.contains("Full anchor selected; choose target then Enter."),
        &control.0,
    );
    write(&master, b"\x1b[B\x1b[B\r").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "scan confirmation",
        |s| s.contains("Full volume scan required.") && s.contains("Enter confirms."),
        &control.0,
    );
    write(&master, b"\r").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "validated snapshot",
        |s| s.contains("Validated snapshot:"),
        &control.0,
    );
    write(&master, b"r").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "successful executable preview",
        |s| {
            s.contains("Type the exact ASCII word restore") && s.contains("Acknowledgment [] ready")
        },
        &control.0,
    );
    write(&master, b"restore\r").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "running restore and entered gate",
        |s| {
            s.contains("Restoration running.")
                && marker(&control.0, "execute-entered").unwrap().as_deref()
                    == Some(b"operation_id=3;cancelled=false\n")
        },
        &control.0,
    );
    // While the original Execute is held, a second restore-preview key is
    // ignored by the active-operation state. Give the browser several input
    // polling intervals to consume it, preserving the same visible operation
    // and gate marker. The gate's single-entry assertion also rejects any
    // second Execute dispatched after the first one completes.
    write(&master, b"r").unwrap();
    for _ in 0..4 {
        let previous = output.len();
        pump(&master, &mut output, 100);
        parser.process(&output[previous..]);
        let screen = parser.screen().contents();
        assert!(
            screen.contains("Restoration running."),
            "operation changed after extra r input: {screen}"
        );
        assert!(child.0.try_wait().unwrap().is_none());
    }
    assert_eq!(
        marker(&control.0, "execute-entered").unwrap().as_deref(),
        Some(b"operation_id=3;cancelled=false\n")
    );
    assert!(marker(&control.0, "cancel-observed").unwrap().is_none());
    assert!(marker(&control.0, "execute-result").unwrap().is_none());
    assert!(marker(&control.0, "browser-returned").unwrap().is_none());
    write(&master, key).unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "cancellation request observed on Execute token",
        |s| {
            s.contains("Cancellation requested; waiting for operation outcome")
                && marker(&control.0, "cancel-observed").unwrap().as_deref()
                    == Some(b"operation_id=3;cancelled=true\n")
        },
        &control.0,
    );
    assert!(child.0.try_wait().unwrap().is_none());
    assert!(marker(&control.0, "browser-returned").unwrap().is_none());
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    publish(&control.0, "release", b"release operation_id=3\n").unwrap();
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "typed cancelled result",
        |s| {
            s.contains("Restore result.") && s.contains("Unpublished: cause Cancelled")
            && marker(&control.0, "execute-result").unwrap().as_deref() == Some(b"operation_id=3;outcome=Unpublished;cause=Cancelled;uncertain_outer=false\n")
        },
        &control.0,
    );
    assert!(marker(&control.0, "browser-returned").unwrap().is_none());
    assert!(!destination_parent.join("restored").exists());
    assert!(fs::read_dir(&destination_parent).unwrap().next().is_none());
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    if post_result == b"q" {
        write(&master, b"q").unwrap();
    } else {
        write(&master, b"\x1b").unwrap();
        wait_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "cancelled result retained in history",
            |s| {
                s.contains("Restore result retained: Unpublished: cause Cancelled")
                    && s.contains("Snapshot paths")
            },
            &control.0,
        );
        write(&master, b"q").unwrap();
    }
    let status =
        wait_exit(&master, &mut child, &mut output).expect("browser failed to exit boundedly");
    assert!(
        status.success(),
        "isolated browser failed: {status}; output={}",
        String::from_utf8_lossy(&output)
    );
    assert_eq!(
        marker(&control.0, "browser-returned").unwrap().as_deref(),
        Some(COMPLETE)
    );
    assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(output
        .windows(b"\x1b[?1049h".len())
        .any(|w| w == b"\x1b[?1049h"));
    assert!(output
        .windows(b"\x1b[?1049l".len())
        .any(|w| w == b"\x1b[?1049l"));
    assert_eq!(fingerprint(&fixture), before_source);
    assert!(fs::read_dir(&destination_parent).unwrap().next().is_none());
    assert!(fs::read_dir(&working.0).unwrap().all(|entry| {
        let name = entry.unwrap().file_name();
        name == "destination-parent"
    }));
}

//= docs/parsing-requirements.md#restore-ui-004
//= type=test
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-006
//= type=test
//# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
//= docs/parsing-requirements.md#restore-ui-007
//= type=test
//# The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.
//= docs/parsing-requirements.md#restore-ui-008
//= type=test
//# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
#[test]
fn q_cancels_active_restore_through_test_gate_before_backend_filesystem_work_and_joins_worker() {
    check_active_restore_cancel(b"q", b"q");
}

//= docs/parsing-requirements.md#restore-ui-004
//= type=test
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-006
//= type=test
//# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
//= docs/parsing-requirements.md#restore-ui-007
//= type=test
//# The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.
//= docs/parsing-requirements.md#restore-ui-008
//= type=test
//# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
#[test]
fn escape_cancels_active_restore_through_test_gate_before_backend_filesystem_work_and_joins_worker()
{
    check_active_restore_cancel(b"\x1b", b"esc");
}

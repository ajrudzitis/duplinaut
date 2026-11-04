//! Deterministic source-injected production-browser cancellation cases.
//! The default parent tests launch the ignored driver in a separate process;
//! only that child owns crossterm's process-global terminal state.

use crate::pty_acceptance::{
    contains, fingerprint, wait_for_exit, wait_for_screen, ChildGuard, Scratch, HISTORY_MARKER,
};
use duplicity_core::{
    local::LocalSource,
    source::{Cancellation, ObjectSource, SourceError, SourceLimits, StoredObject},
};
use nix::{
    pty::{openpty, Winsize},
    sys::termios::tcgetattr,
    unistd::write,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const DRIVER_TEST: &str = "scan_cancel::gated_scan_browser_helper";
const CONTROL_ENV: &str = "DUPLINAUT_PTY_SCAN_CANCEL_CONTROL";
const REQUIRED_VOLUME: &str = "duplicity-full.20231114T221320Z.vol1.difftar.gz";
const CONFIG: &[u8] = b"duplinaut source-injected scan-cancel driver v1\n";
const ENTERED: &[u8] = b"uncancelled duplicity-full.20231114T221320Z.vol1.difftar.gz\n";
const OBSERVED: &[u8] = b"cancelled duplicity-full.20231114T221320Z.vol1.difftar.gz\n";
const RELEASE: &[u8] = b"release the cancelled volume open\n";
const COMPLETED: &[u8] =
    b"browser returned; entered=true observed=true released=true returned-cancelled=true failure=none\n";
const PHASE_TIMEOUT: Duration = Duration::from_secs(30);
const MARKER_LIMIT: u64 = 256;

// Markers have fixed private names and bounded bodies. Rename publishes each
// complete body atomically; a partially written marker is never an accepted phase.
fn publish(control: &Path, name: &str, body: &[u8]) -> io::Result<()> {
    assert!(body.len() as u64 <= MARKER_LIMIT);
    let path = control.join(name);
    if path.try_exists()? {
        return Err(io::Error::other("duplicate test control phase"));
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

fn marker_matches(control: &Path, name: &str, expected: &[u8]) -> io::Result<bool> {
    let file = match File::open(control.join(name)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let mut body = Vec::new();
    file.take(MARKER_LIMIT + 1).read_to_end(&mut body)?;
    if body.len() as u64 > MARKER_LIMIT || body != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected test control phase contents",
        ));
    }
    Ok(true)
}

#[derive(Debug, Default)]
struct GateState {
    claimed: bool,
    entered: bool,
    observed: bool,
    released: bool,
    returned_cancelled: bool,
    failure: Option<String>,
}

struct GatedLocalSource {
    inner: LocalSource,
    control: PathBuf,
    state: Arc<Mutex<GateState>>,
}

impl GatedLocalSource {
    fn fail(&self, error: impl std::fmt::Display) -> SourceError {
        self.state.lock().unwrap().failure = Some(error.to_string());
        SourceError::Backend("test scan gate failed".into())
    }

    fn gate(&self, cancel: &Cancellation) -> io::Result<()> {
        // This is the exact token supplied to the required volume open, which
        // differs from the browser's initial discovery token. Never cancel it here.
        let scan_cancel = cancel.clone();
        if scan_cancel.is_cancelled() {
            return Err(io::Error::other(
                "scan token already cancelled on gate entry",
            ));
        }
        self.state.lock().unwrap().entered = true;
        publish(&self.control, "gate-entered", ENTERED)?;
        let deadline = Instant::now() + PHASE_TIMEOUT;
        while !scan_cancel.is_cancelled() {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "scan gate did not observe key-requested cancellation",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.state.lock().unwrap().observed = true;
        publish(&self.control, "cancel-observed", OBSERVED)?;
        let deadline = Instant::now() + PHASE_TIMEOUT;
        while !marker_matches(&self.control, "release", RELEASE)? {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "parent did not release the cancelled scan gate",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.state.lock().unwrap().released = true;
        Ok(())
    }
}

impl ObjectSource for GatedLocalSource {
    fn list(
        &self,
        prefix: &str,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError> {
        self.inner.list(prefix, limits, cancel)
    }

    fn open(
        &self,
        object: &StoredObject,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError> {
        if object.key != REQUIRED_VOLUME {
            return self.inner.open(object, limits, cancel);
        }
        {
            let mut state = self.state.lock().unwrap();
            if state.claimed {
                drop(state);
                return Err(self.fail("unexpected required-volume gate reentry"));
            }
            state.claimed = true;
        }
        // Establish the real local identity-checked reader before gating. It
        // remains provisional: no volume bytes are delivered or read by this wrapper.
        let reader = self
            .inner
            .open(object, limits, cancel)
            .map_err(|error| self.fail(error))?;
        if let Err(error) = self.gate(cancel) {
            return Err(self.fail(error));
        }
        drop(reader);
        self.state.lock().unwrap().returned_cancelled = true;
        Err(SourceError::Cancelled)
    }
}

#[test]
#[ignore = "internal isolated PTY driver; explicitly executed by the default cancellation tests"]
fn gated_scan_browser_helper() {
    let control = PathBuf::from(
        std::env::var_os(CONTROL_ENV).expect("isolated scan-cancel driver configuration missing"),
    );
    assert!(marker_matches(&control, "driver-config", CONFIG).unwrap());
    assert_ne!(
        fs::canonicalize(&control).unwrap(),
        fs::canonicalize(std::env::current_dir().unwrap()).unwrap(),
        "IPC must not use the browser working directory"
    );
    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let state = Arc::new(Mutex::new(GateState::default()));
    let source = Arc::new(GatedLocalSource {
        inner: LocalSource::new(fixture).unwrap(),
        control: control.clone(),
        state: state.clone(),
    });
    duplicity_tui::browser::run(source, String::new(), Cancellation::default())
        .expect("production browser failed");
    // run returns after its production drain/join path. Generic worker errors
    // must not turn a gate timeout or IPC failure into successful acceptance.
    let state = state.lock().unwrap();
    assert!(state.claimed && state.entered && state.observed && state.released);
    assert!(state.returned_cancelled);
    assert!(state.failure.is_none(), "gate failure: {:?}", state.failure);
    publish(&control, "browser-returned", COMPLETED).unwrap();
}

fn check_scan_cancellation(key: &[u8]) {
    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let before_source = fingerprint(&fixture);
    let working = Scratch::new();
    let control = Scratch::new();
    publish(&control.0, "driver-config", CONFIG).unwrap();
    let pty = openpty(
        Some(&Winsize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("open cancellation PTY");
    let slave = File::from(pty.slave);
    let original_termios = tcgetattr(&slave).unwrap();
    let master = File::from(pty.master);
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                DRIVER_TEST,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CONTROL_ENV, &control.0)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "240")
            .env("LINES", "40")
            .current_dir(&working.0)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()))
            .spawn()
            .expect("spawn isolated production-browser driver"),
    );
    let mut output = Vec::new();
    let mut parser = vt100::Parser::new(40, 240, 0);
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "history",
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
        },
    );
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    write(&master, b"a").unwrap();
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "anchor selection",
        |screen| screen.contains("Full anchor selected; choose target then Enter."),
    );
    write(&master, b"\x1b[B\x1b[B\r").unwrap();
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "scan disclosure",
        |screen| {
            screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
        },
    );
    write(&master, b"\r").unwrap();
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "gated running scan",
        |screen| {
            screen.contains("Reading and validating every required volume...")
                && marker_matches(&control.0, "gate-entered", ENTERED).unwrap()
        },
    );
    assert!(!marker_matches(&control.0, "cancel-observed", OBSERVED).unwrap());
    assert!(!marker_matches(&control.0, "browser-returned", COMPLETED).unwrap());
    write(&master, key).unwrap();
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "cancel requested while worker held",
        |screen| {
            screen.contains("Cancellation requested; waiting for operation outcome")
                && marker_matches(&control.0, "cancel-observed", OBSERVED).unwrap()
        },
    );
    assert!(child.0.try_wait().unwrap().is_none());
    assert!(!marker_matches(&control.0, "browser-returned", COMPLETED).unwrap());
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(!contains(&output, b"\x1b[?1049l"));
    publish(&control.0, "release", RELEASE).unwrap();
    wait_for_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "cancelled scan result",
        |screen| {
            screen.contains("Snapshot validation failed or was cancelled.")
                && screen.contains("History / metadata only")
                && screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && !screen.contains("Snapshot paths")
                && !screen.contains("Validated snapshot:")
        },
    );
    write(&master, b"q").unwrap();
    let status = wait_for_exit(&master, &mut child, &mut output)
        .expect("isolated browser did not exit after completed cancellation");
    assert!(
        status.success(),
        "isolated browser exited with {status}; output: {}",
        String::from_utf8_lossy(&output)
    );
    assert!(marker_matches(&control.0, "browser-returned", COMPLETED).unwrap());
    assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(contains(&output, b"\x1b[?1049h"));
    assert!(contains(&output, b"\x1b[?1049l"));
    assert_eq!(fingerprint(&fixture), before_source);
    assert!(fs::read_dir(&working.0).unwrap().next().is_none());
}

#[test]
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn q_cancels_gated_scan_and_waits_for_worker_before_exit() {
    check_scan_cancellation(b"q");
}

#[test]
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn escape_cancels_gated_scan_and_waits_for_worker_before_exit() {
    check_scan_cancellation(b"\x1b");
}

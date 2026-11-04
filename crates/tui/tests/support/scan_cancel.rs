//! Deterministic source-injected production-browser cancellation cases.
//! The default parent tests launch the ignored driver in a separate process;
//! only that child owns crossterm's process-global terminal state.

use crate::pty_acceptance::{
    contains, fingerprint, wait_for_exit, wait_for_screen, wait_for_screen_with_diagnostic,
    BoundedStderr, ChildGuard, Scratch, HISTORY_MARKER,
};
use duplicity_core::{
    local::LocalSource,
    source::{Cancellation, ObjectSource, SourceError, SourceLimits, StoredObject},
};
use nix::{
    pty::{openpty, Winsize},
    sys::{
        signal::{kill, Signal},
        termios::tcgetattr,
    },
    unistd::write,
};
use rustix::{
    io::{fcntl_setfd, FdFlags},
    termios::{tcgetwinsize, tcsetwinsize, Winsize as TerminalSize},
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
const OUTPUT_DRIVER_TEST: &str = "scan_cancel::gated_output_failure_browser_helper";
const CONTROL_ENV: &str = "DUPLINAUT_PTY_SCAN_CANCEL_CONTROL";
const REQUIRED_VOLUME: &str = "duplicity-full.20231114T221320Z.vol1.difftar.gz";
const CONFIG: &[u8] = b"duplinaut source-injected scan-cancel driver v1\n";
const ENTERED: &[u8] = b"uncancelled duplicity-full.20231114T221320Z.vol1.difftar.gz\n";
const OBSERVED: &[u8] = b"cancelled duplicity-full.20231114T221320Z.vol1.difftar.gz\n";
const RELEASE: &[u8] = b"release the cancelled volume open\n";
const COMPLETED: &[u8] =
    b"browser returned; entered=true observed=true released=true returned-cancelled=true failure=none\n";
const OUTPUT_CONFIG: &[u8] = b"duplinaut source-injected redraw-output-failure driver v1\n";
const OUTPUT_COMPLETED: &[u8] = b"browser-returned; event=true teardown=true raw_failed=false screen_failed=true cursor_failed=true; entered=true observed=true released=true returned-cancelled=true failure=none\n";
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

#[test]
#[ignore = "internal isolated PTY driver; explicitly executed by the default output-failure test"]
fn gated_output_failure_browser_helper() {
    nix::unistd::setsid().expect("isolate output-failure driver from the test runner session");
    let stdin = File::open("/dev/stdin").expect("open child stdin");
    tcgetattr(&stdin).expect("child stdin must remain attached to its PTY");
    assert!(
        File::open("/dev/tty").is_err(),
        "isolated output-failure child must have no controlling terminal"
    );
    let control = PathBuf::from(
        std::env::var_os(CONTROL_ENV)
            .expect("isolated output-failure driver configuration missing"),
    );
    assert!(marker_matches(&control, "driver-config", OUTPUT_CONFIG).unwrap());
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
    let error = duplicity_tui::browser::run(source, String::new(), Cancellation::default())
        .expect_err("redraw output failure must return a controlled browser error");
    #[cfg(not(target_os = "linux"))]
    let completion = error
        .get_ref()
        .and_then(|payload| payload.downcast_ref::<duplicity_tui::terminal::Completion>())
        .expect("browser error must retain structured terminal completion");
    #[cfg(target_os = "linux")]
    let (event_failed, teardown) = {
        let failure = error
            .get_ref()
            .and_then(|payload| {
                payload.downcast_ref::<duplicity_tui::restore_flow::ShutdownFailure>()
            })
            .expect("Linux browser error must retain structured shutdown failure");
        (failure.terminal.is_some(), failure.teardown.as_ref())
    };
    #[cfg(not(target_os = "linux"))]
    let (event_failed, teardown) = (completion.event.is_some(), completion.teardown.as_ref());
    assert!(
        event_failed,
        "redraw failure must be recorded as an event fault"
    );
    let teardown = teardown.expect("terminal teardown error must be retained");
    let faults = teardown
        .get_ref()
        .and_then(|payload| payload.downcast_ref::<duplicity_tui::terminal::TeardownFaults>())
        .expect("teardown error must retain individual action faults");
    let raw_failed = faults.raw.is_some();
    let screen_failed = faults.screen.is_some();
    let cursor_failed = faults.cursor.is_some();
    assert!(!raw_failed, "input PTY raw mode must be restorable");
    assert!(
        screen_failed && cursor_failed,
        "closed stdout PTY must report failed output teardown"
    );
    {
        let state = state.lock().unwrap();
        assert!(state.claimed && state.entered && state.observed && state.released);
        assert!(state.returned_cancelled);
        assert!(state.failure.is_none(), "gate failure: {:?}", state.failure);
    }
    // A successful exit is a controlled harness result. Skip libtest's normal
    // summary because it would write to the intentionally broken stdout PTY.
    publish(&control, "browser-returned", OUTPUT_COMPLETED).unwrap();
    std::process::exit(0);
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

fn check_redraw_output_failure() {
    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let before_source = fingerprint(&fixture);
    let working = Scratch::new();
    let control = Scratch::new();
    publish(&control.0, "driver-config", OUTPUT_CONFIG).unwrap();
    let size = Winsize {
        ws_row: 40,
        ws_col: 240,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let input = openpty(Some(&size), None).expect("open input PTY");
    fcntl_setfd(&input.master, FdFlags::CLOEXEC).expect("make input PTY master close-on-exec");
    let input_slave = File::from(input.slave);
    fcntl_setfd(&input_slave, FdFlags::CLOEXEC).expect("make parent input PTY slave close-on-exec");
    let original_termios = tcgetattr(&input_slave).unwrap();
    let input_master = File::from(input.master);
    let output = openpty(Some(&size), None).expect("open output PTY");
    fcntl_setfd(&output.master, FdFlags::CLOEXEC).expect("make output PTY master close-on-exec");
    let output_slave = File::from(output.slave);
    fcntl_setfd(&output_slave, FdFlags::CLOEXEC)
        .expect("make parent output PTY slave close-on-exec");
    let output_master = File::from(output.master);
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                OUTPUT_DRIVER_TEST,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CONTROL_ENV, &control.0)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "240")
            .env("LINES", "40")
            .current_dir(&working.0)
            .stdin(Stdio::from(input_slave.try_clone().unwrap()))
            .stdout(Stdio::from(output_slave.try_clone().unwrap()))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn isolated output-failure browser driver"),
    );
    let stderr_reader = child
        .0
        .stderr
        .take()
        .expect("capture isolated child stderr");
    let stderr_capture = BoundedStderr::default();
    let stderr_capture_writer = stderr_capture.clone();
    // Drain continuously so a faulty child cannot block on a full pipe. Keep
    // only a fixed diagnostic prefix and report whether the cap was exceeded.
    let stderr_drain = std::thread::spawn(move || -> io::Result<()> {
        let mut reader = stderr_reader;
        let mut chunk = [0u8; 4096];
        loop {
            let count = reader.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            stderr_capture_writer.append(&chunk[..count]);
        }
        Ok(())
    });
    let mut output_bytes = Vec::new();
    let mut parser = vt100::Parser::new(40, 240, 0);
    wait_for_screen_with_diagnostic(
        &output_master,
        &mut child,
        &mut output_bytes,
        &mut parser,
        "output-failure history",
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
        },
        || stderr_capture.diagnostic(),
    );
    assert_ne!(tcgetattr(&input_slave).unwrap(), original_termios);
    write(&input_master, b"a").unwrap();
    wait_for_screen_with_diagnostic(
        &output_master,
        &mut child,
        &mut output_bytes,
        &mut parser,
        "output-failure anchor selection",
        |screen| screen.contains("Full anchor selected; choose target then Enter."),
        || stderr_capture.diagnostic(),
    );
    write(&input_master, b"\x1b[B\x1b[B\r").unwrap();
    wait_for_screen_with_diagnostic(
        &output_master,
        &mut child,
        &mut output_bytes,
        &mut parser,
        "output-failure scan disclosure",
        |screen| {
            screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
        },
        || stderr_capture.diagnostic(),
    );
    write(&input_master, b"\r").unwrap();
    wait_for_screen_with_diagnostic(
        &output_master,
        &mut child,
        &mut output_bytes,
        &mut parser,
        "output-failure gated scan",
        |screen| {
            screen.contains("Reading and validating every required volume...")
                && marker_matches(&control.0, "gate-entered", ENTERED).unwrap()
        },
        || stderr_capture.diagnostic(),
    );
    assert!(!marker_matches(&control.0, "cancel-observed", OBSERVED).unwrap());
    assert!(!marker_matches(&control.0, "browser-returned", OUTPUT_COMPLETED).unwrap());

    // Prove the isolated child obtains its terminal geometry from the output
    // PTY, then cause a second changed frame after the reader is closed.
    let compact = TerminalSize {
        ws_row: 18,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(&output_slave, compact).expect("resize output PTY to compact geometry");
    assert_eq!(tcgetwinsize(&output_slave).unwrap(), compact);
    parser.screen_mut().set_size(compact.ws_row, compact.ws_col);
    let child_pid = i32::try_from(child.0.id()).expect("child PID exceeds signed range");
    assert!(child_pid > 0, "never send a process-group signal");
    kill(nix::unistd::Pid::from_raw(child_pid), Signal::SIGWINCH)
        .expect("notify isolated child of compact output resize");
    wait_for_screen_with_diagnostic(
        &output_master,
        &mut child,
        &mut output_bytes,
        &mut parser,
        "compact output-failure scan frame",
        |screen| crate::pty_acceptance::frame_at_geometry(screen, compact.ws_row, compact.ws_col),
        || stderr_capture.diagnostic(),
    );

    // Preserve the output slave descriptor for the child, but remove the only
    // reader. A second resize notification makes the next production draw
    // write a complete frame to the closed output PTY.
    drop(output_master);
    let expanded = TerminalSize {
        ws_row: size.ws_row,
        ws_col: size.ws_col,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(&output_slave, expanded).expect("resize output PTY after master close");
    assert_eq!(tcgetwinsize(&output_slave).unwrap(), expanded);
    kill(nix::unistd::Pid::from_raw(child_pid), Signal::SIGWINCH)
        .expect("notify isolated child of post-close output resize");
    let deadline = Instant::now() + PHASE_TIMEOUT;
    while !marker_matches(&control.0, "cancel-observed", OBSERVED).unwrap() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before output-failure cancellation{}",
            stderr_capture.diagnostic()
        );
        assert!(
            Instant::now() < deadline,
            "redraw failure did not cancel the gated scan{}",
            stderr_capture.diagnostic()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "child exited before the held scan gate was released"
    );
    assert!(!marker_matches(&control.0, "browser-returned", OUTPUT_COMPLETED).unwrap());
    publish(&control.0, "release", RELEASE).unwrap();

    let deadline = Instant::now() + PHASE_TIMEOUT;
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "output-failure child did not finish controlled teardown{}",
            stderr_capture.diagnostic()
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    stderr_drain
        .join()
        .expect("stderr drainer thread panicked")
        .expect("read isolated child stderr");
    let (stderr, stderr_overflow) = stderr_capture.snapshot();
    assert!(
        !stderr_overflow,
        "isolated child stderr exceeded its bounded diagnostic cap"
    );
    assert!(
        status.success(),
        "output-failure helper rejected lifecycle: {status}; stderr: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(marker_matches(&control.0, "browser-returned", OUTPUT_COMPLETED).unwrap());
    assert_eq!(tcgetattr(&input_slave).unwrap(), original_termios);
    assert_eq!(fingerprint(&fixture), before_source);
    assert!(fs::read_dir(&working.0).unwrap().next().is_none());
    drop(output_slave);
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

#[test]
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn redraw_output_failure_cancels_scan_and_restores_input_terminal() {
    check_redraw_output_failure();
}

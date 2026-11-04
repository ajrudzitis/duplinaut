use std::process::{Command, Stdio};

#[cfg(unix)]
#[path = "support/scan_cancel.rs"]
mod scan_cancel;

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
    assert!(text.contains("type restore, then Enter"));
    assert!(text.contains("p remains read-only"));
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

#[cfg(unix)]
mod pty_acceptance {
    use nix::{
        errno::Errno,
        poll::{poll, PollFd, PollFlags},
        pty::{openpty, Winsize},
        sys::{
            signal::{kill, Signal},
            termios::tcgetattr,
        },
        unistd::{read, write, Pid},
    };
    #[cfg(target_os = "linux")]
    use rustix::io::{fcntl_setfd, FdFlags};
    use rustix::termios::{tcgetwinsize, tcsetwinsize, Winsize as TerminalSize};
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io,
        os::fd::{AsFd, AsRawFd},
        path::{Path, PathBuf},
        process::{Child, Command, ExitStatus, Stdio},
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    pub(super) const HISTORY_MARKER: &[u8] = b"duplicity-full.20231114T221320Z.manifest";
    pub(super) const STDERR_LIMIT: usize = 64 * 1024;
    const EXIT_TIMEOUT: Duration = Duration::from_secs(15);
    const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

    pub(super) struct ChildGuard(pub(super) Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    pub(super) struct Scratch(pub(super) PathBuf, bool);

    impl Scratch {
        pub(super) fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            for _ in 0..100 {
                let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "duplinaut-history-pty-{}-{serial}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path, false),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create PTY test working directory: {error}"),
                }
            }
            panic!("could not allocate unique PTY test working directory")
        }
    }

    impl Scratch {
        #[cfg(target_os = "linux")]
        fn preserve(&mut self) {
            self.1 = true;
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            if self.1 {
                eprintln!(
                    "preserving PTY acceptance artifacts for inspection: {}",
                    self.0.display()
                );
            } else {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    #[cfg(target_os = "linux")]
    struct PreserveUnexpectedRestore<'a> {
        scratch: &'a mut Scratch,
        parent: PathBuf,
        destination: PathBuf,
    }

    #[cfg(target_os = "linux")]
    impl Drop for PreserveUnexpectedRestore<'_> {
        fn drop(&mut self) {
            let destination_exists = fs::symlink_metadata(&self.destination).is_ok();
            let parent_nonempty = match fs::read_dir(&self.parent) {
                Ok(entries) => entries.into_iter().next().is_some(),
                Err(_) => true,
            };
            if destination_exists || parent_nonempty {
                self.scratch.preserve();
            }
        }
    }

    #[cfg(target_os = "linux")]
    struct PreserveOnFailure<'a>(&'a mut Scratch);

    #[cfg(target_os = "linux")]
    impl Drop for PreserveOnFailure<'_> {
        fn drop(&mut self) {
            if std::thread::panicking() {
                self.0.preserve();
            }
        }
    }

    fn hash(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    pub(super) fn fingerprint(root: &Path) -> Vec<(PathBuf, Option<[u8; 32]>)> {
        fn visit(root: &Path, directory: &Path, result: &mut Vec<(PathBuf, Option<[u8; 32]>)>) {
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
                    result.push((relative, None));
                    visit(root, &path, result);
                } else if kind.is_file() {
                    result.push((relative, Some(hash(&fs::read(path).unwrap()))));
                } else if kind.is_symlink() {
                    use std::os::unix::ffi::OsStrExt;
                    let target = fs::read_link(path).unwrap();
                    result.push((relative, Some(hash(target.as_os_str().as_bytes()))));
                } else {
                    panic!("unexpected fixture filesystem entry type");
                }
            }
        }

        let mut result = Vec::new();
        visit(root, root, &mut result);
        result
    }

    fn pump(master: &File, output: &mut Vec<u8>, timeout: u16) {
        let mut descriptors = [PollFd::new(master.as_fd(), PollFlags::POLLIN)];
        poll(&mut descriptors, timeout).expect("poll PTY master");
        if descriptors[0]
            .revents()
            .unwrap_or_else(PollFlags::empty)
            .contains(PollFlags::POLLIN)
        {
            let mut chunk = [0u8; 8192];
            match read(master.as_raw_fd(), &mut chunk) {
                Ok(count) => output.extend_from_slice(&chunk[..count]),
                Err(Errno::EIO | Errno::EAGAIN | Errno::EINTR) => {}
                Err(error) => panic!("read PTY master: {error}"),
            }
            assert!(
                output.len() <= 8 * 1024 * 1024,
                "PTY output exceeded test cap"
            );
        }
    }

    pub(super) fn contains(output: &[u8], marker: &[u8]) -> bool {
        output.windows(marker.len()).any(|window| window == marker)
    }

    fn wait_for_history(master: &File, child: &mut ChildGuard, output: &mut Vec<u8>) -> bool {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            if child.0.try_wait().unwrap().is_some() {
                pump(master, output, 0);
                return false;
            }
            pump(master, output, 100);
            if contains(output, HISTORY_MARKER) {
                return true;
            }
        }
        false
    }

    pub(super) fn wait_for_exit(
        master: &File,
        child: &mut ChildGuard,
        output: &mut Vec<u8>,
    ) -> Option<ExitStatus> {
        let deadline = Instant::now() + EXIT_TIMEOUT;
        loop {
            pump(master, output, 100);
            if let Some(status) = child.0.try_wait().unwrap() {
                // Drain terminal teardown bytes already queued on the PTY.
                pump(master, output, 0);
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
        }
    }

    // Ratatui redraws only changed cells. Keep a zero-scrollback screen model
    // so later assertions observe the current display, not stale transcript text.
    pub(super) fn wait_for_screen(
        master: &File,
        child: &mut ChildGuard,
        output: &mut Vec<u8>,
        parser: &mut vt100::Parser,
        phase: &str,
        expected: impl Fn(&str) -> bool,
    ) {
        wait_for_screen_with_diagnostic(master, child, output, parser, phase, expected, String::new)
    }

    pub(super) fn wait_for_screen_with_diagnostic(
        master: &File,
        child: &mut ChildGuard,
        output: &mut Vec<u8>,
        parser: &mut vt100::Parser,
        phase: &str,
        expected: impl Fn(&str) -> bool,
        diagnostic: impl Fn() -> String,
    ) {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            let previous = output.len();
            pump(master, output, 100);
            parser.process(&output[previous..]);
            let screen = parser.screen().contents();
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "TUI exited during {phase}; current screen:\n{screen}{}",
                diagnostic()
            );
            if expected(&screen) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out during {phase}; current screen:\n{screen}{}",
                diagnostic()
            );
        }
    }

    #[derive(Clone, Default)]
    pub(super) struct BoundedStderr(Arc<Mutex<StderrState>>);

    #[derive(Default)]
    struct StderrState {
        bytes: Vec<u8>,
        overflow: bool,
    }

    impl BoundedStderr {
        pub(super) fn append(&self, bytes: &[u8]) {
            let mut state = self.0.lock().unwrap();
            let remaining = STDERR_LIMIT - state.bytes.len();
            let retained = bytes.len().min(remaining);
            state.bytes.extend_from_slice(&bytes[..retained]);
            state.overflow |= retained != bytes.len();
        }

        pub(super) fn snapshot(&self) -> (Vec<u8>, bool) {
            let state = self.0.lock().unwrap();
            (state.bytes.clone(), state.overflow)
        }

        pub(super) fn diagnostic(&self) -> String {
            let (bytes, overflow) = self.snapshot();
            format!(
                "; child stderr ({} bytes, overflow={overflow}): {}",
                bytes.len(),
                String::from_utf8_lossy(&bytes)
            )
        }
    }

    fn oracle_components(entry: &serde_json::Value) -> Vec<Vec<u8>> {
        entry["components_hex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| {
                let hex = part.as_str().unwrap();
                assert_eq!(hex.len() % 2, 0);
                (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect()
            })
            .collect()
    }

    fn marked_child_on_screen(screen: &str) -> bool {
        screen.lines().any(|line| {
            line.contains("* Regular")
                && line.trim_end_matches([' ', '│']).ends_with("a/b/child.txt")
        })
    }

    pub(super) fn frame_at_geometry(screen: &str, rows: u16, columns: u16) -> bool {
        let Some(top) = screen.lines().next() else {
            return false;
        };
        let Some(bottom) = screen.lines().nth(usize::from(rows - 1)) else {
            return false;
        };
        // Local model resizing can crop/extend the old display, but cannot draw
        // these corners at the new right and bottom boundaries for the child.
        top.starts_with('┌')
            && top.chars().count() == usize::from(columns)
            && top.ends_with('┐')
            && bottom.starts_with('└')
            && bottom.chars().count() == usize::from(columns)
            && bottom.ends_with('┘')
    }

    fn resize_pty(
        master: &File,
        slave: &File,
        child: &ChildGuard,
        output: &mut Vec<u8>,
        parser: &mut vt100::Parser,
        rows: u16,
        columns: u16,
    ) -> usize {
        // Interpret any immediately queued old frame bytes at the old size.
        // This bounded drain need not eliminate every concurrent-draw race;
        // the caller waits for an eventual complete frame at the new geometry.
        let mut drained = false;
        for _ in 0..128 {
            let previous = output.len();
            pump(master, output, 0);
            parser.process(&output[previous..]);
            if output.len() == previous {
                drained = true;
                break;
            }
        }
        assert!(drained, "PTY output did not quiesce before resize");
        let before_resize = output.len();
        let size = TerminalSize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        tcsetwinsize(slave, size).expect("resize slave PTY");
        assert_eq!(tcgetwinsize(slave).unwrap(), size);
        parser.screen_mut().set_size(rows, columns);
        // The child has slave stdio but no controlling-terminal session. Send
        // SIGWINCH explicitly; automatic foreground-group delivery is untested.
        let pid = i32::try_from(child.0.id()).expect("child PID exceeds signed range");
        assert!(pid > 0, "never send a process-group signal");
        kill(Pid::from_raw(pid), Signal::SIGWINCH).expect("notify child of resize");
        before_resize
    }

    fn check_official_chain_readonly_previews(resize: bool) {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/duplicity-3.0.7/expected.json"
        ))
        .unwrap();
        let entries = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
            .unwrap()["entries"]
            .as_array()
            .unwrap();
        let expected_payload: u64 = entries
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| entry["size"].as_u64().unwrap())
            .sum();
        let child_components = vec![b"a".to_vec(), b"b".to_vec(), b"child.txt".to_vec()];
        let mut paths: Vec<_> = entries.iter().map(oracle_components).collect();
        paths.sort();
        let child_index = paths
            .iter()
            .position(|path| path == &child_components)
            .unwrap();
        let selected_count = entries
            .iter()
            .filter(|entry| {
                let path = oracle_components(entry);
                path.starts_with(&child_components)
                    || (entry["kind"] == "directory" && child_components.starts_with(&path))
            })
            .count();
        assert_eq!(selected_count, 4, "fixture selection changed");

        let fixture = fs::canonicalize(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/plain"),
        )
        .unwrap();
        let before_source = fingerprint(&fixture);
        let scratch = Scratch::new();
        let winsize = Winsize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = openpty(Some(&winsize), None).expect("open PTY");
        let slave = File::from(pty.slave);
        let original_termios = tcgetattr(&slave).expect("read original PTY attributes");
        let master = File::from(pty.master);
        let mut child = ChildGuard(
            Command::new(env!("CARGO_BIN_EXE_duplicity-tui"))
                .arg("local")
                .arg(&fixture)
                .current_dir(&scratch.0)
                .env("TERM", "xterm-256color")
                .env("COLUMNS", "240")
                .env("LINES", "40")
                .stdin(Stdio::from(slave.try_clone().unwrap()))
                .stdout(Stdio::from(slave.try_clone().unwrap()))
                .stderr(Stdio::from(slave.try_clone().unwrap()))
                .spawn()
                .expect("spawn TUI in PTY"),
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
        // The history contains the full set followed by its two incrementals.
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
        assert!(!parser.screen().contents().contains("Validated snapshot:"));
        write(&master, b"\r").unwrap();
        let snapshot_status = format!(
            "Validated snapshot: {} entries, {expected_payload} payload bytes.",
            entries.len()
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "validated snapshot",
            |screen| {
                screen.contains(&snapshot_status)
                    && screen.contains("a/b/child.txt")
                    && screen.contains("raw-\\xff.bin")
            },
        );
        write(&master, b"p").unwrap();
        let whole_status = format!(
            "Read-only preview:{} entries including source ancestors. Nothing is written.",
            entries.len()
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "whole snapshot preview",
            |screen| screen.contains(&whole_status),
        );
        for _ in 0..child_index {
            write(&master, b"\x1b[B").unwrap();
        }
        write(&master, b" ").unwrap();
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "marked child",
            |screen| screen.contains("1 marked roots.") && marked_child_on_screen(screen),
        );
        write(&master, b"p").unwrap();
        let marked_status = format!(
            "Read-only preview:{selected_count} entries including source ancestors. Nothing is written."
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "marked child preview",
            |screen| screen.contains(&marked_status) && !screen.contains(&whole_status),
        );
        if resize {
            for (rows, columns) in [(18, 96), (40, 240)] {
                let before_resize = resize_pty(
                    &master,
                    &slave,
                    &child,
                    &mut output,
                    &mut parser,
                    rows,
                    columns,
                );
                wait_for_screen(
                    &master,
                    &mut child,
                    &mut output,
                    &mut parser,
                    "resized marked snapshot preview",
                    |screen| {
                        frame_at_geometry(screen, rows, columns)
                            && screen.contains("Snapshot paths")
                            && marked_child_on_screen(screen)
                            && screen.contains(&marked_status)
                    },
                );
                assert!(
                    output.len() > before_resize,
                    "resize produced no new output"
                );
                assert_eq!(parser.screen().size(), (rows, columns));
            }
            // The mark could survive even if the cursor silently moved. Space
            // must remove the existing child mark rather than add another root.
            write(&master, b" ").unwrap();
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "unmark retained selected child after resize",
                |screen| screen.contains("0 marked roots.") && !marked_child_on_screen(screen),
            );
            write(&master, b"p").unwrap();
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "whole snapshot preview after resize",
                |screen| screen.contains(&whole_status),
            );
        }
        write(&master, b"q").unwrap();
        let status = wait_for_exit(&master, &mut child, &mut output)
            .expect("TUI did not exit after q within timeout");
        assert!(status.success(), "TUI exited with {status}");
        assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
        assert!(contains(&output, b"\x1b[?1049h"));
        assert!(contains(&output, b"\x1b[?1049l"));
        assert!(fs::read_dir(&scratch.0).unwrap().next().is_none());
        assert_eq!(fingerprint(&fixture), before_source);
    }

    #[cfg(target_os = "linux")]
    fn wait_for_preview_ready_with_initial_unarmed_ack(
        master: &File,
        child: &mut ChildGuard,
        output: &mut Vec<u8>,
        parser: &mut vt100::Parser,
        expected_ready: impl Fn(&str) -> bool,
    ) -> bool {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let mut saw_empty_unarmed = false;
        loop {
            let previous = output.len();
            pump(master, output, 100);
            let mut saw_ready = false;
            // Process every byte in the PTY read, latching the first complete
            // empty/unarmed screen before returning on the later ready frame.
            for byte in &output[previous..] {
                parser.process(std::slice::from_ref(byte));
                let screen = parser.screen().contents();
                saw_empty_unarmed |= screen.contains("Acknowledgment [] unarmed");
                saw_ready |= expected_ready(&screen);
            }
            if saw_ready {
                return saw_empty_unarmed;
            }
            let screen = parser.screen().contents();
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "TUI exited before executable preview was ready; current screen:\n{screen}"
            );
            assert!(
                Instant::now() < deadline,
                "timed out waiting for executable preview; initial empty/unarmed acknowledgment observed={saw_empty_unarmed}; current screen:\n{screen}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    fn assert_preview_has_no_execute_side_effects(
        screen: &str,
        destination: &Path,
        parent: &Path,
        workdir: &Path,
    ) {
        assert!(screen.contains("Acknowledgment [restore] ready"));
        assert!(!screen.contains("Restoration running."));
        assert!(!screen.contains("Published completed tree."));
        assert!(!destination.exists(), "preview must not create destination");
        assert!(
            fs::read_dir(parent).unwrap().next().is_none(),
            "preview must not create a staging entry"
        );
        assert!(
            fs::read_dir(workdir).unwrap().next().is_none(),
            "preview must not write in the temporary working directory"
        );
    }

    #[cfg(target_os = "linux")]
    fn production_linux_executable_preview_survives_pty_resize_without_execute() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/duplicity-3.0.7/expected.json"
        ))
        .expect("parse official source oracle");
        let snapshot_oracle = oracle["snapshots"]
            .as_array()
            .expect("snapshot records")
            .iter()
            .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
            .expect("final plain snapshot oracle");
        let entries = snapshot_oracle["entries"]
            .as_array()
            .expect("final snapshot entries");
        assert_eq!(entries.len(), 21);
        let payload_bytes: u64 = entries
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| entry["size"].as_u64().expect("file byte count"))
            .sum();
        assert_eq!(payload_bytes, 2_621_564);

        let fixture = fs::canonicalize(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/plain"),
        )
        .expect("official plain fixture");
        let source_before = fingerprint(&fixture);
        let mut scratch = Scratch::new();
        let workdir = scratch.0.join("work");
        let parent = scratch.0.join("restore-parent");
        fs::create_dir(&workdir).expect("create empty work directory");
        fs::create_dir(&parent).expect("create empty destination parent");
        let destination_name = "preview-resize-tree";
        let destination = parent.join(destination_name);
        // Declared before ChildGuard so unwind first kills/reaps the child,
        // then this guard preserves the scratch tree if it left any candidate
        // or destination entry behind.
        let _preserve_on_unexpected_restore = PreserveUnexpectedRestore {
            scratch: &mut scratch,
            parent: parent.clone(),
            destination: destination.clone(),
        };
        assert!(fs::read_dir(&parent).unwrap().next().is_none());
        assert!(!destination.exists());

        let original_size = TerminalSize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = openpty(
            Some(&Winsize {
                ws_row: original_size.ws_row,
                ws_col: original_size.ws_col,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .expect("open production preview PTY");
        fcntl_setfd(&pty.master, FdFlags::CLOEXEC).expect("close PTY master on exec");
        let master = File::from(pty.master);
        fcntl_setfd(&pty.slave, FdFlags::CLOEXEC).expect("close parent slave on exec");
        let slave = File::from(pty.slave);
        assert_eq!(tcgetwinsize(&slave).unwrap(), original_size);
        let original_termios = tcgetattr(&slave).expect("read original terminal attributes");

        // Start the production process in a session without an inherited
        // controlling terminal; the test PTY is its only terminal geometry.
        let mut command = Command::new("setsid");
        command
            .arg("--wait")
            .arg(env!("CARGO_BIN_EXE_duplicity-tui"))
            .arg("local")
            .arg(&fixture)
            .arg("--restore-parent")
            .arg(&parent)
            .arg("--restore-name")
            .arg(destination_name)
            .current_dir(&workdir)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "240")
            .env("LINES", "40")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        let mut child = ChildGuard(command.spawn().expect("launch production TUI under setsid"));
        let mut output = Vec::new();
        let mut parser = vt100::Parser::new(original_size.ws_row, original_size.ws_col, 0);

        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "history at 40x240",
            |screen| {
                screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                    && screen.contains("Metadata only. a selects full anchor")
                    && frame_at_geometry(screen, original_size.ws_row, original_size.ws_col)
            },
        );
        assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
        write(&master, b"a").expect("select visible full anchor");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "full-anchor selection",
            |screen| screen.contains("Full anchor selected; choose target then Enter."),
        );
        write(&master, b"\x1b[B\x1b[B\r").expect("select final snapshot target");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "volume-scan disclosure",
            |screen| {
                screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
            },
        );
        write(&master, b"\r").expect("confirm full volume scan");
        let validated = format!(
            "Validated snapshot: {} entries, {payload_bytes} payload bytes.",
            entries.len()
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "validated final snapshot",
            |screen| screen.contains(&validated),
        );

        write(&master, b"r").expect("request whole-snapshot executable preview");
        let parent_text = parent.display().to_string();
        let full_preview_content = |screen: &str| {
            screen.contains("Restore preview / outcome / Up-Down scroll")
                && screen.contains("Snapshot target: 1700172800")
                && screen.contains("Scope: whole snapshot (no marks)")
                && screen.contains("Include exact source directory ancestors; no flattening.")
                && screen.contains("Entries: 21")
                && screen.contains("Regular payload bytes: 2621564")
                && screen.contains(&format!("Requested parent: {parent_text}"))
                && screen.contains("New destination leaf: preview-resize-tree")
                && screen.contains("Atomic no-replacement publication")
                && screen.contains("Backend checks parent policy/filesystem")
                && screen.contains("Preserves ordinary modes, supported exact modification times and raw symlink target bytes.")
                && screen.contains("Ownership, source ACLs/xattrs and crash durability are excluded.")
        };
        let initial_preview_ready = |screen: &str| {
            full_preview_content(screen) && screen.contains("Acknowledgment [] ready")
        };
        let saw_initial_unarmed = wait_for_preview_ready_with_initial_unarmed_ack(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            initial_preview_ready,
        );
        assert!(
            saw_initial_unarmed,
            "successful preview first renders with an empty unarmed acknowledgment"
        );

        // Empty Enter must be refused without Execute, a destination, or stage.
        write(&master, b"\r").expect("submit empty acknowledgment");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "empty acknowledgment refusal",
            |screen| {
                screen.contains("Acknowledgment must be exactly restore, then Enter.")
                    && screen.contains("Restore preview / outcome / Up-Down scroll")
            },
        );
        assert!(!parser.screen().contents().contains("Restoration running."));
        assert!(!parser
            .screen()
            .contents()
            .contains("Published completed tree."));
        assert!(!destination.exists());
        assert!(fs::read_dir(&parent).unwrap().next().is_none());
        assert!(fs::read_dir(&workdir).unwrap().next().is_none());

        write(&master, b"restore").expect("type exact acknowledgment");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "exact acknowledgment ready",
            |screen| {
                full_preview_content(screen) && screen.contains("Acknowledgment [restore] ready")
            },
        );
        assert_preview_has_no_execute_side_effects(
            &parser.screen().contents(),
            &destination,
            &parent,
            &workdir,
        );

        let compact = TerminalSize {
            ws_row: 18,
            ws_col: 96,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let before_compact = resize_pty(
            &master,
            &slave,
            &child,
            &mut output,
            &mut parser,
            compact.ws_row,
            compact.ws_col,
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "child redraw at 18x96",
            |screen| {
                frame_at_geometry(screen, compact.ws_row, compact.ws_col)
                    && screen.contains("Restore preview / outcome / Up-Down scroll")
                    && screen.contains("Snapshot target: 1700172800")
                    && screen.contains("Scope: whole snapshot (no marks)")
                    && screen.contains("Include exact source directory ancestors; no flattening.")
                    && screen.contains("Entries: 21")
                    && screen.contains("Regular payload bytes: 2621564")
                    && screen.contains("Acknowledgment [restore] ready")
            },
        );
        assert!(
            output.len() > before_compact,
            "shrink produced child output"
        );
        assert_eq!(parser.screen().size(), (compact.ws_row, compact.ws_col));
        assert_preview_has_no_execute_side_effects(
            &parser.screen().contents(),
            &destination,
            &parent,
            &workdir,
        );

        let mut saw_parent = false;
        let mut saw_leaf = false;
        let mut saw_no_replace = false;
        let mut saw_backend_validation = false;
        let mut saw_mode_metadata = false;
        for step in 0..12 {
            let previous_screen = parser.screen().contents();
            write(&master, b"\x1b[B").expect("scroll executable preview down");
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "compact preview scroll down",
                |screen| screen != previous_screen,
            );
            let screen = parser.screen().contents();
            assert_preview_has_no_execute_side_effects(&screen, &destination, &parent, &workdir);
            saw_parent |= screen.contains("Requested parent:") && screen.contains(&parent_text);
            saw_leaf |= screen.contains("New destination leaf: preview-resize-tree");
            saw_no_replace |= screen.contains("Atomic no-replacement publication");
            saw_backend_validation |= screen.contains("Backend checks parent policy/filesystem");
            saw_mode_metadata |= screen.contains("Preserves ordinary modes")
                && screen.contains("supported exact modification times")
                && screen.contains("raw symlink target bytes");
            if saw_parent
                && saw_leaf
                && saw_no_replace
                && saw_backend_validation
                && saw_mode_metadata
            {
                break;
            }
            assert!(
                step < 11,
                "scroll did not expose every required preview detail"
            );
        }
        assert!(saw_parent, "scrolled screen exposes the configured parent");
        assert!(saw_leaf, "scrolled screen exposes the new leaf");
        assert!(
            saw_no_replace,
            "scrolled screen exposes no-replacement policy"
        );
        assert!(
            saw_backend_validation,
            "scrolled screen exposes backend caveat"
        );
        assert!(saw_mode_metadata, "scrolled screen exposes metadata policy");

        // Return to the top through production preview scrolling before the
        // second resize; do not rely on the enlarged viewport clamping offset.
        let mut saw_compact_summary = false;
        for step in 0..12 {
            let screen = parser.screen().contents();
            if screen.contains("Snapshot target: 1700172800")
                && screen.contains("Scope: whole snapshot (no marks)")
                && screen.contains("Entries: 21")
                && screen.contains("Regular payload bytes: 2621564")
                && screen.contains("Acknowledgment [restore] ready")
            {
                saw_compact_summary = true;
                break;
            }
            let previous_screen = screen;
            write(&master, b"\x1b[A").expect("scroll executable preview up to summary");
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "compact preview scroll back to summary",
                |screen| screen != previous_screen,
            );
            assert_preview_has_no_execute_side_effects(
                &parser.screen().contents(),
                &destination,
                &parent,
                &workdir,
            );
            assert!(step < 11, "Up did not return to compact preview summary");
        }
        assert!(
            saw_compact_summary,
            "compact top summary is visible before expand"
        );

        let expanded = TerminalSize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let before_expanded = resize_pty(
            &master,
            &slave,
            &child,
            &mut output,
            &mut parser,
            expanded.ws_row,
            expanded.ws_col,
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "child redraw at 40x240",
            |screen| {
                frame_at_geometry(screen, expanded.ws_row, expanded.ws_col)
                    && full_preview_content(screen)
                    && screen.contains("Acknowledgment [restore] ready")
            },
        );
        assert!(
            output.len() > before_expanded,
            "expand produced child output"
        );
        assert_eq!(parser.screen().size(), (expanded.ws_row, expanded.ws_col));
        assert_preview_has_no_execute_side_effects(
            &parser.screen().contents(),
            &destination,
            &parent,
            &workdir,
        );

        write(&master, b"\x1b").expect("abandon executable preview with Esc");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "preview abandonment",
            |screen| {
                screen.contains("Restore preview abandoned; nothing staged.")
                    && !screen.contains("Restore preview / outcome / Up-Down scroll")
                    && !screen.contains("Acknowledgment")
                    && !screen.contains("Restoration running.")
                    && !screen.contains("Published completed tree.")
            },
        );
        assert!(!destination.exists());
        assert!(fs::read_dir(&parent).unwrap().next().is_none());
        assert!(fs::read_dir(&workdir).unwrap().next().is_none());

        write(&master, b"q").expect("quit from snapshot browser");
        let status = wait_for_exit(&master, &mut child, &mut output)
            .expect("production TUI did not exit after q within timeout");
        assert!(status.success(), "production TUI exited with {status}");
        assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
        assert!(contains(&output, b"\x1b[?1049h"));
        assert!(contains(&output, b"\x1b[?1049l"));
        assert!(fs::read_dir(&parent).unwrap().next().is_none());
        assert!(fs::read_dir(&workdir).unwrap().next().is_none());
        assert_eq!(fingerprint(&fixture), source_before);
    }

    #[cfg(target_os = "linux")]
    fn raw_os_string(bytes: Vec<u8>) -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(bytes)
    }

    #[cfg(target_os = "linux")]
    fn escaped_byte(byte: u8) -> String {
        format!("\\x{byte:02x}")
    }

    #[cfg(target_os = "linux")]
    fn contains_staging_entry(root: &Path) -> bool {
        let entries = fs::read_dir(root).expect("inspect test-owned restore tree");
        entries
            .map(|entry| entry.expect("inspect test-owned directory entry"))
            .any(|entry| {
                use std::os::unix::ffi::OsStrExt;
                let path = entry.path();
                entry.file_name().as_bytes().starts_with(b".duplicity-")
                    || (entry
                        .file_type()
                        .expect("inspect test-owned entry type")
                        .is_dir()
                        && contains_staging_entry(&path))
            })
    }

    #[cfg(target_os = "linux")]
    fn create_raw_file_parent(root: &Path) -> (PathBuf, Vec<u8>) {
        use std::os::unix::ffi::OsStrExt;
        let mut parent = root.to_path_buf();
        // Escaped components make the requested pathname taller than the
        // result viewport, proving that the complete identity is scrollable.
        for index in 0x80..0x8e {
            parent.push(raw_os_string(vec![index; 120]));
            fs::create_dir(&parent).expect("create raw-byte workdir component");
        }
        let parent_leaf = vec![b'p', 0x1b, b'\\', 0xff];
        parent.push(raw_os_string(parent_leaf.clone()));
        let sentinel = b"restore-parent sentinel; must remain unchanged".to_vec();
        fs::write(&parent, &sentinel).expect("create regular-file restore parent");
        assert!(fs::metadata(&parent).unwrap().is_file());
        assert_eq!(parent.file_name().unwrap().as_bytes(), parent_leaf);
        (parent, sentinel)
    }

    #[cfg(target_os = "linux")]
    fn metadata_identity(metadata: &fs::Metadata) -> (u64, u64, u32, u64, i64, i64) {
        use std::os::unix::fs::MetadataExt;
        (
            metadata.dev(),
            metadata.ino(),
            metadata.mode(),
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
        )
    }

    #[test]
    #[cfg(target_os = "linux")]
    //= docs/parsing-requirements.md#restore-ui-003
    //= type=test
    //# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
    //= docs/parsing-requirements.md#restore-ui-005
    //= type=test
    //# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
    //= docs/parsing-requirements.md#restore-ui-008
    //= type=test
    //# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
    //= docs/parsing-requirements.md#restore-ui-010
    //= type=test
    //# The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    fn invalid_raw_parent_produces_scrolled_unpublished_diagnostic_over_pty() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/duplicity-3.0.7/expected.json"
        ))
        .expect("parse official source oracle");
        let entries = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
            .unwrap()["entries"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 21);
        let payload_bytes: u64 = entries
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| entry["size"].as_u64().unwrap())
            .sum();
        assert_eq!(payload_bytes, 2_621_564);

        let fixture = fs::canonicalize(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/plain"),
        )
        .expect("official plain fixture");
        let source_before = fingerprint(&fixture);
        let mut scratch = Scratch::new();
        let workdir = scratch.0.clone();
        let (parent, sentinel) = create_raw_file_parent(&workdir);
        // This guard is declared before ChildGuard below, so failures reap the
        // process first and then retain the private tree for diagnosis.
        let _preserve_on_failure = PreserveOnFailure(&mut scratch);
        let parent_before = fs::metadata(&parent).expect("parent sentinel metadata");
        let parent_identity = metadata_identity(&parent_before);
        let leaf = vec![0x1b, b'\\', 0xff, b'R'];
        let destination = parent.join(raw_os_string(leaf.clone()));

        let original_size = TerminalSize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = openpty(
            Some(&Winsize {
                ws_row: original_size.ws_row,
                ws_col: original_size.ws_col,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .expect("open invalid-parent acceptance PTY");
        fcntl_setfd(&pty.master, FdFlags::CLOEXEC).expect("close PTY master on exec");
        let master = File::from(pty.master);
        fcntl_setfd(&pty.slave, FdFlags::CLOEXEC).expect("close parent slave on exec");
        let slave = File::from(pty.slave);
        let original_termios = tcgetattr(&slave).expect("read original terminal attributes");

        let mut command = Command::new("setsid");
        command
            .arg("--wait")
            .arg(env!("CARGO_BIN_EXE_duplicity-tui"))
            .arg("local")
            .arg(&fixture)
            .arg("--restore-parent")
            .arg(&parent)
            .arg("--restore-name")
            .arg(raw_os_string(leaf.clone()))
            .current_dir(&workdir)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "240")
            .env("LINES", "40")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        let mut child = ChildGuard(command.spawn().expect("launch TUI under setsid"));
        let mut output = Vec::new();
        let mut parser = vt100::Parser::new(original_size.ws_row, original_size.ws_col, 0);
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "history before invalid-parent restore",
            |screen| {
                screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                    && screen.contains("Metadata only. a selects full anchor")
            },
        );
        assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
        write(&master, b"a").expect("select full anchor");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "full anchor selection",
            |screen| screen.contains("Full anchor selected; choose target then Enter."),
        );
        write(&master, b"\x1b[B\x1b[B\r").expect("select final target and request scan");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "full scan disclosure",
            |screen| {
                screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
            },
        );
        write(&master, b"\r").expect("confirm full scan");
        let validated = format!(
            "Validated snapshot: {} entries, {payload_bytes} payload bytes.",
            entries.len()
        );
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "official final snapshot validated",
            |screen| screen.contains(&validated),
        );
        write(&master, b"r").expect("request executable whole-snapshot preview");
        let preview_ready = |screen: &str| {
            screen.contains("Restore preview / outcome / Up-Down scroll")
                && screen.contains("Snapshot target: 1700172800")
                && screen.contains("Scope: whole snapshot (no marks)")
                && screen.contains("Entries: 21")
                && screen.contains("Regular payload bytes: 2621564")
                && screen.contains("Include exact source directory ancestors; no flattening.")
                && screen.contains("Acknowledgment [] ready")
        };
        assert!(wait_for_preview_ready_with_initial_unarmed_ack(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            preview_ready,
        ));

        let mut preview_frames = parser.screen().contents();
        let escaped_leaf = r"\x1b\\\xffR";
        let mut preview_saw_leaf = false;
        for step in 0..80 {
            let screen = parser.screen().contents();
            preview_saw_leaf |= screen.contains(&format!("New destination leaf: {escaped_leaf}"));
            preview_frames.push_str(&screen);
            if preview_saw_leaf {
                break;
            }
            let previous = screen;
            write(&master, b"\x1b[B").expect("scroll preview to requested identity");
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "scroll preview identity",
                |screen| screen != previous,
            );
            assert!(step < 79, "preview did not expose configured leaf");
        }
        assert!(preview_saw_leaf, "preview shows exact escaped leaf");
        assert!(preview_frames.contains("Atomic no-replacement publication"));
        assert!(preview_frames.contains("Backend checks parent policy/filesystem"));
        assert!(preview_frames.contains("Preserves ordinary modes"));
        assert!(preview_frames.contains("supported exact modification times"));
        assert!(preview_frames.contains("raw symlink target bytes"));
        assert!(preview_frames
            .contains("Ownership, source ACLs/xattrs and crash durability are excluded."));
        assert!(preview_frames.contains(&format!("Requested parent: {}", workdir.display())));
        let mut previous_component = 0;
        for byte in 0x80..0x8e {
            let marker = escaped_byte(byte).repeat(20);
            let position = preview_frames
                .find(&marker)
                .unwrap_or_else(|| panic!("preview preserves raw component byte {byte:02x}"));
            assert!(
                position >= previous_component,
                "preview preserves requested parent component order"
            );
            previous_component = position;
        }
        assert!(preview_frames.contains(r"p\x1b\\\xff"));
        // Return to the preview start so confirmation applies to the complete
        // concrete request and its summary, then acknowledge exactly.
        for _ in 0..80 {
            let screen = parser.screen().contents();
            if preview_ready(&screen) {
                break;
            }
            let previous = screen;
            write(&master, b"\x1b[A").expect("scroll preview back to top");
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "return preview to summary",
                |screen| screen != previous,
            );
        }
        assert!(preview_ready(&parser.screen().contents()));
        write(&master, b"restore").expect("type exact acknowledgment");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "ready acknowledgment",
            |screen| preview_ready(screen) && screen.contains("Acknowledgment [restore] ready"),
        );
        write(&master, b"\r").expect("confirm exact restore acknowledgment");
        wait_for_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "typed ENOTDIR result",
            |screen| {
                screen.contains("Restore result. Up/Down scroll; Esc returns; q exits.")
                    && screen.contains("Unpublished: cause Syscall errno Some(20)")
                    && screen.contains("Cleanup attempted=0 failures=0 first_errno=None exhausted=false detail_truncated=false")
                    && !screen.contains("Publication indeterminate")
                    && !screen.contains("Published completed tree")
                    && !screen.contains("First-stage candidate uncertain")
                    && !screen.contains("Profile ")
                    && !screen.contains("Parent device=")
                    && !screen.contains("Destination: ")
                    && !screen.contains("Staging: ")
                    && !screen.contains("Empty-root mtime")
            },
        );
        let result_first_screen = parser.screen().contents();
        assert!(!result_first_screen.contains("Requested leaf:"));
        assert!(!result_first_screen.contains('\u{fffd}'));

        let mut result_frames = result_first_screen.clone();
        let mut result_has_identity = false;
        for step in 0..80 {
            let screen = parser.screen().contents();
            result_has_identity |= screen.contains(&format!("Requested leaf: {escaped_leaf}"));
            result_frames.push_str(&screen);
            if result_has_identity {
                break;
            }
            let previous = screen;
            write(&master, b"\x1b[B").expect("scroll typed result diagnostics");
            wait_for_screen(
                &master,
                &mut child,
                &mut output,
                &mut parser,
                "scroll result identity",
                |screen| screen != previous,
            );
            assert!(step < 79, "result document did not expose requested leaf");
        }
        assert!(
            result_has_identity,
            "result exposes exact escaped requested leaf"
        );
        assert!(result_frames.contains(r"p\x1b\\\xff"));
        assert!(result_frames.contains(&format!("Requested parent: {}", workdir.display())));
        assert!(!result_frames.contains('\u{fffd}'));
        let mut previous_component = 0;
        for byte in 0x80..0x8e {
            let marker = escaped_byte(byte).repeat(20);
            let position = result_frames
                .find(&marker)
                .unwrap_or_else(|| panic!("result preserves raw component byte {byte:02x}"));
            assert!(
                position >= previous_component,
                "result preserves requested parent component order"
            );
            previous_component = position;
        }

        assert_eq!(fs::read(&parent).unwrap(), sentinel);
        assert_eq!(
            metadata_identity(&fs::metadata(&parent).unwrap()),
            parent_identity
        );
        assert!(
            matches!(fs::symlink_metadata(&destination), Err(error) if error.kind() == std::io::ErrorKind::NotFound || error.kind() == std::io::ErrorKind::NotADirectory)
        );
        assert!(
            !contains_staging_entry(&workdir),
            "no .duplicity-* staging path was created"
        );
        assert_eq!(fingerprint(&fixture), source_before);

        write(&master, b"q").expect("quit from result document");
        let status = wait_for_exit(&master, &mut child, &mut output)
            .expect("TUI did not exit after q within timeout");
        assert!(status.success(), "TUI exited with {status}");
        assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
        assert!(contains(&output, b"\x1b[?1049h"));
        assert!(contains(&output, b"\x1b[?1049l"));
        fs::remove_dir_all(&workdir).expect("remove test-owned invalid-parent scratch tree");
        assert!(!workdir.exists(), "test-owned scratch tree was removed");
    }

    #[test]
    #[cfg(target_os = "linux")]
    //= docs/parsing-requirements.md#restore-ui-003
    //= type=test
    //# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
    //= docs/parsing-requirements.md#restore-ui-004
    //= type=test
    //# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    fn executable_preview_resizes_scrolls_and_abandons_without_execute() {
        production_linux_executable_preview_survives_pty_resize_without_execute();
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    fn official_chain_readonly_previews_through_pty_match_source_oracle() {
        check_official_chain_readonly_previews(false);
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    fn official_chain_pty_resize_preserves_snapshot_mark_and_cursor() {
        check_official_chain_readonly_previews(true);
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    fn local_fixture_history_quit_restores_pty_state() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain");
        let fixture = fs::canonicalize(fixture).unwrap();
        let before_source = fingerprint(&fixture);
        let scratch = Scratch::new();
        let winsize = Winsize {
            ws_row: 40,
            ws_col: 240,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = openpty(Some(&winsize), None).expect("open PTY");
        let slave = File::from(pty.slave);
        let original_termios = tcgetattr(&slave).expect("read original PTY attributes");
        let master = File::from(pty.master);

        let mut command = Command::new(env!("CARGO_BIN_EXE_duplicity-tui"));
        command
            .arg("local")
            .arg(&fixture)
            .current_dir(&scratch.0)
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "240")
            .env("LINES", "40")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        let mut child = ChildGuard(command.spawn().expect("spawn TUI in PTY"));
        let mut output = Vec::new();

        assert!(
            wait_for_history(&master, &mut child, &mut output),
            "history did not render before timeout or child exit; output: {}",
            String::from_utf8_lossy(&output)
        );
        let active_termios = tcgetattr(&slave).expect("read active PTY attributes");
        assert_ne!(
            active_termios, original_termios,
            "UI did not enter raw mode"
        );

        write(&master, b"q").expect("send normal quit key");
        let status = wait_for_exit(&master, &mut child, &mut output)
            .expect("TUI did not exit after q within timeout");
        assert!(status.success(), "TUI exited with {status}");
        assert_eq!(
            tcgetattr(&slave).expect("read restored PTY attributes"),
            original_termios,
            "terminal attributes were not restored"
        );
        assert!(
            contains(&output, b"\x1b[?1049h"),
            "alternate screen was not entered"
        );
        assert!(
            contains(&output, b"\x1b[?1049l"),
            "alternate screen was not left"
        );
        assert!(fs::read_dir(&scratch.0).unwrap().next().is_none());
        assert_eq!(
            fingerprint(&fixture),
            before_source,
            "source fixture changed"
        );
    }
}

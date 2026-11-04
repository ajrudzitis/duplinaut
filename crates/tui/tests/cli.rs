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
    use rustix::termios::{tcgetwinsize, tcsetwinsize, Winsize as TerminalSize};
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io,
        os::fd::{AsFd, AsRawFd},
        path::{Path, PathBuf},
        process::{Child, Command, ExitStatus, Stdio},
        time::{Duration, Instant},
    };

    pub(super) const HISTORY_MARKER: &[u8] = b"duplicity-full.20231114T221320Z.manifest";
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

    pub(super) struct Scratch(pub(super) PathBuf);

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
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create PTY test working directory: {error}"),
                }
            }
            panic!("could not allocate unique PTY test working directory")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
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
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            let previous = output.len();
            pump(master, output, 100);
            parser.process(&output[previous..]);
            let screen = parser.screen().contents();
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "TUI exited during {phase}; current screen:\n{screen}"
            );
            if expected(&screen) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out during {phase}; current screen:\n{screen}"
            );
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

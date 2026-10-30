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
        sys::termios::tcgetattr,
        unistd::{read, write},
    };
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io,
        os::fd::{AsFd, AsRawFd},
        path::{Path, PathBuf},
        process::{Child, Command, ExitStatus, Stdio},
        time::{Duration, Instant},
    };

    const HISTORY_MARKER: &[u8] = b"duplicity-full.20231114T221320Z.manifest";
    const EXIT_TIMEOUT: Duration = Duration::from_secs(15);
    const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
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

    fn fingerprint(root: &Path) -> Vec<(PathBuf, Option<[u8; 32]>)> {
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

    fn contains(output: &[u8], marker: &[u8]) -> bool {
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

    fn wait_for_exit(
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

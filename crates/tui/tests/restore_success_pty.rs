#![cfg(target_os = "linux")]

use nix::{
    errno::Errno,
    poll::{poll, PollFd, PollFlags},
    pty::{openpty, Winsize},
    sys::termios::tcgetattr,
    unistd::{read, write},
};
use rustix::io::{fcntl_setfd, FdFlags};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    os::{
        fd::{AsFd, AsRawFd},
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::{MetadataExt, PermissionsExt},
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const ROWS: u16 = 22;
const COLS: u16 = 240;
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
const PHASE_TIMEOUT: Duration = Duration::from_secs(45);
const RESTORE_TIMEOUT: Duration = Duration::from_secs(120);
const HISTORY_MARKER: &[u8] = b"duplicity-full.20231114T221320Z.manifest";

struct TestTree {
    path: PathBuf,
    preserve: bool,
}

impl TestTree {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        for _ in 0..100 {
            let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "duplinaut-success-pty-{}-{serial}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                    return Self {
                        path,
                        preserve: false,
                    };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create successful-restore test directory: {error}"),
            }
        }
        panic!("could not allocate a unique successful-restore test directory")
    }

    fn preserve(&mut self) {
        self.preserve = true;
    }

    fn cleanup_after_assertions(&mut self) {
        fs::remove_dir_all(&self.path).expect("remove verified successful-restore test tree");
        self.preserve = true;
    }
}

impl Drop for TestTree {
    fn drop(&mut self) {
        if self.preserve {
            if self.path.exists() {
                eprintln!(
                    "preserving successful-restore PTY artifacts for inspection: {}",
                    self.path.display()
                );
            }
        } else {
            let _ = fs::remove_dir_all(&self.path);
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    kind: EntryKind,
    mode: u32,
    mtime_ns: Option<i128>,
    size: Option<u64>,
    sha256: Option<String>,
    symlink_target: Option<Vec<u8>>,
}

fn observe_tree(
    path: &Path,
    components: &mut Vec<Vec<u8>>,
    observed: &mut BTreeMap<Vec<Vec<u8>>, Entry>,
) {
    let metadata =
        fs::symlink_metadata(path).expect("inspect restored entry without following links");
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        panic!(
            "restore produced an unsupported filesystem entry type: {}",
            path.display()
        );
    };
    let mtime_ns = (kind != EntryKind::Symlink)
        .then(|| i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec()));
    let mut entry = Entry {
        kind,
        mode: metadata.mode() & 0o777,
        mtime_ns,
        size: None,
        sha256: None,
        symlink_target: None,
    };
    match kind {
        EntryKind::Directory => {}
        EntryKind::File => {
            entry.size = Some(metadata.len());
            let mut file = File::open(path).expect("read restored regular file");
            let mut digest = Sha256::new();
            let mut buffer = [0u8; 16 * 1024];
            loop {
                let count = file.read(&mut buffer).expect("hash restored regular file");
                if count == 0 {
                    break;
                }
                digest.update(&buffer[..count]);
            }
            entry.sha256 = Some(format!("{:x}", digest.finalize()));
        }
        EntryKind::Symlink => {
            entry.symlink_target = Some(
                fs::read_link(path)
                    .expect("read raw restored symlink target")
                    .as_os_str()
                    .as_bytes()
                    .to_vec(),
            );
        }
    }
    assert!(observed.insert(components.clone(), entry).is_none());
    if kind == EntryKind::Directory {
        for child in fs::read_dir(path).expect("enumerate restored directory") {
            let child = child.expect("read restored directory entry");
            components.push(child.file_name().as_bytes().to_vec());
            observe_tree(&child.path(), components, observed);
            components.pop();
        }
    }
}

fn decode_hex(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty(), "oracle hex has complete byte pairs");
    pairs
        .iter()
        .map(|pair| {
            let digit = |value: u8| match value {
                b'0'..=b'9' => value - b'0',
                b'a'..=b'f' => value - b'a' + 10,
                b'A'..=b'F' => value - b'A' + 10,
                _ => panic!("invalid oracle hex digit"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

fn oracle_components(entry: &serde_json::Value) -> Vec<Vec<u8>> {
    entry["components_hex"]
        .as_array()
        .expect("oracle component array")
        .iter()
        .map(|component| decode_hex(component.as_str().expect("oracle component hex")))
        .collect()
}

fn expected_tree(entries: &[serde_json::Value]) -> BTreeMap<Vec<Vec<u8>>, &serde_json::Value> {
    entries
        .iter()
        .map(|entry| (oracle_components(entry), entry))
        .collect()
}

fn pump(master: &File, output: &mut Vec<u8>, timeout_ms: u16) {
    let mut descriptors = [PollFd::new(master.as_fd(), PollFlags::POLLIN)];
    poll(&mut descriptors, timeout_ms).expect("poll successful-restore PTY");
    if descriptors[0]
        .revents()
        .unwrap_or_else(PollFlags::empty)
        .contains(PollFlags::POLLIN)
    {
        let mut chunk = [0u8; 8192];
        match read(master.as_raw_fd(), &mut chunk) {
            Ok(count) => output.extend_from_slice(&chunk[..count]),
            Err(Errno::EIO | Errno::EAGAIN | Errno::EINTR) => {}
            Err(error) => panic!("read successful-restore PTY: {error}"),
        }
        assert!(output.len() <= OUTPUT_LIMIT, "PTY transcript exceeded cap");
    }
}

fn wait_screen(
    master: &File,
    child: &mut ChildGuard,
    output: &mut Vec<u8>,
    parser: &mut vt100::Parser,
    phase: &str,
    timeout: Duration,
    expected: impl Fn(&str) -> bool,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let previous = output.len();
        pump(master, output, 100);
        parser.process(&output[previous..]);
        let screen = parser.screen().contents();
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "production TUI exited during {phase}: {screen}"
        );
        if expected(&screen) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out during {phase}; current terminal screen:\n{screen}"
        );
    }
}

fn wait_preview_and_observe_initial_unarmed_ack(
    master: &File,
    child: &mut ChildGuard,
    output: &mut Vec<u8>,
    parser: &mut vt100::Parser,
    expected: impl Fn(&str) -> bool,
) -> bool {
    let deadline = Instant::now() + PHASE_TIMEOUT;
    let mut saw_empty_unarmed = false;
    loop {
        let previous = output.len();
        pump(master, output, 100);
        // Process bytes incrementally so a ready redraw in the same PTY read
        // cannot hide the earlier rendered empty/unarmed preview state.
        let mut saw_ready = false;
        for byte in &output[previous..] {
            parser.process(std::slice::from_ref(byte));
            let screen = parser.screen().contents();
            saw_empty_unarmed |= screen.contains("Acknowledgment [] unarmed");
            saw_ready |= expected(&screen);
        }
        if saw_ready {
            return saw_empty_unarmed;
        }
        let screen = parser.screen().contents();
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "production TUI exited before executable preview was ready; screen:\n{screen}"
        );
        assert!(
            Instant::now() < deadline,
            "timed out waiting for ready executable preview; initial empty/unarmed acknowledgment observed={saw_empty_unarmed}; screen:\n{screen}"
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

fn frame_at_pty_geometry(screen: &str) -> bool {
    let Some(top) = screen.lines().next() else {
        return false;
    };
    let Some(bottom) = screen.lines().nth(usize::from(ROWS - 1)) else {
        return false;
    };
    top.starts_with('┌')
        && top.chars().count() == usize::from(COLS)
        && top.ends_with('┐')
        && bottom.starts_with('└')
        && bottom.chars().count() == usize::from(COLS)
        && bottom.ends_with('┘')
}

fn assert_matches_expected_tree(root: &Path, expected: BTreeMap<Vec<Vec<u8>>, &serde_json::Value>) {
    let mut actual = BTreeMap::new();
    observe_tree(root, &mut Vec::new(), &mut actual);
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "published tree has exactly the oracle's raw path component vectors"
    );

    for (components, record) in expected {
        let found = actual
            .get(&components)
            .expect("oracle path exists in restored tree");
        let kind = match record["kind"].as_str().expect("oracle kind") {
            "directory" => EntryKind::Directory,
            "file" => EntryKind::File,
            "symlink" => EntryKind::Symlink,
            other => panic!("unsupported oracle entry kind {other}"),
        };
        assert_eq!(found.kind, kind, "entry kind at {components:?}");
        assert_eq!(
            found.mode,
            record["mode"].as_u64().expect("oracle mode") as u32 & 0o777,
            "ordinary mode at {components:?}"
        );
        match kind {
            EntryKind::Directory | EntryKind::File => assert_eq!(
                found.mtime_ns,
                Some(i128::from(
                    record["mtime_ns"].as_i64().expect("signed oracle mtime"),
                )),
                "exact supported mtime at {components:?}"
            ),
            EntryKind::Symlink => {
                // The official producer oracle excludes symlink mtime.
                assert!(found.mtime_ns.is_none());
                assert_eq!(
                    found.symlink_target.as_deref(),
                    Some(
                        decode_hex(
                            record["symlink_target_hex"]
                                .as_str()
                                .expect("symlink target hex"),
                        )
                        .as_slice()
                    ),
                    "raw symlink target at {components:?}"
                );
            }
        }
        if kind == EntryKind::File {
            assert_eq!(
                found.size,
                Some(record["size"].as_u64().expect("oracle file size")),
                "file size at {components:?}"
            );
            assert_eq!(
                found.sha256.as_deref(),
                Some(record["sha256"].as_str().expect("oracle file SHA-256")),
                "file SHA-256 at {components:?}"
            );
        }
    }
}

fn assert_matches_oracle(root: &Path, oracle_entries: &[serde_json::Value]) {
    assert_eq!(oracle_entries.len(), 21);
    let expected_payload: u64 = oracle_entries
        .iter()
        .filter(|entry| entry["kind"] == "file")
        .map(|entry| entry["size"].as_u64().expect("file size"))
        .sum();
    assert_eq!(expected_payload, 2_621_564);
    let expected = expected_tree(oracle_entries);
    assert_eq!(
        expected.len(),
        oracle_entries.len(),
        "oracle paths are unique"
    );
    assert_matches_expected_tree(root, expected);
}

fn assert_matches_oracle_subset(
    root: &Path,
    oracle_entries: &[serde_json::Value],
    expected_paths: BTreeSet<Vec<Vec<u8>>>,
    expected_payload: u64,
) {
    let all = expected_tree(oracle_entries);
    let selected: BTreeMap<_, _> = all
        .into_iter()
        .filter(|(components, _)| expected_paths.contains(components))
        .collect();
    assert_eq!(
        selected.keys().cloned().collect::<BTreeSet<_>>(),
        expected_paths,
        "all expected raw paths are represented in the official oracle"
    );
    assert_eq!(selected.len(), 6, "selected restore oracle has six entries");
    let payload = selected
        .values()
        .filter(|entry| entry["kind"] == "file")
        .map(|entry| entry["size"].as_u64().expect("oracle file size"))
        .sum::<u64>();
    assert_eq!(payload, expected_payload);
    assert_matches_expected_tree(root, selected);
}

#[test]
//= docs/parsing-requirements.md#restore-ui-003
//= type=test
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
//= docs/parsing-requirements.md#restore-ui-004
//= type=test
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-005
//= type=test
//# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
//= docs/parsing-requirements.md#restore-ui-008
//= type=test
//# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn production_linux_tui_publishes_official_plain_snapshot_through_pty() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .expect("parse official source oracle");
    let final_snapshot = oracle["snapshots"]
        .as_array()
        .expect("oracle snapshots")
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
        .expect("final plain snapshot");
    assert!(final_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions")
        .iter()
        .any(|exclusion| exclusion == "symlink_mtime_ns"));
    let oracle_entries = final_snapshot["entries"]
        .as_array()
        .expect("final snapshot entries")
        .clone();
    assert_eq!(oracle_entries.len(), 21);
    let expected_payload: u64 = oracle_entries
        .iter()
        .filter(|entry| entry["kind"] == "file")
        .map(|entry| entry["size"].as_u64().unwrap())
        .sum();
    assert_eq!(expected_payload, 2_621_564);

    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .expect("official plain fixture");
    let mut source_before = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_before);

    let mut test_tree = TestTree::new();
    let destination_parent = test_tree.path.join("restore-parent");
    fs::create_dir(&destination_parent).expect("create destination parent");
    fs::set_permissions(&destination_parent, fs::Permissions::from_mode(0o700))
        .expect("restrict destination parent permissions");
    let destination_name = "published-tree";
    let destination = destination_parent.join(destination_name);
    assert!(!destination.exists(), "destination leaf starts absent");

    let winsize = Winsize {
        ws_row: ROWS,
        ws_col: COLS,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = openpty(Some(&winsize), None).expect("open production TUI PTY");
    fcntl_setfd(&pty.master, FdFlags::CLOEXEC).expect("make PTY master close-on-exec");
    let slave = File::from(pty.slave);
    fcntl_setfd(&slave, FdFlags::CLOEXEC).expect("make parent PTY slave close-on-exec");
    let original_termios = tcgetattr(&slave).expect("read original PTY termios");
    let master = File::from(pty.master);
    let mut command = Command::new("setsid");
    command
        .arg("--wait")
        .arg(env!("CARGO_BIN_EXE_duplicity-tui"))
        .arg("local")
        .arg(&fixture)
        .arg("--restore-parent")
        .arg(&destination_parent)
        .arg("--restore-name")
        .arg(destination_name)
        .current_dir(&test_tree.path)
        .env("TERM", "xterm-256color")
        .env("COLUMNS", COLS.to_string())
        .env("LINES", ROWS.to_string())
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    // Crossterm probes /dev/tty before stdio for geometry. The Linux util-linux
    // setsid wrapper starts the production executable in a fresh session and
    // `--wait` forwards its exit status. The frame assertion below confirms
    // the child used this PTY's configured dimensions.
    let mut child = ChildGuard(
        command
            .spawn()
            .expect("launch production duplicity-tui under setsid --wait"),
    );
    let mut output = Vec::new();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "history discovery",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
                && frame_at_pty_geometry(screen)
        },
    );
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    write(&master, b"a").expect("select full anchor");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "full-anchor selection",
        PHASE_TIMEOUT,
        |screen| screen.contains("Full anchor selected; choose target then Enter."),
    );
    write(&master, b"\x1b[B\x1b[B\r").expect("select final target and request scan");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "full-volume scan confirmation",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
        },
    );
    write(&master, b"\r").expect("confirm complete target scan");
    let validated = format!(
        "Validated snapshot: {} entries, {expected_payload} payload bytes.",
        oracle_entries.len()
    );
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "final validated snapshot",
        PHASE_TIMEOUT,
        |screen| screen.contains(&validated),
    );
    write(&master, b"r").expect("request executable restore preview");
    let parent_text = destination_parent.display().to_string();
    let saw_empty_unarmed = wait_preview_and_observe_initial_unarmed_ack(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        |screen| {
            screen.contains("Snapshot target: 1700172800")
                && screen.contains("Scope: whole snapshot (no marks)")
                && screen.contains("Include exact source directory ancestors; no flattening.")
                && screen.contains("Entries: 21")
                && screen.contains("Regular payload bytes: 2621564")
                && screen.contains(&format!("Requested parent: {parent_text}"))
                && screen.contains("New destination leaf: published-tree")
                && screen.contains("Atomic no-replacement publication")
                && screen.contains("Backend checks parent policy/filesystem")
                && screen.contains("Preserves ordinary modes, supported exact modification times and raw symlink target bytes.")
                && screen.contains("Ownership, source ACLs/xattrs and crash durability are excluded.")
                && screen.contains("Type restore, then Enter.")
                && screen.contains("Acknowledgment [] ready")
        },
    );
    assert!(
        saw_empty_unarmed,
        "the preview first rendered with an empty, unarmed acknowledgment"
    );
    write(&master, b"\x1b[B").expect("scroll preview toward its end");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "preview end rows",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Legacy terminals can report repeated keys as Press")
                && !screen.contains("Snapshot target: 1700172800")
        },
    );
    write(&master, b"\x1b[A").expect("scroll preview back toward its beginning");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "preview beginning rows",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Snapshot target: 1700172800")
                && screen.contains("New destination leaf: published-tree")
                && !screen.contains("Legacy terminals can report repeated keys as Press")
        },
    );
    assert!(!destination.exists());

    // Empty confirmation is refused and does not dispatch a restore.
    write(&master, b"\r").expect("send empty acknowledgment Enter");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "empty acknowledgment refusal",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment must be exactly restore, then Enter."),
    );
    assert!(!screen_has(&parser, "Restoration running."));
    assert!(!screen_has(&parser, "Published completed tree."));
    assert!(!destination.exists());

    write(&master, b"restore").expect("type exact seven-byte acknowledgment");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "armed exact acknowledgment",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment [restore] ready"),
    );
    test_tree.preserve();
    write(&master, b"\r").expect("confirm one restore operation");
    // Fast local restores can finish before the PTY observes an intermediate
    // running frame, so accept either that frame or the completed result here.
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "restore operation started or already completed",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Restoration running.") || screen.contains("Published completed tree.")
        },
    );
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "published result document",
        RESTORE_TIMEOUT,
        |screen| {
            screen.contains("Restore result. Up/Down scroll; Esc returns; q exits.")
                && screen.contains("Published completed tree.")
                && screen.contains("Destination:")
                && screen.contains(destination_name)
        },
    );
    assert!(
        destination.is_dir(),
        "Published result has the requested leaf"
    );
    let siblings: Vec<_> = fs::read_dir(&destination_parent)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(siblings.len(), 1);
    assert_eq!(siblings[0].as_bytes(), destination_name.as_bytes());
    assert_matches_oracle(&destination, &oracle_entries);

    write(&master, b"q").expect("quit through production result-view key path");
    let status = wait_exit(&master, &mut child, &mut output)
        .expect("production TUI did not exit within its deadline");
    assert!(status.success(), "production TUI exited with {status}");
    assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(
        output
            .windows(b"\x1b[?1049h".len())
            .any(|window| window == b"\x1b[?1049h"),
        "TUI entered the alternate screen"
    );
    assert!(
        output
            .windows(b"\x1b[?1049l".len())
            .any(|window| window == b"\x1b[?1049l"),
        "TUI left the alternate screen"
    );
    let mut source_after = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_after);
    assert_eq!(
        source_after, source_before,
        "source fixture remained unchanged"
    );
    assert!(fs::read_dir(&destination_parent)
        .unwrap()
        .all(|entry| { entry.unwrap().file_name().as_bytes() == destination_name.as_bytes() }));
    test_tree.cleanup_after_assertions();
}

fn displayed_value(screen: &str, label: &str) -> Option<String> {
    let line = screen.lines().find(|line| line.contains(label))?;
    let value = line.split_once(label)?.1.trim();
    Some(value.strip_suffix('│').unwrap_or(value).trim().to_owned())
}

fn only_marked_raw_file_is_visible(screen: &str) -> bool {
    screen.lines().any(|line| {
        line.contains("* Regular") && line.trim_end_matches([' ', '│']).ends_with("raw-\\xff.bin")
    })
}

fn has_marked_file(screen: &str, path: &str) -> bool {
    screen
        .lines()
        .any(|line| line.contains("* Regular") && line.trim_end_matches([' ', '│']).ends_with(path))
}

fn has_unmarked_file(screen: &str, path: &str) -> bool {
    screen.lines().any(|line| {
        line.contains("Regular")
            && !line.contains("* Regular")
            && line.trim_end_matches([' ', '│']).ends_with(path)
    })
}

#[test]
//= docs/parsing-requirements.md#restore-ui-003
//= type=test
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
//= docs/parsing-requirements.md#restore-ui-004
//= type=test
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-005
//= type=test
//# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
//= docs/parsing-requirements.md#restore-ui-008
//= type=test
//# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn production_linux_tui_restores_two_marked_roots_with_exact_oracle_subset() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .expect("parse official source oracle");
    let final_snapshot = oracle["snapshots"]
        .as_array()
        .expect("oracle snapshots")
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
        .expect("final plain snapshot");
    let oracle_entries = final_snapshot["entries"]
        .as_array()
        .expect("final snapshot entries")
        .clone();
    let child_path = vec![b"a".to_vec(), b"b".to_vec(), b"child.txt".to_vec()];
    let sibling_path = vec![b"a!".to_vec(), b"sibling.txt".to_vec()];
    let all_components: Vec<_> = oracle_entries.iter().map(oracle_components).collect();
    let mut ordered_components = all_components.clone();
    ordered_components.sort();
    let child_index = ordered_components
        .iter()
        .position(|components| components == &child_path)
        .expect("official child path is in the snapshot");
    let sibling_index = ordered_components
        .iter()
        .position(|components| components == &sibling_path)
        .expect("official sibling path is in the snapshot");
    assert_eq!(child_index, 3, "fixture row order for a/b/child.txt");
    assert_eq!(sibling_index, 5, "fixture row order for a!/sibling.txt");

    let expected = expected_tree(&oracle_entries);
    let child_record = expected.get(&child_path).expect("oracle child file");
    let sibling_record = expected.get(&sibling_path).expect("oracle sibling file");
    assert_eq!(child_record["kind"], "file");
    assert_eq!(child_record["size"].as_u64(), Some(16));
    assert_eq!(
        child_record["sha256"].as_str(),
        Some("0d52c47f7783d2887fc36b0ab7657533cad3b748e12793a13837b19e9ff80965")
    );
    assert_eq!(sibling_record["kind"], "file");
    assert_eq!(sibling_record["size"].as_u64(), Some(12));
    assert_eq!(
        sibling_record["sha256"].as_str(),
        Some("ad319e8ad90014e64d573d16f3594fcf1fe52fe767cea9f4e5caf2b9b17fcd2b")
    );
    let expected_paths = BTreeSet::from([
        Vec::<Vec<u8>>::new(),
        vec![b"a".to_vec()],
        vec![b"a".to_vec(), b"b".to_vec()],
        child_path.clone(),
        vec![b"a!".to_vec()],
        sibling_path.clone(),
    ]);

    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .expect("official plain fixture");
    let mut source_before = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_before);

    let mut test_tree = TestTree::new();
    let destination_parent = test_tree.path.join("restore-parent");
    fs::create_dir(&destination_parent).expect("create destination parent");
    fs::set_permissions(&destination_parent, fs::Permissions::from_mode(0o700))
        .expect("restrict destination parent permissions");
    let destination_name = "selective-two-roots";
    let destination = destination_parent.join(destination_name);
    assert!(!destination.exists(), "selective destination starts absent");
    assert!(fs::read_dir(&destination_parent).unwrap().next().is_none());

    let pty = openpty(
        Some(&Winsize {
            ws_row: ROWS,
            ws_col: COLS,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("open selective restore production PTY");
    fcntl_setfd(&pty.master, FdFlags::CLOEXEC).expect("make PTY master close-on-exec");
    let slave = File::from(pty.slave);
    fcntl_setfd(&slave, FdFlags::CLOEXEC).expect("make parent PTY slave close-on-exec");
    let original_termios = tcgetattr(&slave).expect("read original PTY termios");
    let master = File::from(pty.master);
    let mut command = Command::new("setsid");
    command
        .arg("--wait")
        .arg(env!("CARGO_BIN_EXE_duplicity-tui"))
        .arg("local")
        .arg(&fixture)
        .arg("--restore-parent")
        .arg(&destination_parent)
        .arg("--restore-name")
        .arg(destination_name)
        .current_dir(&test_tree.path)
        .env("TERM", "xterm-256color")
        .env("COLUMNS", COLS.to_string())
        .env("LINES", ROWS.to_string())
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    let mut child = ChildGuard(
        command
            .spawn()
            .expect("launch production TUI under setsid --wait"),
    );
    let mut output = Vec::new();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "selective restore history",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
                && frame_at_pty_geometry(screen)
        },
    );
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    write(&master, b"a").expect("select full anchor");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "selective restore anchor",
        PHASE_TIMEOUT,
        |screen| screen.contains("Full anchor selected; choose target then Enter."),
    );
    write(&master, b"\x1b[B\x1b[B\r").expect("select final target and request scan");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "selective restore full scan disclosure",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
        },
    );
    write(&master, b"\r").expect("confirm required volume scan");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "selective restore validated snapshot",
        PHASE_TIMEOUT,
        |screen| screen.contains("Validated snapshot: 21 entries, 2621564 payload bytes."),
    );

    // Snapshot row order is checked against the oracle above. Mark the first
    // regular file, then the second, and observe exactly those two visible
    // stars through the live production screen.
    for _ in 0..child_index {
        write(&master, b"\x1b[B").expect("navigate to a/b/child.txt");
    }
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "child file selected",
        PHASE_TIMEOUT,
        |screen| screen.contains("a/b/child.txt") && !screen.contains("1 marked roots."),
    );
    write(&master, b" ").expect("mark a/b/child.txt");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "first marked root",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("1 marked roots.")
                && has_marked_file(screen, "a/b/child.txt")
                && has_unmarked_file(screen, "a!/sibling.txt")
        },
    );
    for _ in 0..(sibling_index - child_index) {
        write(&master, b"\x1b[B").expect("navigate to a!/sibling.txt");
    }
    write(&master, b" ").expect("mark a!/sibling.txt");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "exactly two marked roots",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("2 marked roots.")
                && has_marked_file(screen, "a/b/child.txt")
                && has_marked_file(screen, "a!/sibling.txt")
                && screen
                    .lines()
                    .filter(|line| line.contains("* Regular"))
                    .count()
                    == 2
        },
    );

    write(&master, b"r").expect("request selective executable preview");
    let parent_text = destination_parent.display().to_string();
    let expected_roots_visible = |screen: &str| {
        screen.contains("Snapshot target: 1700172800")
            && screen.contains("Scope: marked roots")
            && screen.contains("Root: a/b/child.txt")
            && screen.contains("Root: a!/sibling.txt")
            && screen.contains("Entries: 6")
            && screen.contains("Regular payload bytes: 28")
            && screen.contains("Include exact source directory ancestors; no flattening.")
            && screen.contains("Acknowledgment [] ready")
    };
    let saw_empty_unarmed = wait_preview_and_observe_initial_unarmed_ack(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        expected_roots_visible,
    );
    assert!(
        saw_empty_unarmed,
        "selective preview began empty and unarmed"
    );
    let mut saw_parent = false;
    let mut saw_leaf = false;
    let mut saw_no_replace = false;
    let mut saw_backend_check = false;
    let mut saw_mode_metadata = false;
    let mut saw_scope_summary = false;
    for step in 0..8 {
        let screen = parser.screen().contents();
        saw_parent |= screen.contains(&format!("Requested parent: {parent_text}"));
        saw_leaf |= screen.contains(&format!("New destination leaf: {destination_name}"));
        saw_no_replace |= screen.contains("Atomic no-replacement publication");
        saw_backend_check |= screen.contains("Backend checks parent policy/filesystem");
        saw_mode_metadata |= screen.contains(
            "Preserves ordinary modes, supported exact modification times and raw symlink target bytes.",
        );
        saw_scope_summary |= expected_roots_visible(&screen);
        if saw_parent
            && saw_leaf
            && saw_no_replace
            && saw_backend_check
            && saw_mode_metadata
            && saw_scope_summary
        {
            break;
        }
        let previous = screen;
        write(&master, b"\x1b[B").expect("scroll selective preview details");
        wait_screen(
            &master,
            &mut child,
            &mut output,
            &mut parser,
            "selective preview details",
            PHASE_TIMEOUT,
            |screen| screen != previous,
        );
        assert!(
            step < 7,
            "preview scroll reached its bound before all fields"
        );
    }
    assert!(saw_parent, "preview shows the requested parent");
    assert!(saw_leaf, "preview shows the requested leaf");
    assert!(saw_no_replace, "preview shows no-replacement behavior");
    assert!(saw_backend_check, "preview shows backend validation caveat");
    assert!(saw_mode_metadata, "preview shows metadata fidelity policy");
    assert!(
        saw_scope_summary,
        "preview showed both roots and six-entry summary"
    );
    let preview_screen = parser.screen().contents();
    assert!(preview_screen.contains("Restore preview / outcome / Up-Down scroll"));
    assert!(preview_screen.contains("Acknowledgment [] ready"));
    assert!(!destination.exists(), "preview has not executed restore");
    assert!(fs::read_dir(&destination_parent).unwrap().next().is_none());

    write(&master, b"\r").expect("submit empty acknowledgment");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "empty selective acknowledgment refusal",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment must be exactly restore, then Enter."),
    );
    assert!(!screen_has(&parser, "Restoration running."));
    assert!(!screen_has(&parser, "Published completed tree."));
    assert!(!destination.exists());
    assert!(fs::read_dir(&destination_parent).unwrap().next().is_none());

    write(&master, b"restore").expect("type exact ASCII acknowledgment");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "selective acknowledgment armed",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment [restore] ready"),
    );
    test_tree.preserve();
    write(&master, b"\r").expect("execute exactly one selective restore");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "Published selective restore result",
        RESTORE_TIMEOUT,
        |screen| {
            screen.contains("Published completed tree.")
                && screen.contains("Restore result. Up/Down scroll; Esc returns; q exits.")
                && displayed_value(screen, "Destination:").as_deref() == Some(destination_name)
        },
    );
    assert!(destination.is_dir());
    let siblings: Vec<Vec<u8>> = fs::read_dir(&destination_parent)
        .expect("enumerate selective restore parent")
        .map(|entry| {
            entry
                .expect("read selective restore parent entry")
                .file_name()
                .as_bytes()
                .to_vec()
        })
        .collect();
    assert_eq!(siblings, vec![destination_name.as_bytes().to_vec()]);
    assert_matches_oracle_subset(&destination, &oracle_entries, expected_paths, 28);

    write(&master, b"q").expect("quit through Published result screen");
    let status = wait_exit(&master, &mut child, &mut output)
        .expect("selective restore TUI did not exit within its deadline");
    assert!(
        status.success(),
        "selective restore TUI exited with {status}"
    );
    assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(output
        .windows(b"\x1b[?1049h".len())
        .any(|window| window == b"\x1b[?1049h"));
    assert!(output
        .windows(b"\x1b[?1049l".len())
        .any(|window| window == b"\x1b[?1049l"));
    let mut source_after = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_after);
    assert_eq!(
        source_after, source_before,
        "source fixture remains unchanged"
    );
    assert_eq!(
        fs::read_dir(&destination_parent)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
            .collect::<Vec<_>>(),
        vec![destination_name.as_bytes().to_vec()],
        "no sibling staging residue remains"
    );
    test_tree.cleanup_after_assertions();
}

// This acceptance covers one valid raw OS leaf, its reversible preview and
// Published receipt rendering, and the exact byte names observed on disk.
#[test]
//= docs/parsing-requirements.md#restore-ui-001
//= type=test
//# The frontend MUST accept a restoration destination only as a complete pair of `--restore-parent PATH` and `--restore-name NAME`, preserving their original OS bytes and rejecting duplicate, missing, oversized or invalid destination values before starting restoration.
//= docs/parsing-requirements.md#restore-ui-010
//= type=test
//# The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn production_linux_tui_preserves_raw_destination_and_selected_path_identity() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .expect("parse official source oracle");
    let final_snapshot = oracle["snapshots"]
        .as_array()
        .expect("oracle snapshots")
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 2)
        .expect("final plain snapshot");
    let oracle_entries = final_snapshot["entries"]
        .as_array()
        .expect("final snapshot entries");
    let raw_component = b"raw-\xff.bin".to_vec();
    let raw_file = oracle_entries
        .iter()
        .find(|entry| {
            entry["path_hex"] == "7261772dff2e62696e"
                && oracle_components(entry) == vec![raw_component.clone()]
        })
        .expect("official final snapshot has the raw-byte regular file");
    assert_eq!(raw_file["kind"], "file");
    assert_eq!(raw_file["size"].as_u64(), Some(9));
    assert_eq!(
        raw_file["sha256"].as_str(),
        Some("83e33998a9bc776bbe6bf39220f517734c5dc8b14168817e65b2a3b6edfee5f8")
    );
    let mut ordered_components: Vec<_> = oracle_entries.iter().map(oracle_components).collect();
    ordered_components.sort();
    let raw_file_index = ordered_components
        .iter()
        .position(|components| components == &vec![raw_component.clone()])
        .expect("raw file has an entry in the snapshot order");

    let destination_name = vec![0x1b, b'\\', 0xff, b'R'];
    assert!(!destination_name.is_empty());
    assert!(!destination_name.contains(&0));
    assert!(!destination_name.contains(&b'/'));
    assert_eq!(
        duplicity_tui::display::bytes(&destination_name),
        r"\x1b\\\xffR"
    );
    let source_display = duplicity_tui::display::bytes(&raw_component);
    assert_eq!(source_display, r"raw-\xff.bin");

    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .expect("official plain fixture");
    let mut source_before = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_before);

    let mut test_tree = TestTree::new();
    let destination_parent = test_tree.path.join("restore-parent");
    fs::create_dir(&destination_parent).expect("create destination parent");
    fs::set_permissions(&destination_parent, fs::Permissions::from_mode(0o700))
        .expect("restrict destination parent permissions");
    let destination =
        destination_parent.join(std::ffi::OsString::from_vec(destination_name.clone()));
    assert!(!destination.exists(), "raw destination leaf starts absent");

    let winsize = Winsize {
        ws_row: ROWS,
        ws_col: COLS,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = openpty(Some(&winsize), None).expect("open raw-identity production PTY");
    fcntl_setfd(&pty.master, FdFlags::CLOEXEC).expect("make PTY master close-on-exec");
    let slave = File::from(pty.slave);
    fcntl_setfd(&slave, FdFlags::CLOEXEC).expect("make parent PTY slave close-on-exec");
    let original_termios = tcgetattr(&slave).expect("read original PTY attributes");
    let master = File::from(pty.master);
    let mut command = Command::new("setsid");
    command
        .arg("--wait")
        .arg(env!("CARGO_BIN_EXE_duplicity-tui"))
        .arg("local")
        .arg(&fixture)
        .arg("--restore-parent")
        .arg(&destination_parent)
        .arg("--restore-name")
        .arg(std::ffi::OsString::from_vec(destination_name.clone()))
        .current_dir(&test_tree.path)
        .env("TERM", "xterm-256color")
        .env("COLUMNS", COLS.to_string())
        .env("LINES", ROWS.to_string())
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    let mut child = ChildGuard(
        command
            .spawn()
            .expect("launch production TUI with raw destination bytes"),
    );
    let mut output = Vec::new();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity history discovery",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains(std::str::from_utf8(HISTORY_MARKER).unwrap())
                && screen.contains("Metadata only. a selects full anchor")
                && frame_at_pty_geometry(screen)
        },
    );
    assert_ne!(tcgetattr(&slave).unwrap(), original_termios);
    write(&master, b"a").expect("select full anchor");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity full-anchor selection",
        PHASE_TIMEOUT,
        |screen| screen.contains("Full anchor selected; choose target then Enter."),
    );
    write(&master, b"\x1b[B\x1b[B\r").expect("select final target and request scan");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity scan confirmation",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Full volume scan required.") && screen.contains("Enter confirms.")
        },
    );
    write(&master, b"\r").expect("confirm final scan");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity validated snapshot",
        PHASE_TIMEOUT,
        |screen| screen.contains("Validated snapshot: 21 entries, 2621564 payload bytes."),
    );
    for _ in 0..raw_file_index {
        write(&master, b"\x1b[B").expect("move snapshot selection to raw-byte file");
    }
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw source file visible in snapshot list",
        PHASE_TIMEOUT,
        |screen| screen.contains(&source_display),
    );
    write(&master, b" ").expect("mark the selected raw-byte file");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "only raw source file marked",
        PHASE_TIMEOUT,
        |screen| screen.contains("1 marked roots.") && only_marked_raw_file_is_visible(screen),
    );
    write(&master, b"r").expect("request marked-file restore preview");
    let expected_root = format!("Root: {source_display}");
    let expected_destination = duplicity_tui::display::bytes(&destination_name);
    let parent_text = destination_parent.display().to_string();
    let saw_unarmed = wait_preview_and_observe_initial_unarmed_ack(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        |screen| {
            screen.contains("Snapshot target: 1700172800")
                && screen.contains("Scope: marked roots")
                && screen.contains(&expected_root)
                && screen.contains("Entries: 2")
                && screen.contains("Regular payload bytes: 9")
                && screen.contains(&format!("Requested parent: {parent_text}"))
                && displayed_value(screen, "New destination leaf:").as_deref()
                    == Some(expected_destination.as_str())
                && screen.contains("Atomic no-replacement publication")
                && screen.contains("Preserves ordinary modes, supported exact modification times and raw symlink target bytes.")
                && screen.contains("Acknowledgment [] ready")
        },
    );
    assert!(
        saw_unarmed,
        "raw destination preview first appears empty/unarmed"
    );
    let preview_screen = parser.screen().contents();
    assert_eq!(
        displayed_value(&preview_screen, "Root:").as_deref(),
        Some(source_display.as_str())
    );
    assert_eq!(
        displayed_value(&preview_screen, "New destination leaf:").as_deref(),
        Some(expected_destination.as_str())
    );
    assert!(expected_destination.bytes().all(|byte| byte.is_ascii()));
    assert!(source_display.bytes().all(|byte| byte.is_ascii()));
    assert!(!destination.exists());

    // Empty acknowledgment cannot dispatch and must leave the raw leaf absent.
    write(&master, b"\r").expect("send empty acknowledgment Enter");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity empty acknowledgment refusal",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment must be exactly restore, then Enter."),
    );
    assert!(!screen_has(&parser, "Restoration running."));
    assert!(!screen_has(&parser, "Published completed tree."));
    assert!(!destination.exists());

    write(&master, b"restore").expect("type exact ASCII acknowledgment");
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity armed acknowledgment",
        PHASE_TIMEOUT,
        |screen| screen.contains("Acknowledgment [restore] ready"),
    );
    test_tree.preserve();
    write(&master, b"\r").expect("confirm raw-byte restore exactly once");
    // A fast restore can publish before the PTY exposes an intermediate frame.
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "raw-identity restore started or already completed",
        PHASE_TIMEOUT,
        |screen| {
            screen.contains("Restoration running.") || screen.contains("Published completed tree.")
        },
    );
    wait_screen(
        &master,
        &mut child,
        &mut output,
        &mut parser,
        "Published raw destination receipt",
        RESTORE_TIMEOUT,
        |screen| {
            screen.contains("Published completed tree.")
                && screen.contains("Restore result. Up/Down scroll; Esc returns; q exits.")
                && displayed_value(screen, "Destination:").as_deref()
                    == Some(expected_destination.as_str())
        },
    );
    let receipt_screen = parser.screen().contents();
    assert_eq!(
        displayed_value(&receipt_screen, "Destination:").as_deref(),
        Some(expected_destination.as_str())
    );
    assert!(expected_destination.bytes().all(|byte| byte.is_ascii()));
    let actual_names: Vec<Vec<u8>> = fs::read_dir(&destination_parent)
        .expect("enumerate test-owned destination parent")
        .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
        .collect();
    assert_eq!(actual_names, vec![destination_name.clone()]);
    let root_metadata = fs::symlink_metadata(&destination).expect("inspect published raw leaf");
    assert!(root_metadata.file_type().is_dir());
    let raw_file_path = destination.join(std::ffi::OsString::from_vec(raw_component.clone()));
    let raw_file_metadata =
        fs::symlink_metadata(&raw_file_path).expect("inspect selected raw-byte file");
    assert!(raw_file_metadata.file_type().is_file());
    assert_eq!(raw_file_metadata.len(), 9);
    let mut actual_tree = BTreeMap::new();
    observe_tree(&destination, &mut Vec::new(), &mut actual_tree);
    assert_eq!(actual_tree.len(), 2);
    assert!(actual_tree.contains_key(&Vec::<Vec<u8>>::new()));
    let restored_file = actual_tree
        .get(&vec![raw_component.clone()])
        .expect("published tree contains exact raw path component");
    assert_eq!(restored_file.kind, EntryKind::File);
    assert_eq!(restored_file.size, Some(9));
    assert_eq!(
        restored_file.sha256.as_deref(),
        Some(raw_file["sha256"].as_str().unwrap())
    );

    write(&master, b"q").expect("quit through production Published result path");
    let status = wait_exit(&master, &mut child, &mut output)
        .expect("raw-identity TUI did not exit within its deadline");
    assert!(status.success(), "raw-identity TUI exited with {status}");
    assert_eq!(tcgetattr(&slave).unwrap(), original_termios);
    assert!(output
        .windows(b"\x1b[?1049l".len())
        .any(|window| window == b"\x1b[?1049l"));
    let mut source_after = BTreeMap::new();
    observe_tree(&fixture, &mut Vec::new(), &mut source_after);
    assert_eq!(
        source_after, source_before,
        "source fixture remained unchanged"
    );
    let final_names: Vec<Vec<u8>> = fs::read_dir(&destination_parent)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
        .collect();
    assert_eq!(final_names, vec![destination_name]);
    test_tree.cleanup_after_assertions();
}

fn screen_has(parser: &vt100::Parser, text: &str) -> bool {
    parser.screen().contents().contains(text)
}

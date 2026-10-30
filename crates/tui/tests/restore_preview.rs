#![cfg(unix)]

use duplicity_core::{local::LocalSource, names::Timestamp, source::Cancellation};
use duplicity_tui::{
    cli::Destination,
    worker::{Command, Reply, Worker},
};
use std::{
    fs,
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "duplicity-preview-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
static TEMP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn receive(worker: &mut Worker) -> Reply {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match worker.poll() {
            Ok(reply) => return reply,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                assert!(Instant::now() < deadline, "worker response timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("worker disconnected: {error:?}"),
        }
    }
}

fn worker() -> Worker {
    let source = LocalSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    Worker::start(
        Arc::new(source),
        "duplicity-".into(),
        Cancellation::default(),
    )
    .unwrap()
}

fn anchor(worker: &mut Worker) -> usize {
    match receive(worker) {
        Reply::Ready(rows, _) => {
            rows.into_iter()
                .find(|row| row.end == Timestamp(1700000000))
                .unwrap()
                .index
        }
        _ => panic!("discovery"),
    }
}

fn snapshot(
    worker: &mut Worker,
    id: u64,
    anchor: usize,
    target: i128,
) -> Arc<duplicity_core::reader::ValidatedSnapshot> {
    worker
        .send(Command::Scan {
            id,
            anchor,
            target: Timestamp(target),
            cancel: Cancellation::default(),
        })
        .unwrap();
    match receive(worker) {
        Reply::Snapshot { value, .. } => value,
        _ => panic!("scan"),
    }
}

fn assert_preview_policy(text: &str) {
    assert!(text.contains("Include exact source directory ancestors; no flattening."));
    assert!(text.contains(
        "Atomic no-replacement publication; existing destinations are never overwritten."
    ));
    assert!(text.contains(
        "Preserves ordinary modes, supported exact modification times and raw symlink target bytes."
    ));
    assert!(text.contains("Ownership, source ACLs/xattrs and crash durability are excluded."));
    assert!(text.contains(
        "Backend checks parent policy/filesystem, automatic profile and exact metadata after acknowledgment, before staging."
    ));
}

#[test]
//= docs/parsing-requirements.md#restore-ui-003
//= type=test
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
fn official_whole_snapshot_previews_report_oracle_counts_without_publishing() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = anchor(&mut worker);
    let mut id = 0;

    for day in 0..3 {
        let target = 1700000000 + day * 86400;
        id += 1;
        let snapshot = snapshot(&mut worker, id, anchor, target);
        let expected = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["mode"] == "plain" && row["index"] == day as u64)
            .unwrap()["entries"]
            .as_array()
            .unwrap();
        let expected_payload = expected
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| entry["size"].as_u64().unwrap())
            .sum::<u64>();
        let leaf = format!("whole-preview-{day}").into_bytes();
        id += 1;
        worker
            .send(Command::Prepare {
                id,
                snapshot,
                marked: Vec::new(),
                destination: Arc::new(Destination {
                    parent: temp.0.clone(),
                    name: leaf.clone(),
                }),
                cancel: Cancellation::default(),
            })
            .unwrap();
        let preview = match receive(&mut worker) {
            Reply::Preview {
                result: Ok(preview),
                ..
            } => preview,
            _ => panic!("whole-snapshot preview"),
        };

        assert!(preview
            .text
            .contains(&format!("Snapshot target: {target}\n")));
        assert!(preview.text.contains("Scope: whole snapshot (no marks)\n"));
        assert!(preview
            .text
            .contains(&format!("Entries: {}\n", expected.len())));
        assert!(preview
            .text
            .contains(&format!("Regular payload bytes: {expected_payload}\n")));
        assert!(preview
            .text
            .contains(&format!("Requested parent: {}\n", temp.0.display())));
        assert!(preview
            .text
            .contains(&format!("New destination leaf: whole-preview-{day}\n")));
        assert_preview_policy(&preview.text);
        assert!(!temp.0.join(std::ffi::OsString::from_vec(leaf)).exists());
    }
    worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
}

#[test]
//= docs/parsing-requirements.md#restore-ui-002
//= type=test
//# The frontend MUST derive a restoration preview from the exact completed immutable snapshot and selected original byte-valued roots, including precisely the entries and source directory ancestors accepted by its `RestorePlan`.
//= docs/parsing-requirements.md#restore-ui-003
//= type=test
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
fn marked_official_preview_preserves_raw_roots_and_required_ancestors_without_executing() {
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = anchor(&mut worker);
    let snapshot = snapshot(&mut worker, 1, anchor, 1700172800);

    let raw_root = vec![b"raw-\xff.bin".to_vec()];
    let child_root = vec![b"a".to_vec(), b"b".to_vec(), b"child.txt".to_vec()];
    let marked = snapshot
        .state()
        .entries()
        .enumerate()
        .filter_map(|(index, (path, _))| {
            let components = path.components();
            (components == &raw_root || components == &child_root).then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(marked.len(), 2, "both official roots must be present");
    let roots = marked
        .iter()
        .map(|index| snapshot.state().entries().nth(*index).unwrap().0.clone())
        .collect::<Vec<_>>();
    let plan = snapshot
        .restore_roots(
            &roots,
            duplicity_core::restore_plan::RestorePlanLimits::default(),
        )
        .unwrap();
    let expected_entries = plan.entries().len();
    let expected_payload = plan
        .entries()
        .iter()
        .map(|entry| {
            entry
                .entry()
                .content()
                .map_or(0, |content| content.len() as u64)
        })
        .sum::<u64>();
    assert!(
        expected_entries > roots.len(),
        "directory ancestors are included"
    );
    assert!(plan
        .entries()
        .iter()
        .any(|entry| entry.path().components() == &[b"a".to_vec()]));
    assert!(plan
        .entries()
        .iter()
        .any(|entry| entry.path().components() == &[b"a".to_vec(), b"b".to_vec()]));

    let leaf = b"marked-preview".to_vec();
    worker
        .send(Command::Prepare {
            id: 2,
            snapshot,
            marked,
            destination: Arc::new(Destination {
                parent: temp.0.clone(),
                name: leaf.clone(),
            }),
            cancel: Cancellation::default(),
        })
        .unwrap();
    let preview = match receive(&mut worker) {
        Reply::Preview {
            result: Ok(preview),
            ..
        } => preview,
        _ => panic!("marked preview"),
    };
    assert!(preview.text.contains("Snapshot target: 1700172800\n"));
    assert!(preview.text.contains("Scope: marked roots\n"));
    assert!(preview.text.contains(r"Root: raw-\xff.bin"));
    assert!(preview.text.contains("Root: a/b/child.txt"));
    assert!(preview
        .text
        .contains(&format!("Entries: {expected_entries}\n")));
    assert!(preview
        .text
        .contains(&format!("Regular payload bytes: {expected_payload}\n")));
    assert!(preview
        .text
        .contains(&format!("Requested parent: {}\n", temp.0.display())));
    assert!(preview
        .text
        .contains("New destination leaf: marked-preview\n"));
    assert_preview_policy(&preview.text);
    assert!(!temp.0.join(std::ffi::OsString::from_vec(leaf)).exists());
    worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
}

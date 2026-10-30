#![cfg(target_os = "linux")]
use duplicity_core::{
    local::LocalSource, names::Timestamp, restore_linux::RestoreOutcome, source::Cancellation,
};
use duplicity_tui::{
    cli::Destination,
    restore_flow,
    worker::{Command, Reply, Worker},
};
use std::{
    fs,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, PermissionsExt},
    },
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "duplicity-frontend-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn receive(worker: &mut Worker) -> Reply {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match worker.poll() {
            Ok(r) => return r,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("worker disconnected {e:?}"),
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
#[test]
//= docs/parsing-requirements.md#restore-ui-002
//= type=test
//# The frontend MUST derive a restoration preview from the exact completed immutable snapshot and selected original byte-valued roots, including precisely the entries and source directory ancestors accepted by its `RestorePlan`.
//= docs/parsing-requirements.md#restore-ui-003
//= type=test
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
//= docs/parsing-requirements.md#restore-ui-005
//= type=test
//# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
//= docs/parsing-requirements.md#restore-ui-011
//= type=test
//# The frontend MUST bound owned restoration request, preview and receipt metadata before allocating it, retaining original payload ownership without payload copies.
//= docs/parsing-requirements.md#restore-ui-013
//= type=test
//# The frontend MUST keep backup source access read-only and avoid collecting or exposing credentials, backup payload bytes or secret material through the restoration interaction, diagnostics or automatic logs.
//= docs/parsing-requirements.md#restore-ui-014
//= type=test
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
fn official_three_dates_pass_through_frozen_frontend_worker_and_source_oracles() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = match receive(&mut worker) {
        Reply::Ready(rows, _) => {
            rows.into_iter()
                .find(|r| r.end == Timestamp(1700000000))
                .unwrap()
                .index
        }
        _ => panic!("discovery"),
    };
    let mut id = 0;
    for day in 0..3 {
        id += 1;
        worker
            .send(Command::Scan {
                id,
                anchor,
                target: Timestamp(1700000000 + day * 86400),
                cancel: Cancellation::default(),
            })
            .unwrap();
        let snapshot = match receive(&mut worker) {
            Reply::Snapshot { value, .. } => value,
            _ => panic!("scan"),
        };
        let expected = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["mode"] == "plain" && r["index"] == day as u64)
            .unwrap()["entries"]
            .as_array()
            .unwrap();
        let expected_payload = expected
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| entry["size"].as_u64().unwrap())
            .sum::<u64>();
        let name = format!("restored{day}").into_bytes();
        let destination = Arc::new(Destination {
            parent: temp.0.clone(),
            name: name.clone(),
        });
        id += 1;
        worker
            .send(Command::Prepare {
                id,
                snapshot: snapshot.clone(),
                marked: Vec::new(),
                destination,
                cancel: Cancellation::default(),
            })
            .unwrap();
        let preview = match receive(&mut worker) {
            Reply::Preview {
                result: Ok(value), ..
            } => value,
            _ => panic!("prepare"),
        };
        assert!(!temp
            .0
            .join(std::ffi::OsString::from_vec(name.clone()))
            .exists());
        assert!(preview
            .text
            .contains(&format!("Snapshot target: {}\n", 1700000000 + day * 86400)));
        assert!(preview.text.contains("Scope: whole snapshot (no marks)\n"));
        assert!(preview
            .text
            .contains("Include exact source directory ancestors; no flattening."));
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
            .contains(&format!("New destination leaf: restored{day}\n")));
        assert!(preview.text.contains(
            "Atomic no-replacement publication; existing destinations are never overwritten."
        ));
        assert!(preview.text.contains(
            "Preserves ordinary modes, supported exact modification times and raw symlink target bytes."
        ));
        assert!(preview
            .text
            .contains("Ownership, source ACLs/xattrs and crash durability are excluded."));
        assert!(preview.text.contains(
            "Backend checks parent policy/filesystem, automatic profile and exact metadata after acknowledgment, before staging."
        ));
        id += 1;
        worker
            .send(Command::Execute {
                id,
                frozen: preview.frozen,
                cancel: Cancellation::default(),
            })
            .unwrap();
        let result = match receive(&mut worker) {
            Reply::Outcome {
                result: Ok(value), ..
            } => value,
            _ => panic!("execute"),
        };
        assert!(matches!(result, RestoreOutcome::Published(_)));
        assert!(!restore_flow::retry_allowed(&result));
        assert_eq!(expected.len(), snapshot.state().len());
        for (path, entry) in snapshot.state().entries() {
            let mut actual = temp.0.join(std::ffi::OsString::from_vec(name.clone()));
            for part in path.components() {
                actual.push(std::ffi::OsString::from_vec(part.clone()));
            }
            let bytes = path
                .components()
                .iter()
                .map(|p| p.as_slice())
                .collect::<Vec<_>>()
                .join(&b'/');
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            let source = expected.iter().find(|r| r["path_hex"] == hex).unwrap();
            let meta = fs::symlink_metadata(&actual).unwrap();
            assert_eq!(meta.mode() & 0o777, source["mode"].as_u64().unwrap() as u32);
            assert_eq!(meta.mtime(), entry.mtime().seconds);
            assert_eq!(meta.mtime_nsec(), i64::from(entry.mtime().nanoseconds));
            if source["kind"] != "symlink" {
                assert_eq!(
                    i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec()),
                    i128::from(source["mtime_ns"].as_i64().unwrap())
                );
            }
            match source["kind"].as_str().unwrap() {
                "file" => {
                    use sha2::Digest;
                    assert!(meta.is_file());
                    assert_eq!(
                        format!("{:x}", sha2::Sha256::digest(fs::read(actual).unwrap())),
                        source["sha256"].as_str().unwrap()
                    );
                }
                "directory" => assert!(meta.is_dir()),
                "symlink" => {
                    assert!(meta.file_type().is_symlink());
                    assert_eq!(
                        fs::read_link(actual).unwrap().as_os_str().as_bytes(),
                        entry.linkname()
                    );
                }
                other => panic!("{other}"),
            }
        }
    }
    worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
}

#[test]
//= docs/parsing-requirements.md#restore-ui-006
//= type=test
//# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
fn preview_cancellation_releases_request_and_scan_serializes_work() {
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = match receive(&mut worker) {
        Reply::Ready(rows, _) => {
            rows.into_iter()
                .find(|r| r.end == Timestamp(1700000000))
                .unwrap()
                .index
        }
        _ => panic!("ready"),
    };
    worker
        .send(Command::Scan {
            id: 1,
            anchor,
            target: Timestamp(1700000000),
            cancel: Cancellation::default(),
        })
        .unwrap();
    assert!(worker
        .send(Command::Scan {
            id: 2,
            anchor,
            target: Timestamp(1700000000),
            cancel: Cancellation::default()
        })
        .is_err());
    let snapshot = match receive(&mut worker) {
        Reply::Snapshot { value, .. } => value,
        _ => panic!("scan"),
    };
    let cancel = Cancellation::default();
    cancel.cancel();
    worker
        .send(Command::Prepare {
            id: 2,
            snapshot,
            marked: vec![],
            destination: Arc::new(Destination {
                parent: temp.0.clone(),
                name: b"dest".to_vec(),
            }),
            cancel,
        })
        .unwrap();
    assert!(matches!(
        receive(&mut worker),
        Reply::Preview {
            result: Err(restore_flow::Error::Cancelled),
            ..
        }
    ));
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
}
#[test]
//= docs/parsing-requirements.md#restore-ui-012
//= type=test
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
fn controlled_terminal_error_during_execute_drains_joins_and_retains_typed_result() {
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = match receive(&mut worker) {
        Reply::Ready(rows, _) => {
            rows.into_iter()
                .find(|r| r.end == Timestamp(1700000000))
                .unwrap()
                .index
        }
        _ => panic!("ready"),
    };
    worker
        .send(Command::Scan {
            id: 1,
            anchor,
            target: Timestamp(1700000000),
            cancel: Cancellation::default(),
        })
        .unwrap();
    let snapshot = match receive(&mut worker) {
        Reply::Snapshot { value, .. } => value,
        _ => panic!("scan"),
    };
    let destination = Arc::new(Destination {
        parent: temp.0.clone(),
        name: b"dest".to_vec(),
    });
    worker
        .send(Command::Prepare {
            id: 2,
            snapshot,
            marked: Vec::new(),
            destination: destination.clone(),
            cancel: Cancellation::default(),
        })
        .unwrap();
    let frozen = match receive(&mut worker) {
        Reply::Preview {
            result: Ok(value), ..
        } => value.frozen,
        _ => panic!("prepare"),
    };
    let cancel = Cancellation::default();
    worker
        .send(Command::Execute {
            id: 3,
            frozen,
            cancel: cancel.clone(),
        })
        .unwrap();
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected sink failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = duplicity_tui::worker::finish(
        worker,
        &cancel,
        duplicity_tui::terminal::Completion::event(Err(std::io::Error::other(
            "injected terminal I/O",
        ))),
        None,
        Some(destination),
        Some(3),
        &mut Broken,
    )
    .unwrap_err();
    let failure = error
        .get_ref()
        .unwrap()
        .downcast_ref::<restore_flow::ShutdownFailure>()
        .unwrap();
    assert!(failure.terminal.is_some());
    assert!(failure.shutdown.is_none());
    assert!(failure.output.is_some());
    assert!(!failure.unavailable);
    let outcome = failure
        .outcome
        .as_ref()
        .expect("typed result survives output failure");
    match outcome {
        RestoreOutcome::Published(_) => assert!(temp.0.join("dest").exists()),
        RestoreOutcome::Unpublished(e) => {
            assert!(e.uncertain_outer.is_none());
            assert!(!temp.0.join("dest").exists());
        }
        RestoreOutcome::PublicationIndeterminate(_) => {
            panic!("healthy local backend unexpectedly indeterminate")
        }
    }
}
#[test]
//= docs/parsing-requirements.md#restore-ui-012
//= type=test
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
fn production_lifecycle_teardown_and_sink_faults_during_execute_keep_typed_outcome() {
    use duplicity_tui::terminal::{self, Lifecycle, TeardownFaults};
    struct Injected {
        cancel: Cancellation,
        restored: bool,
    }
    impl Lifecycle for Injected {
        type Terminal = ();
        fn initialize(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn restore(&mut self) -> std::io::Result<()> {
            assert!(self.cancel.is_cancelled(), "cancel precedes teardown");
            self.restored = true;
            Err(std::io::Error::other(TeardownFaults {
                raw: Some(std::io::Error::other("raw")),
                screen: Some(std::io::Error::other("screen")),
                cursor: Some(std::io::Error::other("cursor")),
            }))
        }
    }
    let temp = Temp::new();
    let mut worker = worker();
    let anchor = match receive(&mut worker) {
        Reply::Ready(rows, _) => {
            rows.into_iter()
                .find(|r| r.end == Timestamp(1700000000))
                .unwrap()
                .index
        }
        _ => panic!("ready"),
    };
    worker
        .send(Command::Scan {
            id: 1,
            anchor,
            target: Timestamp(1700000000),
            cancel: Cancellation::default(),
        })
        .unwrap();
    let snapshot = match receive(&mut worker) {
        Reply::Snapshot { value, .. } => value,
        _ => panic!("scan"),
    };
    let destination = Arc::new(Destination {
        parent: temp.0.clone(),
        name: b"dest".to_vec(),
    });
    worker
        .send(Command::Prepare {
            id: 2,
            snapshot,
            marked: Vec::new(),
            destination: destination.clone(),
            cancel: Cancellation::default(),
        })
        .unwrap();
    let frozen = match receive(&mut worker) {
        Reply::Preview {
            result: Ok(value), ..
        } => value.frozen,
        _ => panic!("prepare"),
    };
    let mut cancel = Cancellation::default();
    let mut lifecycle = Injected {
        cancel: cancel.clone(),
        restored: false,
    };
    let completion = terminal::interact(
        &mut worker,
        &mut cancel,
        &mut lifecycle,
        |_, worker, cancel| {
            worker.send(Command::Execute {
                id: 3,
                frozen,
                cancel: cancel.clone(),
            })?;
            Err(std::io::Error::other("injected event/render failure"))
        },
    );
    assert!(lifecycle.restored);
    assert!(worker.is_closed(), "request path closes before drain/join");
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("stderr failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = duplicity_tui::worker::finish(
        worker,
        &cancel,
        completion,
        None,
        Some(destination),
        Some(3),
        &mut Broken,
    )
    .unwrap_err();
    let fault = error
        .get_ref()
        .unwrap()
        .downcast_ref::<restore_flow::ShutdownFailure>()
        .unwrap();
    assert!(fault.initialization.is_none());
    assert!(fault.terminal.is_some());
    assert!(fault.teardown.is_some());
    let teardown = fault
        .teardown
        .as_ref()
        .unwrap()
        .get_ref()
        .unwrap()
        .downcast_ref::<TeardownFaults>()
        .unwrap();
    assert!(teardown.raw.is_some() && teardown.screen.is_some() && teardown.cursor.is_some());
    assert!(fault.shutdown.is_none());
    assert!(fault.output.is_some());
    assert!(fault.outcome.is_some());
    assert!(!fault.unavailable);
}
#[test]
//= docs/parsing-requirements.md#restore-ui-012
//= type=test
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
fn partial_terminal_init_failure_closes_ready_queue_and_attempts_teardown() {
    use duplicity_tui::terminal::{self, Lifecycle};
    struct Partial {
        cancel: Cancellation,
        changed: bool,
        restored: bool,
    }
    impl Lifecycle for Partial {
        type Terminal = ();
        fn initialize(&mut self) -> std::io::Result<()> {
            self.changed = true;
            Err(std::io::Error::other("after partial terminal init"))
        }
        fn restore(&mut self) -> std::io::Result<()> {
            assert!(self.changed && self.cancel.is_cancelled());
            self.restored = true;
            Err(std::io::Error::other("teardown failure"))
        }
    }
    let mut worker = worker();
    std::thread::sleep(Duration::from_millis(20));
    let mut cancel = Cancellation::default();
    let mut lifecycle = Partial {
        cancel: cancel.clone(),
        changed: false,
        restored: false,
    };
    let completion = terminal::interact(&mut worker, &mut cancel, &mut lifecycle, |_, _, _| {
        panic!("event loop must not run after init error")
    });
    assert!(worker.is_closed());
    assert!(lifecycle.restored);
    let mut sink = Vec::new();
    let error =
        duplicity_tui::worker::finish(worker, &cancel, completion, None, None, Some(0), &mut sink)
            .unwrap_err();
    let fault = error
        .get_ref()
        .unwrap()
        .downcast_ref::<restore_flow::ShutdownFailure>()
        .unwrap();
    assert!(fault.initialization.is_some());
    assert!(fault.terminal.is_none());
    assert!(fault.teardown.is_some());
    assert!(fault.shutdown.is_none());
    assert!(fault.outcome.is_none());
    assert!(!fault.unavailable);
    assert!(sink.is_empty());
}

//! One owned thread and capacity-one queues; shutdown closes/drains before join.
use crate::{
    cli::Destination,
    credentials::ReadCredentials,
    display,
    restore_flow::{self, Frozen, Preview},
};
use duplicity_core::{
    chain::SetKind,
    names::{NameConfig, Timestamp},
    reader::{Catalog, ReaderError, ReaderLimits, ValidatedSnapshot},
    source::{Cancellation, ObjectSource},
};
use std::{
    io,
    sync::{mpsc, Arc},
    thread::JoinHandle,
};
pub struct History {
    pub index: usize,
    pub kind: SetKind,
    pub end: Timestamp,
    pub label: String,
}
pub enum Command {
    Scan {
        id: u64,
        anchor: usize,
        target: Timestamp,
        credentials: Option<Arc<ReadCredentials>>,
        cancel: Cancellation,
    },
    Prepare {
        id: u64,
        snapshot: Arc<ValidatedSnapshot>,
        marked: Vec<usize>,
        destination: Arc<Destination>,
        cancel: Cancellation,
    },
    Execute {
        id: u64,
        frozen: Frozen,
        cancel: Cancellation,
    },
}
impl Command {
    pub fn id(&self) -> u64 {
        match self {
            Self::Scan { id, .. } | Self::Prepare { id, .. } | Self::Execute { id, .. } => *id,
        }
    }
}
pub enum Reply {
    Ready(Vec<History>, Option<u64>),
    Snapshot {
        id: u64,
        value: Arc<ValidatedSnapshot>,
    },
    ReadFailure {
        id: u64,
        error: ReaderError,
    },
    Preview {
        id: u64,
        result: Result<Preview, restore_flow::Error>,
    },
    #[cfg(target_os = "linux")]
    Outcome {
        id: u64,
        result: Result<duplicity_core::restore_linux::RestoreOutcome, restore_flow::Error>,
    },
    Error {
        id: u64,
        text: &'static str,
    },
}
impl Reply {
    pub fn id(&self) -> u64 {
        match self {
            Self::Ready(..) => 0,
            Self::Snapshot { id, .. }
            | Self::ReadFailure { id, .. }
            | Self::Preview { id, .. }
            | Self::Error { id, .. } => *id,
            #[cfg(target_os = "linux")]
            Self::Outcome { id, .. } => *id,
        }
    }
}

//= docs/parsing-requirements.md#parse-env-009
//= type=implementation
//# The adapter MUST support caller-supplied passphrases and exported supported secret keys without invoking external key agents or crypto applications.
//= docs/parsing-requirements.md#parse-env-021
//= type=implementation
//# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
//= docs/parsing-requirements.md#restore-ui-015
//= type=implementation
//# The worker MUST build a borrowed CryptoReadContext from the selected profile, preserve typed ReaderError values across its bounded reply channel, and install a snapshot only after that context succeeds.
fn perform_scan(
    catalog: &Catalog<'_>,
    anchor: usize,
    target: Timestamp,
    cancel: &Cancellation,
    credentials: Option<&ReadCredentials>,
) -> Result<ValidatedSnapshot, ReaderError> {
    match credentials {
        Some(credentials) => credentials.with_context(|context| {
            catalog.read_snapshot_with_crypto(anchor, target, cancel, context)
        }),
        None => catalog.read_snapshot(anchor, target, cancel),
    }
}

//= docs/parsing-requirements.md#restore-ui-006
//= type=implementation
//# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
//= docs/parsing-requirements.md#restore-ui-012
//= type=implementation
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
pub struct Worker {
    requests: Option<mpsc::SyncSender<Command>>,
    replies: mpsc::Receiver<Reply>,
    handle: JoinHandle<()>,
    active: Option<u64>,
    last: u64,
    #[cfg(test)]
    close_observer: Option<mpsc::SyncSender<(bool, bool)>>,
    #[cfg(test)]
    join_observer: Option<mpsc::SyncSender<()>>,
}
impl Worker {
    pub fn start(
        source: Arc<dyn ObjectSource>,
        prefix: String,
        cancel: Cancellation,
    ) -> io::Result<Self> {
        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let handle = std::thread::Builder::new()
            .name("duplicity-reader".into())
            .spawn(move || {
                let catalog = match Catalog::discover(
                    source.as_ref(),
                    &prefix,
                    NameConfig::default(),
                    ReaderLimits::default(),
                    &cancel,
                ) {
                    Ok(c) => c,
                    Err(_) => {
                        let _ = responses.send(Reply::Error {
                            id: 0,
                            text: "Source discovery failed or was cancelled.",
                        });
                        return;
                    }
                };
                let mut rows = Vec::new();
                let mut charge = 0usize;
                for (index, object, set) in catalog.manifests() {
                    charge = match object
                        .key
                        .len()
                        .checked_mul(4)
                        .and_then(|n| n.checked_add(256 + std::mem::size_of::<History>()))
                        .and_then(|n| charge.checked_add(n))
                    {
                        Some(n) if n <= 32 * 1024 * 1024 && rows.len() < 100_000 => n,
                        _ => {
                            let _ = responses.send(Reply::Error {
                                id: 0,
                                text: "History display limit exceeded.",
                            });
                            return;
                        }
                    };
                    if cancel.is_cancelled() {
                        let _ = responses.send(Reply::Error {
                            id: 0,
                            text: "Discovery cancelled.",
                        });
                        return;
                    }
                    if rows.try_reserve_exact(1).is_err() {
                        let _ = responses.send(Reply::Error {
                            id: 0,
                            text: "History allocation failed.",
                        });
                        return;
                    }
                    rows.push(History {
                        index,
                        kind: set.kind(),
                        end: set.end(),
                        label: format!(
                            "{:?} {} -> {}  {}",
                            set.kind(),
                            set.start().0,
                            set.end().0,
                            display::bytes(object.key.as_bytes())
                        ),
                    });
                }
                rows.sort_by_key(|row| {
                    (
                        row.end.0,
                        if row.kind == SetKind::Full { 0 } else { 1 },
                        row.index,
                    )
                });
                let total = catalog
                    .objects()
                    .iter()
                    .try_fold(0u64, |n, o| n.checked_add(o.object().size));
                if responses.send(Reply::Ready(rows, total)).is_err() {
                    return;
                }
                while let Ok(command) = inbox.recv() {
                    let reply = match command {
                        Command::Scan {
                            id,
                            anchor,
                            target,
                            credentials,
                            cancel,
                        } => {
                            let result = perform_scan(
                                &catalog,
                                anchor,
                                target,
                                &cancel,
                                credentials.as_deref(),
                            );
                            match result {
                                Ok(value) => Reply::Snapshot {
                                    id,
                                    value: Arc::new(value),
                                },
                                Err(error) => Reply::ReadFailure { id, error },
                            }
                        }
                        Command::Prepare {
                            id,
                            snapshot,
                            marked,
                            destination,
                            cancel,
                        } => Reply::Preview {
                            id,
                            result: restore_flow::prepare(snapshot, marked, destination, &cancel),
                        },
                        Command::Execute { id, frozen, cancel } => {
                            #[cfg(target_os = "linux")]
                            {
                                #[cfg(test)]
                                execute_cancel_test_gate(id, &cancel);
                                let result = restore_flow::execute(frozen, &cancel);
                                #[cfg(test)]
                                record_test_execute_result(id, &result);
                                Reply::Outcome { id, result }
                            }
                            #[cfg(not(target_os = "linux"))]
                            {
                                let _ = (frozen, cancel);
                                Reply::Error {
                                    id,
                                    text: "Native restoration unsupported on this OS.",
                                }
                            }
                        }
                    };
                    if responses.send(reply).is_err() {
                        return;
                    }
                }
            })?;
        Ok(Self {
            requests: Some(requests),
            replies,
            handle,
            active: Some(0),
            last: 0,
            #[cfg(test)]
            close_observer: None,
            #[cfg(test)]
            join_observer: None,
        })
    }
    pub fn send(&mut self, command: Command) -> io::Result<()> {
        let id = command.id();
        if self.active.is_some() || id <= self.last {
            return Err(io::Error::other("worker busy or stale operation"));
        }
        self.requests
            .as_ref()
            .ok_or_else(|| io::Error::other("worker closed"))?
            .try_send(command)
            .map_err(|_| io::Error::other("worker unavailable"))?;
        self.active = Some(id);
        self.last = id;
        Ok(())
    }
    pub fn poll(&mut self) -> Result<Reply, mpsc::TryRecvError> {
        let reply = self.replies.try_recv()?;
        if self.active == Some(reply.id()) {
            self.active = None;
        }
        Ok(reply)
    }
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=implementation
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    pub fn close(&mut self, cancel: &Cancellation) {
        cancel.cancel();
        drop(self.requests.take());
        #[cfg(test)]
        if let Some(observer) = self.close_observer.take() {
            let _ = observer.send((cancel.is_cancelled(), self.requests.is_none()));
        }
    }
    pub fn is_closed(&self) -> bool {
        self.requests.is_none()
    }
    pub fn shutdown(
        mut self,
        cancel: &Cancellation,
        mut observe: impl FnMut(Reply),
    ) -> io::Result<()> {
        self.close(cancel);
        while let Ok(reply) = self.replies.recv() {
            observe(reply);
        }
        let joined = self.handle.join();
        #[cfg(test)]
        if let Some(observer) = self.join_observer.take() {
            let _ = observer.send(());
        }
        joined.map_err(|_| io::Error::other("worker outcome unavailable; inspection required"))
    }
}

// The deterministic active-restore PTY acceptance holds the actual Execute
// command immediately before backend entry. It is absent from production builds.
#[cfg(all(test, target_os = "linux"))]
fn execute_cancel_test_gate(id: u64, cancel: &Cancellation) {
    use std::{
        fs,
        path::PathBuf,
        thread,
        time::{Duration, Instant},
    };

    let Some(control) = std::env::var_os("DUPLINAUT_RESTORE_CANCEL_CONTROL") else {
        return;
    };
    let control = PathBuf::from(control);
    assert!(
        !control.join("execute-entered").exists(),
        "only one Execute may enter this acceptance gate"
    );
    assert!(
        !cancel.is_cancelled(),
        "Execute token cancelled before gate entry"
    );
    let marker = |name: &str, body: &[u8]| {
        let path = control.join(name);
        let temporary = control.join(format!("{name}.pending"));
        fs::write(&temporary, body).expect("write restore-cancel gate marker");
        fs::rename(temporary, path).expect("publish restore-cancel gate marker");
    };
    marker(
        "execute-entered",
        format!("operation_id={id};cancelled=false\n").as_bytes(),
    );
    let token = cancel.clone();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !token.is_cancelled() {
        assert!(
            Instant::now() < deadline,
            "Execute gate timed out waiting for UI cancellation"
        );
        thread::sleep(Duration::from_millis(5));
    }
    marker(
        "cancel-observed",
        format!("operation_id={id};cancelled=true\n").as_bytes(),
    );
    let expected = format!("release operation_id={id}\n");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if fs::read(control.join("release")).is_ok_and(|bytes| bytes == expected.as_bytes()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Execute gate timed out waiting for parent release"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(all(test, target_os = "linux"))]
fn record_test_execute_result(
    id: u64,
    result: &Result<duplicity_core::restore_linux::RestoreOutcome, restore_flow::Error>,
) {
    use std::{fs, path::PathBuf};
    let Some(control) = std::env::var_os("DUPLINAUT_RESTORE_CANCEL_CONTROL") else {
        return;
    };
    let outcome = match result {
        Ok(duplicity_core::restore_linux::RestoreOutcome::Unpublished(value))
            if value.cause == duplicity_core::restore_linux::Cause::Cancelled
                && value.uncertain_outer.is_none() =>
        {
            "outcome=Unpublished;cause=Cancelled;uncertain_outer=false"
        }
        Ok(duplicity_core::restore_linux::RestoreOutcome::Unpublished(value)) => {
            panic!("unexpected unpublished Execute result: {value:?}")
        }
        Ok(duplicity_core::restore_linux::RestoreOutcome::Published(_)) => {
            panic!("cancelled Execute unexpectedly published")
        }
        Ok(duplicity_core::restore_linux::RestoreOutcome::PublicationIndeterminate(_)) => {
            panic!("cancelled Execute unexpectedly became indeterminate")
        }
        Err(error) => panic!("cancelled Execute did not return a typed backend outcome: {error:?}"),
    };
    let control = PathBuf::from(control);
    let temporary = control.join("execute-result.pending");
    fs::write(&temporary, format!("operation_id={id};{outcome}\n"))
        .expect("write restore-cancel result marker");
    fs::rename(temporary, control.join("execute-result"))
        .expect("publish restore-cancel result marker");
}
/// Called after terminal teardown was attempted. Keeps a known typed result even
/// if the terminal, join or fallback sink failed; no modifying recovery follows.
#[cfg(target_os = "linux")]
//= docs/parsing-requirements.md#restore-ui-012
//= type=implementation
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
pub fn finish(
    worker: Worker,
    cancel: &Cancellation,
    terminal: crate::terminal::Completion,
    mut outcome: Option<duplicity_core::restore_linux::RestoreOutcome>,
    destination: Option<Arc<Destination>>,
    operation: Option<u64>,
    sink: &mut impl std::io::Write,
) -> io::Result<()> {
    let shutdown = worker.shutdown(cancel, |reply| {
        if let Reply::Outcome {
            result: Ok(value), ..
        } = reply
        {
            outcome = Some(value);
        }
    });
    if terminal.failed() || shutdown.is_err() {
        let output = if let Some(value) = &outcome {
            restore_flow::fallback(value, sink).err()
        } else if shutdown.is_err() {
            match restore_flow::unavailable(destination.as_deref(), operation) {
                Ok(text) => sink.write_all(text.as_bytes()).err(),
                Err(_) => Some(io::Error::other("outcome unavailable diagnostic limit")),
            }
        } else {
            None
        };
        let unavailable = shutdown.is_err() && outcome.is_none();
        return Err(io::Error::other(restore_flow::ShutdownFailure {
            initialization: terminal.initialization,
            terminal: terminal.event,
            teardown: terminal.teardown,
            shutdown: shutdown.err(),
            output,
            outcome,
            unavailable,
            operation,
            requested_destination: destination,
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use duplicity_core::{
        local::LocalSource,
        reader::{OpenPgpError, OpenPgpPassword},
    };
    #[test]
    //= docs/parsing-requirements.md#restore-ui-006
    //= type=test
    //# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn closed_requests_are_drained_before_join_even_with_full_reply_queue() {
        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            responses
                .send(Reply::Error {
                    id: 0,
                    text: "first",
                })
                .unwrap();
            responses
                .send(Reply::Error {
                    id: 1,
                    text: "second",
                })
                .unwrap();
            assert!(inbox.recv().is_err());
        });
        let worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: Some(0),
            last: 0,
            close_observer: None,
            join_observer: None,
        };
        let mut seen = Vec::new();
        worker
            .shutdown(&Cancellation::default(), |reply| seen.push(reply.id()))
            .unwrap();
        assert_eq!(seen, [0, 1]);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-env-009
    //= type=test
    //# The adapter MUST support caller-supplied passphrases and exported supported secret keys without invoking external key agents or crypto applications.
    //= docs/parsing-requirements.md#parse-env-014
    //= type=test
    //# The adapter MUST return unsuccessful restoration for any failed integrity, key-binding, configured signer, or complete-read check without publishing staged plaintext.
    //= docs/parsing-requirements.md#parse-env-021
    //= type=test
    //# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# The worker MUST build a borrowed CryptoReadContext from the selected profile, preserve typed ReaderError values across its bounded reply channel, and install a snapshot only after that context succeeds.
    fn crypto_scan_preserves_typed_failure_and_uses_borrowed_symmetric_context() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
        let source = LocalSource::new(fixture).unwrap();
        let mut worker =
            Worker::start(Arc::new(source), String::new(), Cancellation::default()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let anchor = loop {
            match worker.poll() {
                Ok(Reply::Ready(rows, _)) => {
                    break rows
                        .into_iter()
                        .find(|row| row.kind == SetKind::Full)
                        .expect("full fixture row")
                        .index;
                }
                Ok(_) => panic!("unexpected startup reply"),
                Err(mpsc::TryRecvError::Empty) => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("worker disconnected during discovery: {error:?}"),
            }
        };
        worker
            .send(Command::Scan {
                id: 1,
                anchor,
                target: Timestamp(1700172800),
                credentials: Some(Arc::new(ReadCredentials::Symmetric {
                    message_passphrase: OpenPgpPassword::from("incorrect-public-test-passphrase"),
                })),
                cancel: Cancellation::default(),
            })
            .unwrap();
        let wrong_reply = loop {
            match worker.poll() {
                Ok(reply) => break reply,
                Err(mpsc::TryRecvError::Empty) => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("worker disconnected during wrong credential scan: {error:?}"),
            }
        };
        match wrong_reply {
            Reply::ReadFailure {
                id: 1,
                error:
                    ReaderError::Crypto(
                        OpenPgpError::WrongCredentials | OpenPgpError::AuthenticationFailed,
                    ),
            } => {}
            Reply::ReadFailure { .. } => panic!("wrong credential returned another typed error"),
            Reply::Snapshot { .. } => panic!("wrong credential produced a snapshot"),
            _ => panic!("wrong credential returned a non-scan reply"),
        }
        worker
            .send(Command::Scan {
                id: 2,
                anchor,
                target: Timestamp(1700172800),
                credentials: Some(Arc::new(ReadCredentials::Symmetric {
                    message_passphrase: OpenPgpPassword::from(
                        "duplicity-reader-public-test-fixture",
                    ),
                })),
                cancel: Cancellation::default(),
            })
            .unwrap();
        let success_reply = loop {
            match worker.poll() {
                Ok(reply) => break reply,
                Err(mpsc::TryRecvError::Empty) => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("worker disconnected during valid credential scan: {error:?}"),
            }
        };
        match success_reply {
            Reply::Snapshot { id: 2, value } => {
                assert_eq!(value.target(), Timestamp(1700172800));
                assert_eq!(value.authentication_evidence().objects().len(), 6);
                assert!(value
                    .authentication_evidence()
                    .objects()
                    .iter()
                    .all(|record| record.kind
                        == duplicity_core::reader::AuthenticationKind::OpenPgpIntegrity));
            }
            Reply::ReadFailure { .. } => panic!("valid credentials failed to read snapshot"),
            _ => panic!("valid credentials returned a non-scan reply"),
        }
        worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
    }
    #[test]
    //= docs/parsing-requirements.md#restore-ui-006
    //= type=test
    //# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
    //= docs/parsing-requirements.md#restore-ui-009
    //= type=test
    //# The frontend MUST refrain from automatic retry, rollback, destination inspection-based outcome promotion or modifying cleanup after publication uncertainty or an unavailable worker outcome.
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn stale_ids_do_not_clear_current_operation_and_panics_are_unavailable() {
        let (requests, _inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        responses
            .send(Reply::Error {
                id: 1,
                text: "stale",
            })
            .unwrap();
        let handle = std::thread::spawn(move || {
            drop(responses);
        });
        let mut worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: Some(2),
            last: 2,
            close_observer: None,
            join_observer: None,
        };
        assert_eq!(worker.poll().unwrap().id(), 1);
        assert_eq!(worker.active, Some(2));
        assert!(worker
            .send(Command::Scan {
                id: 3,
                anchor: 0,
                target: Timestamp(0),
                credentials: None,
                cancel: Cancellation::default()
            })
            .is_err());
        worker.shutdown(&Cancellation::default(), |_| {}).unwrap();
        let (requests, _inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            drop(responses);
            panic!("injected worker failure");
        });
        let worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: Some(0),
            last: 0,
            close_observer: None,
            join_observer: None,
        };
        assert!(worker.shutdown(&Cancellation::default(), |_| {}).is_err());
    }
    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn lifecycle_worker_result_loss_is_unavailable_not_unpublished() {
        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            responses
                .send(Reply::Error {
                    id: 0,
                    text: "queued metadata",
                })
                .unwrap();
            assert!(inbox.recv().is_err());
            panic!("injected result loss");
        });
        let mut worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: Some(3),
            last: 3,
            close_observer: None,
            join_observer: None,
        };
        struct Faulty;
        impl crate::terminal::Lifecycle for Faulty {
            type Terminal = ();
            fn initialize(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn restore(&mut self) -> io::Result<()> {
                Err(io::Error::other("teardown"))
            }
        }
        let mut cancel = Cancellation::default();
        let terminal =
            crate::terminal::interact(&mut worker, &mut cancel, &mut Faulty, |_, _, _| {
                Err(io::Error::other("event"))
            });
        assert!(worker.is_closed());
        struct Broken;
        impl std::io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("output"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let result =
            finish(worker, &cancel, terminal, None, None, Some(3), &mut Broken).unwrap_err();
        let faults = result
            .get_ref()
            .unwrap()
            .downcast_ref::<restore_flow::ShutdownFailure>()
            .unwrap();
        assert!(faults.outcome.is_none());
        assert!(faults.unavailable);
        assert!(
            faults.terminal.is_some()
                && faults.teardown.is_some()
                && faults.shutdown.is_some()
                && faults.output.is_some()
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn partial_init_failure_closes_requests_before_teardown_and_drains_queued_ready() {
        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel(1);
        let (queued, ready) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            responses.send(Reply::Ready(Vec::new(), None)).unwrap();
            queued.send(()).unwrap();
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let mut worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: Some(0),
            last: 0,
            close_observer: None,
            join_observer: None,
        };
        struct Partial {
            inbox: mpsc::Receiver<Command>,
            cancel: Cancellation,
            restored: bool,
        }
        impl crate::terminal::Lifecycle for Partial {
            type Terminal = ();
            fn initialize(&mut self) -> io::Result<()> {
                Err(io::Error::other("partial init"))
            }
            fn restore(&mut self) -> io::Result<()> {
                assert!(self.cancel.is_cancelled());
                assert!(matches!(
                    self.inbox.try_recv(),
                    Err(mpsc::TryRecvError::Disconnected)
                ));
                self.restored = true;
                Err(io::Error::other("teardown"))
            }
        }
        let mut cancel = Cancellation::default();
        let mut lifecycle = Partial {
            inbox,
            cancel: cancel.clone(),
            restored: false,
        };
        let terminal =
            crate::terminal::interact(&mut worker, &mut cancel, &mut lifecycle, |_, _, _| {
                panic!("no event loop after init failure")
            });
        assert!(lifecycle.restored);
        let error = finish(
            worker,
            &cancel,
            terminal,
            None,
            None,
            Some(0),
            &mut Vec::new(),
        )
        .unwrap_err();
        let faults = error
            .get_ref()
            .unwrap()
            .downcast_ref::<restore_flow::ShutdownFailure>()
            .unwrap();
        assert!(faults.initialization.is_some() && faults.teardown.is_some());
        assert!(faults.shutdown.is_none() && faults.outcome.is_none() && !faults.unavailable);
    }

    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-007
    //= type=test
    //# The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.
    //= docs/parsing-requirements.md#restore-ui-008
    //= type=test
    //# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    // The event error and typed backend-shaped result are injected at the
    // lifecycle/worker boundary; the native filesystem backend is never called.
    fn injected_event_failure_while_execute_is_held_retains_synthetic_typed_result() {
        use duplicity_core::{
            chain::SetKind,
            local::LocalSource,
            names::{NameConfig, Timestamp},
            reader::{Catalog, ReaderLimits},
            restore_linux::{Cause, Cleanup, RestoreOutcome, Unpublished},
            source::Cancellation,
        };
        use std::{path::PathBuf, time::Duration};

        const OPERATION: u64 = 73;
        const EVENT_ERROR: &str = "injected active-restore event-loop error";
        const TEARDOWN_ERROR: &str = "injected active-restore teardown error";
        const OUTPUT_ERROR: &str = "injected active-restore fallback-write error";
        const PHASE_TIMEOUT: Duration = Duration::from_secs(10);

        let source = LocalSource::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/plain"),
        )
        .unwrap();
        let cancellation = Cancellation::default();
        let catalog = Catalog::discover(
            &source,
            "duplicity-",
            NameConfig::default(),
            ReaderLimits::default(),
            &cancellation,
        )
        .unwrap();
        let anchor = catalog
            .manifests()
            .find(|(_, _, set)| {
                set.kind() == SetKind::Full
                    && set.start() == Timestamp(1700000000)
                    && set.end() == Timestamp(1700000000)
            })
            .unwrap()
            .0;
        let snapshot = Arc::new(
            catalog
                .read_snapshot(anchor, Timestamp(1700172800), &cancellation)
                .unwrap(),
        );
        let frozen = restore_flow::prepare(
            snapshot,
            Vec::new(),
            Arc::new(crate::cli::Destination {
                parent: PathBuf::from("/unused/test-parent"),
                name: b"synthetic-cancelled-destination".to_vec(),
            }),
            &cancellation,
        )
        .unwrap()
        .frozen;

        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel::<Reply>(1);
        let (execute_entered_tx, execute_entered_rx) = mpsc::sync_channel::<u64>(1);
        let (event_error_tx, event_error_rx) = mpsc::sync_channel::<()>(1);
        let (close_observed_tx, close_observed_rx) = mpsc::sync_channel::<(bool, bool)>(1);
        let (release_tx, release_rx) = mpsc::sync_channel::<u64>(1);
        let (synthetic_result_tx, synthetic_result_rx) = mpsc::sync_channel::<u64>(1);
        let (worker_joined_tx, worker_joined_rx) = mpsc::sync_channel::<()>(1);
        let handle = std::thread::spawn(move || {
            let command = inbox.recv().expect("Execute request arrives");
            let (id, execute_cancel) = match command {
                Command::Execute { id, frozen, cancel } => {
                    drop(frozen);
                    (id, cancel)
                }
                _ => panic!("test worker expected only the Execute command"),
            };
            assert_eq!(id, OPERATION);
            assert!(!execute_cancel.is_cancelled());
            execute_entered_tx.send(id).unwrap();

            // This gate precedes any native filesystem backend call. The only
            // result this worker can send is the explicitly synthetic outcome.
            assert_eq!(release_rx.recv_timeout(PHASE_TIMEOUT).unwrap(), OPERATION);
            assert!(execute_cancel.is_cancelled());
            assert!(matches!(
                inbox.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ));
            let outcome = RestoreOutcome::Unpublished(Unpublished {
                cause: Cause::Cancelled,
                cleanup: Cleanup::default(),
                uncertain_outer: None,
            });
            responses
                .send(Reply::Outcome {
                    id,
                    result: Ok(outcome),
                })
                .unwrap();
            synthetic_result_tx.send(id).unwrap();
        });

        let mut worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: None,
            last: 0,
            close_observer: Some(close_observed_tx),
            join_observer: Some(worker_joined_tx),
        };
        worker
            .send(Command::Execute {
                id: OPERATION,
                frozen,
                cancel: cancellation.clone(),
            })
            .unwrap();

        struct InjectedLifecycle {
            cancellation: Cancellation,
            close_observed: mpsc::Receiver<(bool, bool)>,
            event_error: mpsc::Receiver<()>,
            restored: bool,
            close_state: Option<(bool, bool)>,
        }
        impl crate::terminal::Lifecycle for InjectedLifecycle {
            type Terminal = ();
            fn initialize(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn restore(&mut self) -> io::Result<()> {
                self.event_error
                    .try_recv()
                    .expect("event-loop failure precedes close observation");
                let close_state = self
                    .close_observed
                    .recv_timeout(PHASE_TIMEOUT)
                    .expect("close observer phase arrives before lifecycle restore");
                assert!(self.cancellation.is_cancelled());
                assert_eq!(close_state, (true, true));
                self.close_state = Some(close_state);
                self.restored = true;
                Err(io::Error::other(TEARDOWN_ERROR))
            }
        }
        let mut lifecycle = InjectedLifecycle {
            cancellation: cancellation.clone(),
            close_observed: close_observed_rx,
            event_error: event_error_rx,
            restored: false,
            close_state: None,
        };
        let mut lifecycle_cancellation = cancellation.clone();
        let completion = crate::terminal::interact(
            &mut worker,
            &mut lifecycle_cancellation,
            &mut lifecycle,
            |_, _, _| {
                assert_eq!(
                    execute_entered_rx.recv_timeout(PHASE_TIMEOUT).unwrap(),
                    OPERATION
                );
                event_error_tx.send(()).unwrap();
                Err(io::Error::other(EVENT_ERROR))
            },
        );
        assert_eq!(completion.event.as_ref().unwrap().to_string(), EVENT_ERROR);
        assert_eq!(
            completion.teardown.as_ref().unwrap().to_string(),
            TEARDOWN_ERROR
        );
        assert!(lifecycle.restored);
        assert_eq!(lifecycle.close_state, Some((true, true)));
        assert!(cancellation.is_cancelled());
        assert!(worker.is_closed());

        // Gate release happens only after event failure, cancellation/sender
        // closure observation, and the required teardown attempt/error.
        release_tx.send(OPERATION).unwrap();
        assert_eq!(
            synthetic_result_rx
                .recv_timeout(PHASE_TIMEOUT)
                .expect("synthetic typed result was published after release"),
            OPERATION
        );

        struct BrokenSink;
        impl std::io::Write for BrokenSink {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other(OUTPUT_ERROR))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let error = finish(
            worker,
            &cancellation,
            completion,
            None,
            None,
            Some(OPERATION),
            &mut BrokenSink,
        )
        .unwrap_err();
        worker_joined_rx
            .recv_timeout(PHASE_TIMEOUT)
            .expect("finish called JoinHandle::join and returned after the worker exited");
        let failure = error
            .get_ref()
            .and_then(|payload| payload.downcast_ref::<restore_flow::ShutdownFailure>())
            .expect("finish retains the structured shutdown failure");
        assert!(failure.initialization.is_none());
        assert_eq!(failure.terminal.as_ref().unwrap().to_string(), EVENT_ERROR);
        assert_eq!(
            failure.teardown.as_ref().unwrap().to_string(),
            TEARDOWN_ERROR
        );
        assert!(failure.shutdown.is_none());
        assert_eq!(failure.output.as_ref().unwrap().to_string(), OUTPUT_ERROR);
        assert_eq!(failure.operation, Some(OPERATION));
        assert!(!failure.unavailable);
        assert!(matches!(
            failure.outcome.as_ref(),
            Some(RestoreOutcome::Unpublished(error))
                if error.cause == Cause::Cancelled && error.uncertain_outer.is_none()
        ));
        assert!(cancellation.is_cancelled());
    }

    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-007
    //= type=test
    //# The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.
    //= docs/parsing-requirements.md#restore-ui-008
    //= type=test
    //# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
    //= docs/parsing-requirements.md#restore-ui-009
    //= type=test
    //# The frontend MUST refrain from automatic retry, rollback, destination inspection-based outcome promotion or modifying cleanup after publication uncertainty or an unavailable worker outcome.
    //= docs/parsing-requirements.md#restore-ui-010
    //= type=test
    //# The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    // This synthetic worker deliberately panics before a final backend result;
    // the test exercises frontend reporting only and never calls the native
    // filesystem restore backend.
    fn active_execute_worker_loss_reports_unavailable_with_escaped_destination() {
        use duplicity_core::{
            chain::SetKind,
            local::LocalSource,
            names::{NameConfig, Timestamp},
            reader::{Catalog, ReaderLimits},
            source::Cancellation,
        };
        use std::{
            ffi::OsString,
            os::unix::ffi::{OsStrExt, OsStringExt},
            path::PathBuf,
            time::Duration,
        };

        const OPERATION: u64 = 74;
        const EVENT_ERROR: &str = "injected unavailable-result event-loop error";
        const TEARDOWN_ERROR: &str = "injected unavailable-result teardown error";
        const PHASE_TIMEOUT: Duration = Duration::from_secs(10);

        let source = LocalSource::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/plain"),
        )
        .unwrap();
        let cancellation = Cancellation::default();
        let catalog = Catalog::discover(
            &source,
            "duplicity-",
            NameConfig::default(),
            ReaderLimits::default(),
            &cancellation,
        )
        .unwrap();
        let anchor = catalog
            .manifests()
            .find(|(_, _, set)| {
                set.kind() == SetKind::Full
                    && set.start() == Timestamp(1700000000)
                    && set.end() == Timestamp(1700000000)
            })
            .unwrap()
            .0;
        let snapshot = Arc::new(
            catalog
                .read_snapshot(anchor, Timestamp(1700172800), &cancellation)
                .unwrap(),
        );
        let destination = Arc::new(crate::cli::Destination {
            parent: PathBuf::from(OsString::from_vec(
                b"/tmp/duplinaut-parent-\x1b\x01\xff".to_vec(),
            )),
            name: b"leaf-\x1b\x02\xfe".to_vec(),
        });
        let frozen =
            restore_flow::prepare(snapshot, Vec::new(), destination.clone(), &cancellation)
                .unwrap()
                .frozen;

        let (requests, inbox) = mpsc::sync_channel::<Command>(1);
        let (responses, replies) = mpsc::sync_channel::<Reply>(1);
        let (execute_entered_tx, execute_entered_rx) = mpsc::sync_channel::<u64>(1);
        let (event_error_tx, event_error_rx) = mpsc::sync_channel::<()>(1);
        let (close_observed_tx, close_observed_rx) = mpsc::sync_channel::<(bool, bool)>(1);
        let (release_tx, release_rx) = mpsc::sync_channel::<u64>(1);
        let (driver_panics_tx, driver_panics_rx) = mpsc::sync_channel::<u64>(1);
        let (join_returned_tx, join_returned_rx) = mpsc::sync_channel::<()>(1);
        let handle = std::thread::spawn(move || {
            let command = inbox.recv().expect("one Execute request arrives");
            let (id, execute_cancel) = match command {
                Command::Execute { id, frozen, cancel } => {
                    drop(frozen);
                    (id, cancel)
                }
                _ => panic!("test worker expected only the Execute command"),
            };
            assert_eq!(id, OPERATION);
            assert!(!execute_cancel.is_cancelled());
            execute_entered_tx.send(id).unwrap();

            // The held synthetic worker does not call the native backend.
            assert_eq!(release_rx.recv_timeout(PHASE_TIMEOUT).unwrap(), OPERATION);
            assert!(execute_cancel.is_cancelled());
            assert!(matches!(
                inbox.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ));
            drop(responses);
            driver_panics_tx.send(id).unwrap();
            panic!("injected worker loss before final backend result");
        });
        let mut worker = Worker {
            requests: Some(requests),
            replies,
            handle,
            active: None,
            last: 0,
            close_observer: Some(close_observed_tx),
            join_observer: Some(join_returned_tx),
        };
        worker
            .send(Command::Execute {
                id: OPERATION,
                frozen,
                cancel: cancellation.clone(),
            })
            .unwrap();
        assert_eq!(worker.active, Some(OPERATION));
        assert_eq!(worker.last, OPERATION);

        struct InjectedLifecycle {
            cancellation: Cancellation,
            close_observed: mpsc::Receiver<(bool, bool)>,
            event_error: mpsc::Receiver<()>,
            restored: bool,
        }
        impl crate::terminal::Lifecycle for InjectedLifecycle {
            type Terminal = ();
            fn initialize(&mut self) -> io::Result<()> {
                Ok(())
            }
            fn restore(&mut self) -> io::Result<()> {
                self.event_error
                    .try_recv()
                    .expect("event-loop failure precedes lifecycle restoration");
                assert_eq!(
                    self.close_observed.recv_timeout(PHASE_TIMEOUT).unwrap(),
                    (true, true)
                );
                assert!(self.cancellation.is_cancelled());
                self.restored = true;
                Err(io::Error::other(TEARDOWN_ERROR))
            }
        }
        let mut lifecycle = InjectedLifecycle {
            cancellation: cancellation.clone(),
            close_observed: close_observed_rx,
            event_error: event_error_rx,
            restored: false,
        };
        let mut lifecycle_cancellation = cancellation.clone();
        let completion = crate::terminal::interact(
            &mut worker,
            &mut lifecycle_cancellation,
            &mut lifecycle,
            |_, _, _| {
                assert_eq!(
                    execute_entered_rx.recv_timeout(PHASE_TIMEOUT).unwrap(),
                    OPERATION
                );
                event_error_tx.send(()).unwrap();
                Err(io::Error::other(EVENT_ERROR))
            },
        );
        assert_eq!(completion.event.as_ref().unwrap().to_string(), EVENT_ERROR);
        assert_eq!(
            completion.teardown.as_ref().unwrap().to_string(),
            TEARDOWN_ERROR
        );
        assert!(lifecycle.restored);
        assert!(cancellation.is_cancelled());
        assert!(worker.is_closed());

        release_tx.send(OPERATION).unwrap();
        let mut sink = Vec::new();
        let error = finish(
            worker,
            &cancellation,
            completion,
            None,
            Some(destination.clone()),
            Some(OPERATION),
            &mut sink,
        )
        .unwrap_err();
        assert_eq!(
            driver_panics_rx.recv_timeout(PHASE_TIMEOUT).unwrap(),
            OPERATION
        );
        join_returned_rx
            .recv_timeout(PHASE_TIMEOUT)
            .expect("finish returned only after joining the panicked worker");
        assert!(cancellation.is_cancelled());

        let failure = error
            .get_ref()
            .and_then(|payload| payload.downcast_ref::<restore_flow::ShutdownFailure>())
            .expect("finish retains the structured unavailable-result failure");
        assert!(failure.outcome.is_none());
        assert!(failure.unavailable);
        assert!(failure.shutdown.is_some());
        assert_eq!(failure.operation, Some(OPERATION));
        assert!(Arc::ptr_eq(
            failure.requested_destination.as_ref().unwrap(),
            &destination
        ));
        assert_eq!(
            failure
                .requested_destination
                .as_ref()
                .unwrap()
                .parent
                .as_os_str()
                .as_bytes(),
            b"/tmp/duplinaut-parent-\x1b\x01\xff"
        );
        assert_eq!(
            failure.requested_destination.as_ref().unwrap().name,
            b"leaf-\x1b\x02\xfe"
        );
        assert_eq!(failure.terminal.as_ref().unwrap().to_string(), EVENT_ERROR);
        assert_eq!(
            failure.teardown.as_ref().unwrap().to_string(),
            TEARDOWN_ERROR
        );
        assert!(
            failure.output.is_none(),
            "the in-memory fallback sink succeeded"
        );

        let report = std::str::from_utf8(&sink).unwrap();
        assert!(report.contains("Restoration outcome unavailable; inspection required."));
        assert!(report.contains("Operation Some(74)."));
        assert!(report.contains("Requested parent: /tmp/duplinaut-parent-\\x1b\\x01\\xff"));
        assert!(report.contains("Requested new leaf: leaf-\\x1b\\x02\\xfe"));
        assert!(report.contains("No staging or publication absence is inferred."));
        assert!(report.contains("No retry, recovery or modifying cleanup is authorized."));
        assert!(
            !sink.contains(&0x1b),
            "raw ESC must not reach the report sink"
        );
        assert!(!report.contains("Unpublished"));
        assert!(!report.contains("Cancelled"));
    }
}

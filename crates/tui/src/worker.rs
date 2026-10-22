//! One owned thread and capacity-one queues; shutdown closes/drains before join.
use crate::{
    cli::Destination,
    display,
    restore_flow::{self, Frozen, Preview},
};
use duplicity_core::{
    chain::SetKind,
    names::{NameConfig, Timestamp},
    reader::{Catalog, ReaderLimits, ValidatedSnapshot},
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
            Self::Snapshot { id, .. } | Self::Preview { id, .. } | Self::Error { id, .. } => *id,
            #[cfg(target_os = "linux")]
            Self::Outcome { id, .. } => *id,
        }
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
                            cancel,
                        } => match catalog.read_snapshot(anchor, target, &cancel) {
                            Ok(value) => Reply::Snapshot {
                                id,
                                value: Arc::new(value),
                            },
                            Err(_) => Reply::Error {
                                id,
                                text: "Snapshot validation failed or was cancelled.",
                            },
                        },
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
                                Reply::Outcome {
                                    id,
                                    result: restore_flow::execute(frozen, &cancel),
                                }
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
        self.handle
            .join()
            .map_err(|_| io::Error::other("worker outcome unavailable; inspection required"))
    }
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
        };
        let mut seen = Vec::new();
        worker
            .shutdown(&Cancellation::default(), |reply| seen.push(reply.id()))
            .unwrap();
        assert_eq!(seen, [0, 1]);
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
        };
        assert_eq!(worker.poll().unwrap().id(), 1);
        assert_eq!(worker.active, Some(2));
        assert!(worker
            .send(Command::Scan {
                id: 3,
                anchor: 0,
                target: Timestamp(0),
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
}

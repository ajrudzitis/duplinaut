//! Terminal interaction over one lifecycle-owned storage/preview/restore worker.
use crate::{
    cli::Destination,
    display,
    restore_flow::{Acknowledgment, Preview},
    worker::{Command, History, Reply, Worker},
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use duplicity_core::{
    chain::SetKind,
    reader::ValidatedSnapshot,
    source::{Cancellation, ObjectSource},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    io,
    sync::{mpsc, Arc},
    time::Duration,
};
pub fn run(source: Arc<dyn ObjectSource>, prefix: String, initial: Cancellation) -> io::Result<()> {
    run_configured(source, prefix, initial, None)
}
fn pressed_key(event: Event) -> Option<KeyCode> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(key.code),
        _ => None,
    }
}
fn take_confirmation(ack: &mut Acknowledgment, code: KeyCode) -> bool {
    if code == KeyCode::Enter && ack.ready() {
        ack.clear();
        true
    } else {
        false
    }
}
fn r_transition(
    key: KeyCode,
    retry_allowed: bool,
    has_snapshot: bool,
    has_destination: bool,
    status: &mut String,
    prepare: impl FnOnce() -> io::Result<()>,
) -> io::Result<bool> {
    if key != KeyCode::Char('r') {
        return Ok(false);
    }
    if !retry_allowed {
        *status =
            "Further restores disabled after success or uncertain/unavailable outcome.".into();
        return Ok(false);
    }
    if has_snapshot && has_destination {
        prepare()?;
        Ok(true)
    } else {
        *status = "Restore requires a validated snapshot and paired --restore-parent/--restore-name. Nothing is written.".into();
        Ok(false)
    }
}

#[cfg(target_os = "linux")]
fn outcome_transition(
    result: Result<duplicity_core::restore_linux::RestoreOutcome, crate::restore_flow::Error>,
    retry: &mut bool,
    document: &mut Option<String>,
    offset: &mut usize,
    status: &mut String,
    known_outcome: &mut Option<duplicity_core::restore_linux::RestoreOutcome>,
) {
    match result {
        Ok(value) => {
            *retry = crate::restore_flow::retry_allowed(&value);
            *document = Some(
                crate::restore_flow::diagnostics(&value).unwrap_or_else(|_| {
                    "Known restore outcome retained; diagnostic rendering failed.".into()
                }),
            );
            *offset = 0;
            *status = "Restore result. Up/Down scroll; Esc returns; q exits.".into();
            *known_outcome = Some(value);
        }
        Err(_) => {
            *retry = false;
            *status =
                "Restore request failed before a typed backend result; inspection required, retries disabled."
                    .into();
        }
    }
}
#[cfg(test)]
mod key_event_tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn event(code: KeyCode, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-004
    //= type=test
    //# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
    fn only_press_events_change_and_confirm_restore_acknowledgment() {
        let mut ack = Acknowledgment::default();
        ack.armed = true;

        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            assert_eq!(pressed_key(event(KeyCode::Char('r'), kind)), None);
            assert_eq!(pressed_key(event(KeyCode::Enter, kind)), None);
        }
        assert_eq!(ack.text(), "");
        assert!(!take_confirmation(&mut ack, KeyCode::Enter));

        for character in "restore".chars() {
            let Some(KeyCode::Char(accepted)) =
                pressed_key(event(KeyCode::Char(character), KeyEventKind::Press))
            else {
                panic!("press character must be normalized");
            };
            ack.input(accepted);
        }
        assert!(ack.ready());
        let enter = pressed_key(event(KeyCode::Enter, KeyEventKind::Press)).unwrap();
        assert!(take_confirmation(&mut ack, enter));
        assert_eq!(ack.text(), "");
        assert!(!ack.armed);
        assert!(!take_confirmation(&mut ack, enter));
    }
}
//= docs/parsing-requirements.md#restore-ui-004
//= type=implementation
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-005
//= type=implementation
//# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
//= docs/parsing-requirements.md#restore-ui-006
//= type=implementation
//# The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.
//= docs/parsing-requirements.md#restore-ui-007
//= type=implementation
//# The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.
//= docs/parsing-requirements.md#restore-ui-008
//= type=implementation
//# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
//= docs/parsing-requirements.md#restore-ui-009
//= type=implementation
//# The frontend MUST refrain from automatic retry, rollback, destination inspection-based outcome promotion or modifying cleanup after publication uncertainty or an unavailable worker outcome.
//= docs/parsing-requirements.md#restore-ui-010
//= type=implementation
//# The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.
//= docs/parsing-requirements.md#restore-ui-012
//= type=implementation
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
//= docs/parsing-requirements.md#restore-ui-014
//= type=implementation
//# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
pub fn run_configured(
    source: Arc<dyn ObjectSource>,
    prefix: String,
    initial: Cancellation,
    destination: Option<Arc<Destination>>,
) -> io::Result<()> {
    let mut worker = Worker::start(source, prefix, initial.clone())?;
    let mut history: Vec<History> = Vec::new();
    let mut snapshot: Option<Arc<ValidatedSnapshot>> = None;
    let mut marked: Vec<usize> = Vec::new();
    let mut selected = 0usize;
    let mut anchor = None;
    let mut total = None;
    let mut active = Some(0u64);
    let mut serial = 0u64;
    let mut scan_confirm = false;
    let mut cancel = initial;
    let mut status = "Discovering object metadata...".to_string();
    let mut preview: Option<Preview> = None;
    let mut document: Option<String> = None;
    let mut ack = Acknowledgment::default();
    let mut offset = 0usize;
    let mut scroll_max = 0usize;
    let mut abandoning = false;
    let mut retry = true;
    #[cfg(target_os = "linux")]
    let mut known_outcome = None;
    let result = crate::terminal::interact(
        &mut worker,
        &mut cancel,
        &mut crate::terminal::Native::default(),
        |terminal, worker, cancel| -> io::Result<()> {
            loop {
                match worker.poll() {
                    Ok(reply) if active == Some(reply.id()) => {
                        active = None;
                        scan_confirm = false;
                        selected = 0;
                        ack.clear();
                        match reply {
                            Reply::Ready(rows, bytes) => {
                                history = rows;
                                total = bytes;
                                status="Metadata only. a selects full anchor; Enter selects exact target scan.".into();
                            }
                            Reply::Snapshot { value, .. } => {
                                marked.clear();
                                status=format!("Validated snapshot: {} entries, {} payload bytes. p read-only; r executable preview.",value.state().len(),value.state().payload_bytes());
                                snapshot = Some(value);
                            }
                            Reply::Preview { result, .. } => {
                                if abandoning {
                                    status = "Preview abandoned; nothing staged.".into();
                                    abandoning = false;
                                } else {
                                    match result {
                                        Ok(value) => {
                                            preview = Some(value);
                                            offset = 0;
                                            status="Preparing empty acknowledgment field; pending input is being drained.".into();
                                        }
                                        Err(_) => {
                                            status =
                                                "Executable preview failed; no restore is armed."
                                                    .into()
                                        }
                                    }
                                }
                            }
                            #[cfg(target_os = "linux")]
                            Reply::Outcome { result, .. } => outcome_transition(
                                result,
                                &mut retry,
                                &mut document,
                                &mut offset,
                                &mut status,
                                &mut known_outcome,
                            ),
                            Reply::Error { text, .. } => status = text.into(),
                        }
                    }
                    Ok(_) => {}
                    Err(mpsc::TryRecvError::Disconnected) if active.is_some() => {
                        active = None;
                        preview = None;
                        ack.clear();
                        retry = false;
                        document = Some(
                        crate::restore_flow::unavailable(destination.as_deref(), Some(serial))
                            .unwrap_or_else(|_| {
                                "Worker outcome unavailable; inspection required; retries disabled."
                                    .into()
                            }),
                    );
                        offset = 0;
                        status =
                            "Worker outcome unavailable; inspection required; retries disabled."
                                .into();
                    }
                    _ => {}
                }
                let count = snapshot.as_ref().map_or(history.len(), |s| s.state().len());
                terminal.draw(|frame|{let areas=Layout::vertical([Constraint::Length(3),Constraint::Min(2),Constraint::Length(6)]).split(frame.area());frame.render_widget(Paragraph::new(format!("DUPLICITY / BACKUP READER   full anchor: {anchor:?}")).style(Style::default().fg(Color::Cyan)).block(Block::bordered()),areas[0]);let height=usize::from(areas[1].height.saturating_sub(2)).max(1);
   if let Some(text)=preview.as_ref().map(|p|p.text.as_str()).or(document.as_deref()){let width=usize::from(areas[1].width.saturating_sub(2)).max(1);let wrapped=wrapped_rows(text,width);scroll_max=wrapped.clone().count().saturating_sub(1);offset=offset.min(scroll_max);let lines:Vec<&str>=wrapped.skip(offset).take(height).collect();frame.render_widget(Paragraph::new(lines.join("\n")).block(Block::bordered().title("Restore preview / outcome / Up-Down scroll")),areas[1]);}else{let first=selected.saturating_sub(height/2).min(count.saturating_sub(height));let items:Vec<ListItem>=if let Some(value)=&snapshot{value.state().entries().enumerate().skip(first).take(height).map(|(i,(path,entry))|ListItem::new(format!("{} {:?} {:03o} {:>10} {}",if marked.contains(&i){"*"}else{" "},entry.entry_type(),entry.mode()&0o777,entry.content().map_or(0,<[u8]>::len),display::path(path)))).collect()}else{history.iter().skip(first).take(height).map(|row|ListItem::new(row.label.as_str())).collect()};let mut cursor=ListState::default().with_selected((count!=0).then_some(selected.saturating_sub(first)));frame.render_stateful_widget(List::new(items).block(Block::bordered().title(if snapshot.is_some(){"Snapshot paths"}else{"History / metadata only"})).highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White)),areas[1],&mut cursor);}
   let footer=if preview.is_some(){format!("{status}\nAcknowledgment [{}] {}\nType restore then Enter | Up/Down scroll | Esc abandons",display::bytes(ack.text().as_bytes()),if ack.armed{"ready"}else{"unarmed"})}else{format!("{status}\nUp/Down | a anchor | Enter scan | Space mark | p read-only | r restore preview | Esc back/cancel | q quit")};frame.render_widget(Paragraph::new(footer).wrap(Wrap{trim:false}).block(Block::bordered()),areas[2]);})?;
                // The concrete preview and empty field have rendered. Bounded drain never
                // arms while terminal input remains buffered, including across iterations.
                if preview.is_some() && !ack.armed {
                    crate::restore_flow::drain_pending(
                        &mut ack,
                        || event::poll(Duration::ZERO),
                        || {
                            let _ = event::read()?;
                            Ok(())
                        },
                    )?;
                    if ack.armed {
                        status = "Type the exact ASCII word restore, then Enter.".into();
                    }
                    continue;
                }
                if !event::poll(Duration::from_millis(50))? {
                    continue;
                }
                let Some(code) = pressed_key(event::read()?) else {
                    continue;
                };
                if active.is_some() {
                    if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                        cancel.cancel();
                        abandoning = true;
                        status="Cancellation requested; waiting for operation outcome. q after the result exits.".into();
                    }
                    continue;
                }
                if preview.is_some() {
                    match code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            preview = None;
                            ack.clear();
                            status = "Restore preview abandoned; nothing staged.".into();
                        }
                        KeyCode::Up => offset = offset.saturating_sub(1),
                        KeyCode::Down => offset = offset.saturating_add(1).min(scroll_max),
                        KeyCode::Backspace => ack.backspace(),
                        KeyCode::Enter => {
                            if take_confirmation(&mut ack, code) {
                                let frozen = preview.take().unwrap().frozen;
                                serial = serial
                                    .checked_add(1)
                                    .ok_or_else(|| io::Error::other("operation ID exhausted"))?;
                                *cancel = Cancellation::default();
                                #[cfg(target_os = "linux")]
                                {
                                    known_outcome = None;
                                }
                                worker.send(Command::Execute {
                                    id: serial,
                                    frozen,
                                    cancel: cancel.clone(),
                                })?;
                                active = Some(serial);
                                abandoning = false;
                                status="Restoration running. Esc/q requests cancellation; waiting for typed outcome.".into();
                            } else {
                                status =
                                    "Acknowledgment must be exactly restore, then Enter.".into();
                            }
                        }
                        KeyCode::Char(c) => ack.input(c),
                        _ => {}
                    }
                    continue;
                }
                if document.is_some() {
                    match code {
                        KeyCode::Up => {
                            offset = offset.saturating_sub(1);
                            continue;
                        }
                        KeyCode::Down => {
                            offset = offset.saturating_add(1).min(scroll_max);
                            continue;
                        }
                        KeyCode::Esc => {
                            document = None;
                            offset = 0;
                            #[cfg(target_os = "linux")]
                            if let Some(value) = known_outcome.as_ref() {
                                status = match value {
                                    duplicity_core::restore_linux::RestoreOutcome::Unpublished(error) => {
                                        format!("Restore result retained: Unpublished: cause {:?}. q exits.", error.cause)
                                    }
                                    duplicity_core::restore_linux::RestoreOutcome::Published(_) => {
                                        "Restore result retained: Published. q exits.".into()
                                    }
                                    duplicity_core::restore_linux::RestoreOutcome::PublicationIndeterminate(_) => {
                                        "Restore result retained: Publication indeterminate. q exits.".into()
                                    }
                                };
                            }
                            continue;
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => {
                        snapshot = None;
                        marked.clear();
                        scan_confirm = false;
                        selected = 0;
                        status = "Metadata history. Choose anchor and exact target date.".into();
                    }
                    KeyCode::Down => {
                        selected = selected.saturating_add(1).min(count.saturating_sub(1));
                        scan_confirm = false;
                    }
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                        scan_confirm = false;
                    }
                    KeyCode::Char(' ') if snapshot.is_some() && count != 0 => {
                        if let Some(i) = marked.iter().position(|n| *n == selected) {
                            marked.remove(i);
                        } else if marked.len() < 1024 {
                            marked
                                .try_reserve_exact(1)
                                .map_err(|_| io::Error::other("selection allocation failed"))?;
                            marked.push(selected);
                        } else {
                            status = "Selection limit: 1024 roots.".into();
                            continue;
                        }
                        status = format!(
                            "{} marked roots. p read-only; r freezes selection and destination.",
                            marked.len()
                        );
                    }
                    KeyCode::Char('p') => {
                        if let Some(value) = &snapshot {
                            let plan = if marked.is_empty() {
                                value.restore_all(Default::default())
                            } else {
                                match crate::restore_flow::copy_roots(value, &marked) {
                                    Ok(roots) => value.restore_roots(&roots, Default::default()),
                                    Err(_) => {
                                        status = "Read-only selection limits exceeded.".into();
                                        continue;
                                    }
                                }
                            };
                            status=match plan{Ok(p)=>format!("Read-only preview:{} entries including source ancestors. Nothing is written.",p.entries().len()),Err(_)=>"Read-only selection preview failed.".into()};
                        }
                    }
                    KeyCode::Char('r') => {
                        let prepared = r_transition(
                            code,
                            retry,
                            snapshot.is_some(),
                            destination.is_some(),
                            &mut status,
                            || {
                                let (Some(value), Some(dest)) = (&snapshot, &destination) else {
                                    unreachable!("availability checked by r_transition")
                                };
                                serial = serial
                                    .checked_add(1)
                                    .ok_or_else(|| io::Error::other("operation ID exhausted"))?;
                                *cancel = Cancellation::default();
                                document = None;
                                worker.send(Command::Prepare {
                                    id: serial,
                                    snapshot: value.clone(),
                                    marked: marked.clone(),
                                    destination: dest.clone(),
                                    cancel: cancel.clone(),
                                })?;
                                active = Some(serial);
                                Ok(())
                            },
                        )?;
                        if prepared {
                            abandoning = false;
                            ack.clear();
                            status="Preparing frozen executable preview on worker; acknowledgment input is ignored.".into();
                        }
                        continue;
                    }
                    KeyCode::Char('a') if snapshot.is_none() => {
                        scan_confirm = false;
                        if let Some(row) = history.get(selected) {
                            if row.kind == SetKind::Full {
                                anchor = Some(row.index);
                                status = "Full anchor selected; choose target then Enter.".into();
                            } else {
                                status = "Anchor must be a full set.".into();
                            }
                        }
                    }
                    KeyCode::Enter if snapshot.is_none() => {
                        if let (Some(full), Some(row)) = (anchor, history.get(selected)) {
                            if !scan_confirm {
                                status=format!("Full volume scan required. Listed total {:?} bytes; exact download size unknown. Enter confirms.",total);
                                scan_confirm = true;
                            } else {
                                serial = serial
                                    .checked_add(1)
                                    .ok_or_else(|| io::Error::other("operation ID exhausted"))?;
                                *cancel = Cancellation::default();
                                worker.send(Command::Scan {
                                    id: serial,
                                    anchor: full,
                                    target: row.end,
                                    cancel: cancel.clone(),
                                })?;
                                active = Some(serial);
                                scan_confirm = false;
                                status="Reading and validating every required volume... Esc/q cancels and waits.".into();
                            }
                        } else {
                            status = "Choose a full anchor with a first.".into();
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        },
    );
    // Owned lifecycle has cancelled/closed before attempting every teardown
    // action; now drain before joining, preserving all terminal fault fields.
    drop(document);
    #[cfg(target_os = "linux")]
    {
        crate::worker::finish(
            worker,
            &cancel,
            result,
            known_outcome,
            destination,
            Some(serial),
            &mut io::stderr(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let shutdown = worker.shutdown(&cancel, |_| {});
        if result.failed() {
            Err(io::Error::other(result))
        } else {
            shutdown
        }
    }
}

fn wrapped_rows(text: &str, width: usize) -> impl Iterator<Item = &str> + Clone {
    let width = width.max(1);
    text.lines().flat_map(move |line| {
        line.as_bytes()
            .chunks(width)
            .map(|part| std::str::from_utf8(part).expect("ASCII safe text"))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    //= docs/parsing-requirements.md#restore-ui-010
    //= type=test
    //# The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.
    #[test]
    fn narrow_viewport_exposes_complete_long_single_line_identity() {
        let text = format!(
            "Requested parent: /{}\\xff",
            "long-path-component/".repeat(40)
        );
        assert_eq!(text.lines().count(), 1);
        let wrapped = wrapped_rows(&text, 7);
        let scroll_max = wrapped.clone().count().saturating_sub(1);
        assert!(scroll_max > 100);
        let mut offset = 0;
        let mut accessible = String::new();
        loop {
            accessible.push_str(wrapped_rows(&text, 7).nth(offset).unwrap());
            if offset == scroll_max {
                break;
            }
            offset = offset.saturating_add(1).min(scroll_max);
        }
        assert_eq!(accessible, text);
        assert!(accessible.ends_with("\\xff"));
        assert_eq!(wrapped_rows(&text, 1).count(), text.len());
    }

    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-008
    //= type=test
    //# The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.
    //= docs/parsing-requirements.md#restore-ui-009
    //= type=test
    //# The frontend MUST refrain from automatic retry, rollback, destination inspection-based outcome promotion or modifying cleanup after publication uncertainty or an unavailable worker outcome.
    //= docs/parsing-requirements.md#restore-ui-014
    //= type=test
    //# Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.
    // This test invokes the same typed-result and `r` transition helpers used
    // by the browser loop. Synthetic replies are state-machine evidence only;
    // no native backend or terminal interaction is exercised.
    fn blocked_typed_outcomes_retain_document_and_reject_r_transition() {
        use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers};
        use duplicity_core::{
            chain::SetKind,
            local::LocalSource,
            names::{NameConfig, Timestamp},
            reader::{Catalog, ReaderLimits},
            restore_linux::{
                Cause, Cleanup, Identity, Indeterminate, Profile, Receipt, RestoreOutcome,
                Unpublished,
            },
            source::Cancellation,
        };
        use std::{ffi::CString, path::PathBuf};

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
        let destination = Arc::new(Destination {
            parent: PathBuf::from("/tmp/retry-lockout-parent"),
            name: b"retry-lockout-leaf".to_vec(),
        });
        let receipt = || Receipt {
            destination: CString::new("retry-lockout-leaf").unwrap(),
            staging: CString::new(".retry-lockout-stage").unwrap(),
            profile: Profile::PrivateParentSameParent,
            parent: Identity {
                device: 10,
                inode: 20,
            },
            root: Identity {
                device: 10,
                inode: 30,
            },
            filesystem: 0xef53,
            unspecified_empty_root_mtime: false,
            cleanup: Cleanup::default(),
        };
        let cases = [
            (
                RestoreOutcome::Published(receipt()),
                "Published completed tree.",
                "Parent device=10 inode=20; root device=10 inode=30",
                0,
            ),
            (
                RestoreOutcome::Unpublished(Unpublished {
                    cause: Cause::Fidelity,
                    cleanup: Cleanup {
                        attempted: 2,
                        failures: 1,
                        ..Cleanup::default()
                    },
                    uncertain_outer: Some(CString::new("outer-candidate").unwrap()),
                }),
                "Unpublished: cause Fidelity",
                "First-stage candidate uncertain; no session retry: outer-candidate",
                1,
            ),
            (
                RestoreOutcome::PublicationIndeterminate(Indeterminate {
                    error: rustix::io::Errno::IO,
                    receipt: receipt(),
                }),
                "Publication indeterminate:",
                "Publication may have occurred; no retry, recovery or cleanup is authorized.",
                2,
            ),
        ];

        for (case_index, (outcome, expected_document, expected_evidence, expected_kind)) in
            cases.into_iter().enumerate()
        {
            let mut retry = true;
            let mut document = None;
            let mut offset = 17;
            let mut status = String::from("before result");
            let mut known_outcome = None;
            let reply = Reply::Outcome {
                id: 91 + case_index as u64,
                result: Ok(outcome),
            };
            let Reply::Outcome { result, .. } = reply else {
                unreachable!("test supplies a synthetic outcome reply")
            };
            outcome_transition(
                result,
                &mut retry,
                &mut document,
                &mut offset,
                &mut status,
                &mut known_outcome,
            );

            assert!(!retry);
            assert_eq!(offset, 0);
            assert_eq!(
                status,
                "Restore result. Up/Down scroll; Esc returns; q exits."
            );
            let retained_document = document.clone().unwrap();
            assert!(retained_document.contains(expected_document));
            assert!(retained_document.contains(expected_evidence));
            let assert_evidence = |value: &RestoreOutcome| match (expected_kind, value) {
                (0, RestoreOutcome::Published(r)) => {
                    assert_eq!(r.destination.to_bytes(), b"retry-lockout-leaf");
                    assert_eq!(r.parent.device, 10);
                    assert_eq!(r.root.inode, 30);
                }
                (1, RestoreOutcome::Unpublished(e)) => {
                    assert_eq!(e.cause, Cause::Fidelity);
                    assert_eq!(
                        e.uncertain_outer.as_ref().unwrap().to_bytes(),
                        b"outer-candidate"
                    );
                }
                (2, RestoreOutcome::PublicationIndeterminate(e)) => {
                    assert_eq!(e.error, rustix::io::Errno::IO);
                    assert_eq!(e.receipt.destination.to_bytes(), b"retry-lockout-leaf");
                }
                _ => panic!("typed result variant must remain unchanged"),
            };
            assert_evidence(known_outcome.as_ref().unwrap());

            let mut sent = Vec::new();
            let r_event = Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ));
            let key = pressed_key(r_event).expect("press-r event normalizes to a key");
            assert_eq!(key, KeyCode::Char('r'));
            let prepared = r_transition(
                key,
                retry,
                snapshot.is_some(),
                destination.is_some(),
                &mut status,
                || {
                    sent.push("Prepare");
                    Ok(())
                },
            )
            .unwrap();
            assert!(!prepared);
            assert_eq!(
                status,
                "Further restores disabled after success or uncertain/unavailable outcome."
            );
            assert!(
                sent.is_empty(),
                "no Prepare or Execute command may follow the result"
            );
            assert_eq!(document.as_deref(), Some(retained_document.as_str()));
            assert_evidence(known_outcome.as_ref().unwrap());
        }
    }
}

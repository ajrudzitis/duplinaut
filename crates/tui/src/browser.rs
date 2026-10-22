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
                            Reply::Outcome { result, .. } => match result {
                                Ok(value) => {
                                    retry = crate::restore_flow::retry_allowed(&value);
                                    document=Some(crate::restore_flow::diagnostics(&value).unwrap_or_else(|_|"Known restore outcome retained; diagnostic rendering failed.".into()));
                                    offset = 0;
                                    status =
                                        "Restore result. Up/Down scroll; Esc returns; q exits."
                                            .into();
                                    known_outcome = Some(value);
                                }
                                Err(_) => {
                                    retry = false;
                                    status="Restore request failed before a typed backend result; inspection required, retries disabled.".into();
                                }
                            },
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
                let Event::Key(key) = event::read()? else {
                    continue;
                };
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if active.is_some() {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                        cancel.cancel();
                        abandoning = true;
                        status="Cancellation requested; waiting for operation outcome. q after the result exits.".into();
                    }
                    continue;
                }
                if preview.is_some() {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            preview = None;
                            ack.clear();
                            status = "Restore preview abandoned; nothing staged.".into();
                        }
                        KeyCode::Up => offset = offset.saturating_sub(1),
                        KeyCode::Down => offset = offset.saturating_add(1).min(scroll_max),
                        KeyCode::Backspace => ack.backspace(),
                        KeyCode::Enter => {
                            if ack.ready() {
                                let frozen = preview.take().unwrap().frozen;
                                ack.clear();
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
                    match key.code {
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
                            continue;
                        }
                        _ => {}
                    }
                }
                match key.code {
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
                        if !retry {
                            status="Further restores disabled after success or uncertain/unavailable outcome.".into();
                            continue;
                        }
                        if let (Some(value), Some(dest)) = (&snapshot, &destination) {
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
                            abandoning = false;
                            ack.clear();
                            status="Preparing frozen executable preview on worker; acknowledgment input is ignored.".into();
                        } else {
                            status="Restore requires a validated snapshot and paired --restore-parent/--restore-name. Nothing is written.".into();
                        }
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
}

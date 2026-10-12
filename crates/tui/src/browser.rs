//! Read-only history and full-scan snapshot browsing on a storage worker.
use crate::display;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use duplicity_core::{
    chain::SetKind,
    names::{NameConfig, Timestamp},
    reader::{Catalog, ReaderLimits, ValidatedSnapshot},
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

struct History {
    index: usize,
    kind: SetKind,
    end: Timestamp,
    label: String,
}
enum Reply {
    Ready(Vec<History>, Option<u64>),
    Snapshot(ValidatedSnapshot),
    Error(String),
}
struct Scan {
    anchor: usize,
    target: Timestamp,
    cancel: Cancellation,
}

pub fn run(source: Arc<dyn ObjectSource>, prefix: String, initial: Cancellation) -> io::Result<()> {
    let (requests, inbox) = mpsc::sync_channel::<Scan>(1);
    let (responses, replies) = mpsc::sync_channel(1);
    let discovery_cancel = initial.clone();
    std::thread::Builder::new()
        .name("duplicity-reader".into())
        .spawn(move || {
            let catalog = match Catalog::discover(
                source.as_ref(),
                &prefix,
                NameConfig::default(),
                ReaderLimits::default(),
                &discovery_cancel,
            ) {
                Ok(catalog) => catalog,
                Err(error) => {
                    let _ = responses.send(Reply::Error(format!("Discovery failed: {error:?}")));
                    return;
                }
            };
            let mut history = Vec::new();
            let mut charged = 0usize;
            for (index, object, set) in catalog.manifests() {
                let charge = object
                    .key
                    .len()
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(256 + std::mem::size_of::<History>()));
                charged = match charge.and_then(|n| charged.checked_add(n)) {
                    Some(n) if n <= 32 * 1024 * 1024 && history.len() < 100_000 => n,
                    _ => {
                        let _ =
                            responses.send(Reply::Error("History display limit exceeded.".into()));
                        return;
                    }
                };
                if discovery_cancel.is_cancelled() {
                    return;
                }
                if history.try_reserve_exact(1).is_err() {
                    let _ = responses.send(Reply::Error("History allocation failed.".into()));
                    return;
                }
                history.push(History {
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
            let total = catalog
                .objects()
                .iter()
                .try_fold(0u64, |sum, object| sum.checked_add(object.object().size));
            if responses.send(Reply::Ready(history, total)).is_err() {
                return;
            }
            while let Ok(scan) = inbox.recv() {
                let reply = match catalog.read_snapshot(scan.anchor, scan.target, &scan.cancel) {
                    Ok(snapshot) => Reply::Snapshot(snapshot),
                    Err(error) => Reply::Error(format!("Snapshot failed: {error:?}")),
                };
                if responses.send(reply).is_err() {
                    return;
                }
            }
        })?;
    let mut history: Vec<History> = Vec::new();
    let mut snapshot: Option<ValidatedSnapshot> = None;
    let mut selected = 0usize;
    let mut anchor = None;
    let mut total = None;
    let mut busy = true;
    let mut confirm = false;
    let mut cancel = initial;
    let mut status = "Discovering object metadata...".to_string();
    let result = ratatui::run(|terminal| -> io::Result<()> {
        loop {
            match replies.try_recv() {
                Ok(reply) => {
                    busy = false;
                    confirm = false;
                    selected = 0;
                    match reply {
                        Reply::Ready(rows, bytes) => {
                            history = rows;
                            total = bytes;
                            status = "Metadata only. Choose a full set with a, then a target date with Enter.".into();
                        }
                        Reply::Snapshot(value) => {
                            status = format!("Validated plain/gzip snapshot: {} entries, {} payload bytes. Filesystem restoration is under construction.", value.state().len(), value.state().payload_bytes());
                            snapshot = Some(value);
                        }
                        Reply::Error(error) => status = display::bytes(error.as_bytes()),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) if busy => {
                    busy = false;
                    status = "Reader worker stopped.".into();
                }
                _ => {}
            }
            let count = snapshot.as_ref().map_or(history.len(), |s| s.state().len());
            terminal.draw(|frame| {
                let areas = Layout::vertical([Constraint::Length(3), Constraint::Min(2), Constraint::Length(5)]).split(frame.area());
                frame.render_widget(Paragraph::new(format!("DUPLICITY / BACKUP READER   full anchor: {anchor:?}"))
                    .style(Style::default().fg(Color::Cyan)).block(Block::bordered()), areas[0]);
                let height = usize::from(areas[1].height.saturating_sub(2)).max(1);
                let offset = selected.saturating_sub(height / 2).min(count.saturating_sub(height));
                let items: Vec<ListItem> = if let Some(value) = &snapshot {
                    value.state().entries().skip(offset).take(height).map(|(path, entry)|
                        ListItem::new(format!("{:?} {:03o} {:>10}  {}", entry.entry_type(), entry.mode() & 0o777,
                            entry.content().map_or(0, <[u8]>::len), display::path(path)))).collect()
                } else { history.iter().skip(offset).take(height).map(|row| ListItem::new(row.label.as_str())).collect() };
                let mut cursor = ListState::default().with_selected((count != 0).then_some(selected - offset));
                frame.render_stateful_widget(List::new(items).block(Block::bordered().title(
                    if snapshot.is_some() { "Snapshot paths" } else { "History / metadata only" }))
                    .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White)), areas[1], &mut cursor);
                frame.render_widget(Paragraph::new(format!("{status}\nUp/Down navigate | a anchor | Enter scan/confirm | Esc history/cancel | q quit"))
                    .wrap(Wrap { trim: false }).block(Block::bordered()), areas[2]);
            })?;
            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Esc if busy => {
                    cancel.cancel();
                    status = "Cancellation requested; waiting for bounded work.".into();
                }
                KeyCode::Esc => {
                    snapshot = None;
                    confirm = false;
                    selected = 0;
                    status = "Metadata history. Choose anchor and exact target date.".into();
                }
                KeyCode::Down => {
                    selected = selected.saturating_add(1).min(count.saturating_sub(1));
                    confirm = false;
                }
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                    confirm = false;
                }
                KeyCode::Char('a') if !busy && snapshot.is_none() => {
                    if let Some(row) = history.get(selected) {
                        if row.kind == SetKind::Full {
                            anchor = Some(row.index);
                            confirm = false;
                            status = format!(
                                "Full anchor {} selected. Choose target date, then Enter.",
                                row.index
                            );
                        } else {
                            status = "Anchor must be a full set.".into();
                        }
                    }
                }
                //= docs/parsing-requirements.md#parse-browse-002
                //= type=implementation
                //# When signature metadata cannot provide a requested listing, the reader MUST expose that a volume scan is required before starting that scan.
                KeyCode::Enter if !busy && snapshot.is_none() => {
                    if let (Some(full), Some(row)) = (anchor, history.get(selected)) {
                        if !confirm {
                            let bytes = total.map_or("unknown".into(), |n| format!("{n} bytes"));
                            status = format!("A full volume scan is required. Listed objects total {bytes}; exact download size is unknown. Enter confirms; Esc cancels.");
                            confirm = true;
                        } else {
                            cancel = Cancellation::default();
                            requests
                                .try_send(Scan {
                                    anchor: full,
                                    target: row.end,
                                    cancel: cancel.clone(),
                                })
                                .map_err(|_| io::Error::other("reader worker unavailable"))?;
                            busy = true;
                            confirm = false;
                            status = "Reading and validating all required volumes... Esc cancels."
                                .into();
                        }
                    } else {
                        status = "Choose a full set with a first.".into();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    });
    cancel.cancel();
    result
}

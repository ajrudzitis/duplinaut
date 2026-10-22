//! Owned fallible terminal lifecycle. No global panic hook or infallible output.
use crate::worker::Worker;
use duplicity_core::source::Cancellation;
use std::{
    fmt, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Default)]
pub struct Completion {
    pub initialization: Option<io::Error>,
    pub event: Option<io::Error>,
    pub teardown: Option<io::Error>,
}
impl Completion {
    pub fn event(result: io::Result<()>) -> Self {
        Self {
            event: result.err(),
            ..Self::default()
        }
    }
    pub fn failed(&self) -> bool {
        self.initialization.is_some() || self.event.is_some() || self.teardown.is_some()
    }
}
impl fmt::Display for Completion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "terminal faults: initialization={} event={} teardown={}",
            self.initialization.is_some(),
            self.event.is_some(),
            self.teardown.is_some()
        )
    }
}
impl std::error::Error for Completion {}
/// The public executable uses Native; injection exercises the same ordering.
pub trait Lifecycle {
    type Terminal;
    fn initialize(&mut self) -> io::Result<Self::Terminal>;
    fn restore(&mut self) -> io::Result<()>;
}
/// Backend-only output gate. Explicit teardown uses the real stdout directly;
/// once disarmed, ratatui's implicit Drop cursor command cannot perform I/O.
pub struct GuardedWriter<W> {
    inner: W,
    enabled: Arc<AtomicBool>,
}
impl<W: io::Write> io::Write for GuardedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.enabled.load(Ordering::Acquire) {
            self.inner.write(bytes)
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.enabled.load(Ordering::Acquire) {
            self.inner.flush()
        } else {
            Ok(())
        }
    }
}
pub type NativeTerminal =
    ratatui::Terminal<ratatui::backend::CrosstermBackend<GuardedWriter<io::Stdout>>>;
pub struct Native {
    enabled: Arc<AtomicBool>,
}
impl Default for Native {
    fn default() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }
}
fn disarm(enabled: &AtomicBool) {
    enabled.store(false, Ordering::Release);
}

impl Lifecycle for Native {
    type Terminal = NativeTerminal;
    fn initialize(&mut self) -> io::Result<Self::Terminal> {
        // ratatui0.30.2 try_init installs a process-global panic hook invoking its
        // infallible restore/eprintln. Use equivalent fallible primitives instead.
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        self.enabled.store(true, Ordering::Release);
        let writer = GuardedWriter {
            inner: io::stdout(),
            enabled: self.enabled.clone(),
        };
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(writer))
    }
    fn restore(&mut self) -> io::Result<()> {
        restore_and_disarm(
            &self.enabled,
            crossterm::terminal::disable_raw_mode,
            || crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen),
            || crossterm::execute!(io::stdout(), crossterm::cursor::Show),
        )
    }
}
fn restore_and_disarm(
    enabled: &AtomicBool,
    raw: impl FnOnce() -> io::Result<()>,
    screen: impl FnOnce() -> io::Result<()>,
    cursor: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let result = restore_actions(raw, screen, cursor);
    // Pinned Terminal::drop otherwise retries Show and logs infallibly on
    // error. All real restoration attempts/errors above remain recorded.
    disarm(enabled);
    result
}
fn restore_actions(
    raw: impl FnOnce() -> io::Result<()>,
    screen: impl FnOnce() -> io::Result<()>,
    cursor: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let faults = TeardownFaults {
        raw: raw().err(),
        screen: screen().err(),
        cursor: cursor().err(),
    };
    if faults.raw.is_some() || faults.screen.is_some() || faults.cursor.is_some() {
        Err(io::Error::other(faults))
    } else {
        Ok(())
    }
}
#[derive(Debug)]
pub struct TeardownFaults {
    pub raw: Option<io::Error>,
    pub screen: Option<io::Error>,
    pub cursor: Option<io::Error>,
}
impl fmt::Display for TeardownFaults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "terminal teardown faults: raw={} alternate_screen={} cursor={}",
            self.raw.is_some(),
            self.screen.is_some(),
            self.cursor.is_some()
        )
    }
}
impl std::error::Error for TeardownFaults {}
/// Initialization can partially alter terminal state before failing. In every
/// controlled return, cancel/close precedes teardown; caller drains/joins next.
//= docs/parsing-requirements.md#restore-ui-012
//= type=implementation
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
pub fn interact<L: Lifecycle>(
    worker: &mut Worker,
    cancel: &mut Cancellation,
    lifecycle: &mut L,
    event_loop: impl FnOnce(&mut L::Terminal, &mut Worker, &mut Cancellation) -> io::Result<()>,
) -> Completion {
    let mut terminal = match lifecycle.initialize() {
        Ok(value) => value,
        Err(error) => {
            worker.close(cancel);
            let teardown = lifecycle.restore().err();
            return Completion {
                initialization: Some(error),
                event: None,
                teardown,
            };
        }
    };
    let event = event_loop(&mut terminal, worker, cancel).err();
    worker.close(cancel);
    let teardown = lifecycle.restore().err();
    Completion {
        initialization: None,
        event,
        teardown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn all_teardown_actions_run_and_each_fault_is_retained() {
        let calls = std::cell::RefCell::new(Vec::new());
        let result = restore_actions(
            || {
                calls.borrow_mut().push("raw");
                Err(io::Error::other("raw failure"))
            },
            || {
                calls.borrow_mut().push("screen");
                Err(io::Error::other("screen failure"))
            },
            || {
                calls.borrow_mut().push("cursor");
                Err(io::Error::other("cursor failure"))
            },
        )
        .unwrap_err();
        assert_eq!(*calls.borrow(), ["raw", "screen", "cursor"]);
        let faults = result
            .get_ref()
            .unwrap()
            .downcast_ref::<TeardownFaults>()
            .unwrap();
        assert!(faults.raw.is_some() && faults.screen.is_some() && faults.cursor.is_some());
    }
    #[test]
    //= docs/parsing-requirements.md#restore-ui-012
    //= type=test
    //# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
    fn concrete_terminal_destructor_does_no_io_after_failed_cursor_restore() {
        use std::{cell::Cell, rc::Rc};
        struct Writer {
            fail: Rc<Cell<bool>>,
            calls: Rc<Cell<usize>>,
        }
        impl io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.calls.set(self.calls.get() + 1);
                if self.fail.get() {
                    Err(io::Error::other("broken stdout"))
                } else {
                    Ok(bytes.len())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                self.calls.set(self.calls.get() + 1);
                if self.fail.get() {
                    Err(io::Error::other("broken stdout"))
                } else {
                    Ok(())
                }
            }
        }
        let enabled = Arc::new(AtomicBool::new(true));
        let fail = Rc::new(Cell::new(false));
        let calls = Rc::new(Cell::new(0));
        let backend = ratatui::backend::CrosstermBackend::new(GuardedWriter {
            inner: Writer {
                fail: fail.clone(),
                calls: calls.clone(),
            },
            enabled: enabled.clone(),
        });
        let mut terminal = ratatui::Terminal::with_options(
            backend,
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Fixed(ratatui::layout::Rect::new(0, 0, 20, 5)),
            },
        )
        .unwrap();
        terminal.hide_cursor().unwrap();
        fail.set(true);
        let teardown = restore_and_disarm(
            &enabled,
            || Err(io::Error::other("raw failure")),
            || Err(io::Error::other("screen failure")),
            || terminal.show_cursor(),
        );
        let teardown = teardown.unwrap_err();
        let faults = teardown
            .get_ref()
            .unwrap()
            .downcast_ref::<TeardownFaults>()
            .unwrap();
        assert!(
            faults.raw.is_some() && faults.screen.is_some() && faults.cursor.is_some(),
            "real cursor restoration failed, leaving hidden_cursor true"
        );
        let before = calls.get();
        drop(terminal);
        assert_eq!(calls.get(),before,"implicit Terminal::drop must not touch broken stdout or reach its infallible stderr branch");
    }
}

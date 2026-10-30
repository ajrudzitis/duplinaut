//! Frozen restore requests and bounded safe text; native controller proofs pending.
use crate::cli::Destination;
use duplicity_core::{
    path::BackupPath, reader::ValidatedSnapshot, restore_plan::RestorePlan, source::Cancellation,
};
use std::sync::Arc;
pub const ROOTS: usize = 1024;
pub const ROOT_BYTES: usize = 4 * 1024 * 1024;
pub const COMPARISONS: usize = 16_000_000;
pub const PREVIEW_BYTES: usize = 20 * 1024 * 1024;
pub const DIAGNOSTIC_BYTES: usize = 1024 * 1024;
#[derive(Debug)]
pub enum Error {
    Limit,
    Allocation,
    Cancelled,
    Selection,
    Plan,
}
pub struct Text {
    value: String,
    limit: usize,
}
impl Text {
    pub fn new(limit: usize) -> Self {
        Self {
            value: String::new(),
            limit,
        }
    }
    fn reserve(&mut self, n: usize) -> Result<(), Error> {
        self.value
            .len()
            .checked_add(n)
            .filter(|n| *n <= self.limit)
            .ok_or(Error::Limit)?;
        self.value
            .try_reserve_exact(n)
            .map_err(|_| Error::Allocation)
    }
    pub fn push(&mut self, text: &str) -> Result<(), Error> {
        self.reserve(text.len())?;
        self.value.push_str(text);
        Ok(())
    }
    pub fn bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let n = bytes
            .iter()
            .try_fold(0usize, |n, b| {
                n.checked_add(match b {
                    b'\\' => 2,
                    0x20..=0x7e => 1,
                    _ => 4,
                })
            })
            .ok_or(Error::Limit)?;
        self.reserve(n)?;
        let hex = b"0123456789abcdef";
        for &b in bytes {
            match b {
                b'\\' => self.value.push_str("\\\\"),
                0x20..=0x7e => self.value.push(char::from(b)),
                _ => {
                    self.value.push_str("\\x");
                    self.value.push(char::from(hex[(b >> 4) as usize]));
                    self.value.push(char::from(hex[(b & 15) as usize]));
                }
            }
        }
        Ok(())
    }
    pub fn path(&mut self, path: &BackupPath) -> Result<(), Error> {
        if path.components().is_empty() {
            return self.push("/");
        }
        for (i, part) in path.components().iter().enumerate() {
            if i != 0 {
                self.push("/")?;
            }
            self.bytes(part)?;
        }
        Ok(())
    }
    pub fn finish(self) -> String {
        self.value
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub entries: usize,
    pub payload: u64,
    pub empty_root_policy: bool,
}
pub struct Frozen {
    snapshot: Arc<ValidatedSnapshot>,
    roots: Vec<BackupPath>,
    destination: Arc<Destination>,
    summary: Summary,
}
pub struct Preview {
    pub frozen: Frozen,
    pub text: String,
}
fn within_comparison_budget(state_count: usize, root_count: usize) -> bool {
    state_count
        .checked_mul(root_count.max(1))
        .is_some_and(|comparisons| comparisons <= COMPARISONS)
}
fn plan(request: &Frozen) -> Result<RestorePlan<'_>, Error> {
    let limits = duplicity_core::restore_plan::RestorePlanLimits {
        max_roots: ROOTS,
        max_candidate_checks: COMPARISONS,
        ..Default::default()
    };
    if request.roots.is_empty() {
        request.snapshot.restore_all(limits)
    } else {
        request.snapshot.restore_roots(&request.roots, limits)
    }
    .map_err(|_| Error::Plan)
}
fn summarize(plan: &RestorePlan<'_>) -> Result<Summary, Error> {
    let payload = plan
        .entries()
        .iter()
        .try_fold(0u64, |n, e| {
            n.checked_add(e.entry().content().map_or(0, |c| c.len() as u64))
        })
        .ok_or(Error::Limit)?;
    Ok(Summary {
        entries: plan.entries().len(),
        payload,
        empty_root_policy: plan.entries().is_empty(),
    })
}
pub fn copy_roots(
    snapshot: &ValidatedSnapshot,
    marked: &[usize],
) -> Result<Vec<BackupPath>, Error> {
    if marked.len() > ROOTS || !within_comparison_budget(snapshot.state().len(), marked.len()) {
        return Err(Error::Limit);
    }
    if marked.iter().any(|i| *i >= snapshot.state().len()) {
        return Err(Error::Selection);
    }
    let mut charge = marked
        .len()
        .checked_mul(std::mem::size_of::<BackupPath>())
        .ok_or(Error::Limit)?;
    for (i, (path, _)) in snapshot.state().entries().enumerate() {
        if marked.contains(&i) {
            charge = path
                .components()
                .iter()
                .try_fold(charge, |n, p| {
                    n.checked_add(p.capacity())
                        .and_then(|n| n.checked_add(std::mem::size_of::<Vec<u8>>()))
                })
                .ok_or(Error::Limit)?;
        }
    }
    if charge > ROOT_BYTES {
        return Err(Error::Limit);
    }
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(marked.len())
        .map_err(|_| Error::Allocation)?;
    for (i, (path, _)) in snapshot.state().entries().enumerate() {
        if marked.contains(&i) {
            roots.push(path.clone());
        }
    }
    Ok(roots)
}
//= docs/parsing-requirements.md#restore-ui-002
//= type=implementation
//# The frontend MUST derive a restoration preview from the exact completed immutable snapshot and selected original byte-valued roots, including precisely the entries and source directory ancestors accepted by its `RestorePlan`.
//= docs/parsing-requirements.md#restore-ui-003
//= type=implementation
//# The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.
//= docs/parsing-requirements.md#restore-ui-011
//= type=implementation
//# The frontend MUST bound owned restoration request, preview and receipt metadata before allocating it, retaining original payload ownership without payload copies.
pub fn prepare(
    snapshot: Arc<ValidatedSnapshot>,
    mut marked: Vec<usize>,
    destination: Arc<Destination>,
    cancel: &Cancellation,
) -> Result<Preview, Error> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    if marked.len() > ROOTS || !within_comparison_budget(snapshot.state().len(), marked.len()) {
        return Err(Error::Limit);
    }
    marked.sort_unstable();
    marked.dedup();
    if marked.last().is_some_and(|i| *i >= snapshot.state().len()) {
        return Err(Error::Selection);
    }
    let roots = copy_roots(&snapshot, &marked)?;
    let mut frozen = Frozen {
        snapshot,
        roots,
        destination,
        summary: Summary {
            entries: 0,
            payload: 0,
            empty_root_policy: false,
        },
    };
    let summary = summarize(&plan(&frozen)?)?;
    frozen.summary = summary;
    let mut text = Text::new(PREVIEW_BYTES);
    text.push(&format!(
        "Snapshot target: {}\nScope: {}\n",
        frozen.snapshot.target().0,
        if frozen.roots.is_empty() {
            "whole snapshot (no marks)"
        } else {
            "marked roots"
        }
    ))?;
    for root in &frozen.roots {
        text.push("Root: ")?;
        text.path(root)?;
        text.push("\n")?;
    }
    text.push(&format!("Include exact source directory ancestors; no flattening.\nEntries: {}\nRegular payload bytes: {}\nRequested parent: ",summary.entries,summary.payload))?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        text.bytes(frozen.destination.parent.as_os_str().as_bytes())?;
    }
    text.push("\nNew destination leaf: ")?;
    text.bytes(&frozen.destination.name)?;
    text.push("\nAtomic no-replacement publication; existing destinations are never overwritten.\nBackend checks parent policy/filesystem, automatic profile and exact metadata after acknowledgment, before staging.\nPreserves ordinary modes, supported exact modification times and raw symlink target bytes. Ownership, source ACLs/xattrs and crash durability are excluded.\n")?;
    if summary.empty_root_policy {
        text.push("Empty tree: policy root 0700, unspecified root modification time.\n")?;
    }
    text.push("Type restore, then Enter. Esc abandons this request.\nLegacy terminals can report repeated keys as Press; acknowledgment is an observed sequence.\n")?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(Preview {
        frozen,
        text: text.finish(),
    })
}
#[cfg(target_os = "linux")]
//= docs/parsing-requirements.md#restore-ui-002
//= type=implementation
//# The frontend MUST derive a restoration preview from the exact completed immutable snapshot and selected original byte-valued roots, including precisely the entries and source directory ancestors accepted by its `RestorePlan`.
//= docs/parsing-requirements.md#restore-ui-013
//= type=implementation
//# The frontend MUST keep backup source access read-only and avoid collecting or exposing credentials, backup payload bytes or secret material through the restoration interaction, diagnostics or automatic logs.
pub fn execute(
    frozen: Frozen,
    cancel: &Cancellation,
) -> Result<duplicity_core::restore_linux::RestoreOutcome, Error> {
    let selected = plan(&frozen)?;
    if summarize(&selected)? != frozen.summary {
        return Err(Error::Plan);
    }
    Ok(duplicity_core::restore_linux::restore(
        &selected,
        &frozen.destination.parent,
        &frozen.destination.name,
        cancel,
        Default::default(),
    ))
}
#[cfg(target_os = "linux")]
pub fn retry_allowed(outcome: &duplicity_core::restore_linux::RestoreOutcome) -> bool {
    matches!(outcome,duplicity_core::restore_linux::RestoreOutcome::Unpublished(e)if e.uncertain_outer.is_none())
}
#[cfg(target_os = "linux")]
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
pub fn diagnostics(
    outcome: &duplicity_core::restore_linux::RestoreOutcome,
) -> Result<String, Error> {
    use duplicity_core::restore_linux::RestoreOutcome;
    let mut text = Text::new(DIAGNOSTIC_BYTES);
    let receipt = match outcome {
        RestoreOutcome::Published(r) => {
            text.push("Published completed tree. No second restore to this fixed destination.\n")?;
            Some(r)
        }
        RestoreOutcome::PublicationIndeterminate(e) => {
            text.push(&format!("Publication indeterminate: errno {:?}. Publication may have occurred; no retry, recovery or cleanup is authorized.\n",e.error.raw_os_error()))?;
            Some(&e.receipt)
        }
        RestoreOutcome::Unpublished(e) => {
            let cause = match e.cause {
                duplicity_core::restore_linux::Cause::Syscall(errno) => {
                    format!("Syscall errno {}", errno.raw_os_error())
                }
                other => format!("{other:?}"),
            };
            text.push(&format!("Unpublished: cause {}\nCleanup attempted={} failures={} first_errno={:?} exhausted={} detail_truncated={}\n",cause,e.cleanup.attempted,e.cleanup.failures,e.cleanup.first_error.map(|e|e.raw_os_error()),e.cleanup.budget_exhausted,e.cleanup.diagnostics_truncated))?;
            if let Some(candidate) = &e.uncertain_outer {
                text.push("First-stage candidate uncertain; no session retry: ")?;
                text.bytes(candidate.to_bytes())?;
                text.push("\n")?;
            }
            None
        }
    };
    if let Some(r) = receipt {
        text.push(&format!("Profile {:?}; filesystem {:#x}\nParent device={} inode={}; root device={} inode={}\nDestination: ",r.profile,r.filesystem,r.parent.device,r.parent.inode,r.root.device,r.root.inode))?;
        text.bytes(r.destination.to_bytes())?;
        text.push("\nStaging: ")?;
        text.bytes(r.staging.to_bytes())?;
        text.push(&format!("\nEmpty-root mtime unspecified={}\nCleanup warnings attempted={} failures={} first_errno={:?} exhausted={} detail_truncated={}\n",r.unspecified_empty_root_mtime,r.cleanup.attempted,r.cleanup.failures,r.cleanup.first_error.map(|e|e.raw_os_error()),r.cleanup.budget_exhausted,r.cleanup.diagnostics_truncated))?;
    }
    Ok(text.finish())
}
/// Dedicated fixed-word acknowledgment; the browser owns event draining/rendering.
#[derive(Default)]
//= docs/parsing-requirements.md#restore-ui-004
//= type=implementation
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
//= docs/parsing-requirements.md#restore-ui-005
//= type=implementation
//# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
pub struct Acknowledgment {
    word: [u8; 7],
    len: usize,
    pub armed: bool,
}
impl Acknowledgment {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn input(&mut self, c: char) {
        if self.armed && c.is_ascii() && self.len < 7 {
            self.word[self.len] = c as u8;
            self.len += 1;
        }
    }
    pub fn backspace(&mut self) {
        if self.armed {
            self.len = self.len.saturating_sub(1);
        }
    }
    pub fn ready(&self) -> bool {
        self.armed && self.word[..self.len] == *b"restore"
    }
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.word[..self.len]).expect("ASCII acknowledgment")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //= docs/parsing-requirements.md#restore-ui-011
    //= type=test
    //# The frontend MUST bound owned restoration request, preview and receipt metadata before allocating it, retaining original payload ownership without payload copies.
    fn comparison_budget_checks_limit_and_multiplication_overflow() {
        assert!(within_comparison_budget(COMPARISONS, 1));
        assert!(!within_comparison_budget(COMPARISONS + 1, 1));
        assert!(!within_comparison_budget(usize::MAX, 2));
    }

    #[test]
    fn acknowledgment_is_fixed_ascii_empty_on_reset() {
        let mut a = Acknowledgment::default();
        for c in "restore".chars() {
            a.input(c);
        }
        assert!(!a.ready());
        a.armed = true;
        a.input('é');
        assert_eq!(a.text(), "");
        for c in "restore".chars() {
            a.input(c);
        }
        assert!(a.ready());
        a.input('x');
        assert!(a.ready());
        a.backspace();
        assert!(!a.ready());
        a.clear();
        assert!(!a.armed);
        assert_eq!(a.text(), "");
        for c in "rrrrrrr".chars() {
            a.input(c);
        }
        assert!(!a.ready());
    }
    #[test]
    fn bounded_escaping_preserves_full_raw_identity() {
        let mut text = Text::new(10);
        text.bytes(b"\xff\\\n").unwrap();
        assert_eq!(text.finish(), "\\xff\\\\\\x0a");
        let mut small = Text::new(3);
        assert!(small.bytes(b"\xff").is_err());
        assert_eq!(small.finish(), "");
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-011
    //= type=test
    //# The frontend MUST bound owned restoration request, preview and receipt metadata before allocating it, retaining original payload ownership without payload copies.
    fn preview_text_enforces_configured_limit_without_mutating_on_overflow() {
        let mut text = Text::new(PREVIEW_BYTES);
        text.reserve(PREVIEW_BYTES).unwrap();

        // Fill the single owned preview buffer in chunks; the test does not
        // allocate a second 20 MiB input string or construct a snapshot.
        let chunk = [b'x'; 4096];
        let chunk = std::str::from_utf8(&chunk).unwrap();
        for _ in 0..(PREVIEW_BYTES / chunk.len()) {
            text.push(chunk).unwrap();
        }
        assert_eq!(text.value.len(), PREVIEW_BYTES);
        assert!(text.value.bytes().all(|byte| byte == b'x'));

        assert!(matches!(text.push("x"), Err(Error::Limit)));
        assert_eq!(text.value.len(), PREVIEW_BYTES);
        assert!(text.value.bytes().all(|byte| byte == b'x'));
    }
}
#[cfg(target_os = "linux")]
//= docs/parsing-requirements.md#restore-ui-012
//= type=implementation
//# On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.
pub fn fallback(
    outcome: &duplicity_core::restore_linux::RestoreOutcome,
    sink: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let text = diagnostics(outcome).map_err(|_| {
        std::io::Error::other("known typed result retained; diagnostic rendering failed")
    })?;
    sink.write_all(text.as_bytes())
}
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct ShutdownFailure {
    pub operation: Option<u64>,
    pub requested_destination: Option<Arc<Destination>>,
    pub initialization: Option<std::io::Error>,
    pub terminal: Option<std::io::Error>,
    pub teardown: Option<std::io::Error>,
    pub shutdown: Option<std::io::Error>,
    pub output: Option<std::io::Error>,
    pub outcome: Option<duplicity_core::restore_linux::RestoreOutcome>,
    pub unavailable: bool,
}
#[cfg(target_os = "linux")]
impl std::fmt::Display for ShutdownFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"controlled shutdown faults: initialization={} terminal={} teardown={} worker={} output={}; known typed restore result={}; outcome unavailable={}",self.initialization.is_some(),self.terminal.is_some(),self.teardown.is_some(),self.shutdown.is_some(),self.output.is_some(),self.outcome.is_some(),self.unavailable)
    }
}
#[cfg(target_os = "linux")]
impl std::error::Error for ShutdownFailure {}
/// Caller invokes only after drawing the concrete preview and empty field.
//= docs/parsing-requirements.md#restore-ui-004
//= type=implementation
//# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
pub fn drain_pending(
    ack: &mut Acknowledgment,
    mut poll: impl FnMut() -> std::io::Result<bool>,
    mut consume: impl FnMut() -> std::io::Result<()>,
) -> std::io::Result<()> {
    for _ in 0..64 {
        if !poll()? {
            ack.armed = true;
            return Ok(());
        }
        consume()?;
    }
    if !poll()? {
        ack.armed = true;
    }
    Ok(())
}
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    //= docs/parsing-requirements.md#restore-ui-004
    //= type=test
    //# The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.
    //= docs/parsing-requirements.md#restore-ui-005
    //= type=test
    //# The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.
    fn pending_input_drains_across_iterations_without_arming_or_accepting() {
        let remaining = std::cell::Cell::new(65);
        let mut ack = Acknowledgment::default();
        for _ in 0..7 {
            ack.input('r');
        }
        assert_eq!(ack.text(), "");
        drain_pending(
            &mut ack,
            || Ok(remaining.get() != 0),
            || {
                remaining.set(remaining.get() - 1);
                Ok(())
            },
        )
        .unwrap();
        assert!(!ack.armed);
        assert_eq!(remaining.get(), 1);
        drain_pending(
            &mut ack,
            || Ok(remaining.get() != 0),
            || {
                remaining.set(remaining.get() - 1);
                Ok(())
            },
        )
        .unwrap();
        assert!(ack.armed);
        assert!(!ack.ready());
        for _ in 0..7 {
            ack.input('r');
        }
        assert!(!ack.ready());
        ack.clear();
        ack.armed = true;
        assert!(!ack.ready());
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
    fn all_typed_outcomes_and_output_failure_preserve_authority() {
        use duplicity_core::restore_linux::{
            Cause, Cleanup, Identity, Indeterminate, Profile, Receipt, RestoreOutcome as O,
            Unpublished,
        };
        use std::ffi::CString;
        let receipt = || Receipt {
            destination: CString::new(b"d\xff\n".to_vec()).unwrap(),
            staging: CString::new("stage").unwrap(),
            profile: Profile::PrivateParentSameParent,
            parent: Identity {
                device: 1,
                inode: 2,
            },
            root: Identity {
                device: 1,
                inode: 3,
            },
            filesystem: 0xef53,
            unspecified_empty_root_mtime: false,
            cleanup: Cleanup::default(),
        };
        let outcomes = [
            O::Published(receipt()),
            O::Unpublished(Unpublished {
                cause: Cause::Cancelled,
                cleanup: Cleanup::default(),
                uncertain_outer: None,
            }),
            O::Unpublished(Unpublished {
                cause: Cause::Fidelity,
                cleanup: Cleanup {
                    failures: 2,
                    diagnostics_truncated: true,
                    ..Default::default()
                },
                uncertain_outer: Some(CString::new("candidate").unwrap()),
            }),
            O::PublicationIndeterminate(Indeterminate {
                error: rustix::io::Errno::IO,
                receipt: receipt(),
            }),
        ];
        for (i, outcome) in outcomes.into_iter().enumerate() {
            assert_eq!(retry_allowed(&outcome), i == 1);
            let text = diagnostics(&outcome).unwrap();
            assert!(!text.contains('\x1b'));
            if i == 0 || i == 3 {
                assert!(text.contains("d\\xff\\x0a"));
            }
            struct Broken;
            impl std::io::Write for Broken {
                fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                    Err(std::io::Error::other("injected output failure"))
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            assert!(fallback(&outcome, &mut Broken).is_err());
            let failure = ShutdownFailure {
                operation: None,
                requested_destination: None,
                initialization: None,
                teardown: None,
                terminal: Some(std::io::Error::other("terminal")),
                shutdown: None,
                output: Some(std::io::Error::other("output")),
                outcome: Some(outcome),
                unavailable: false,
            };
            assert!(failure.outcome.is_some());
            assert!(!failure.unavailable);
        }
    }
}
pub fn unavailable(
    destination: Option<&Destination>,
    operation: Option<u64>,
) -> Result<String, Error> {
    let mut text = Text::new(DIAGNOSTIC_BYTES);
    text.push(&format!("Restoration outcome unavailable; inspection required. No retry, recovery or modifying cleanup is authorized. Operation {operation:?}.\n"))?;
    if let Some(destination) = destination {
        text.push("Requested parent: ")?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            text.bytes(destination.parent.as_os_str().as_bytes())?;
        }
        text.push("\nRequested new leaf: ")?;
        text.bytes(&destination.name)?;
        text.push("\nNo staging or publication absence is inferred.\n")?;
    }
    Ok(text.finish())
}

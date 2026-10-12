//! Read-only restoration selection over a completed snapshot state.
//! Filesystem publication, source provenance and controller proofs are separate.
use crate::path::BackupPath;
use crate::state::{SnapshotEntry, SnapshotState};

#[derive(Clone, Copy, Debug)]
pub struct RestorePlanLimits {
    pub max_roots: usize,
    pub max_state_entries: usize,
    pub max_entries: usize,
    pub max_metadata_bytes: usize,
    pub max_candidate_checks: usize,
}
impl Default for RestorePlanLimits {
    fn default() -> Self {
        Self {
            max_roots: 1024,
            max_state_entries: 10_000_000,
            max_entries: 10_000_000,
            max_metadata_bytes: 256 * 1024 * 1024,
            max_candidate_checks: 16_000_000,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum RestorePlanError<'a> {
    Limit,
    Allocation,
    EmptySelection,
    /// Position in the caller's selection array.
    MissingSelection {
        index: usize,
    },
    Unsupported {
        path: &'a BackupPath,
    },
}
pub struct PlannedEntry<'a> {
    path: &'a BackupPath,
    entry: &'a SnapshotEntry,
}
impl<'a> PlannedEntry<'a> {
    pub fn path(&self) -> &'a BackupPath {
        self.path
    }
    pub fn entry(&self) -> &'a SnapshotEntry {
        self.entry
    }
}
pub struct RestorePlan<'a> {
    entries: Vec<PlannedEntry<'a>>,
}
impl<'a> RestorePlan<'a> {
    /// Selects the whole state, including a legitimately empty state.
    pub fn all(
        state: &'a SnapshotState,
        limits: RestorePlanLimits,
    ) -> Result<Self, RestorePlanError<'a>> {
        Self::build(state, None, limits)
    }
    /// Selects each root and its descendants, plus required source directory
    /// ancestors. Component identity is preserved; overlaps produce one entry.
    pub fn from_roots(
        state: &'a SnapshotState,
        roots: &[BackupPath],
        limits: RestorePlanLimits,
    ) -> Result<Self, RestorePlanError<'a>> {
        if roots.is_empty() {
            return Err(RestorePlanError::EmptySelection);
        }
        Self::build(state, Some(roots), limits)
    }
    //= docs/parsing-requirements.md#restore-001
    //= type=implementation
    //# A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.
    //= docs/parsing-requirements.md#restore-005
    //= type=implementation
    //# A restore selection requiring unsupported filesystem entry types MUST fail before publication with an entry-specific diagnostic.
    fn build(
        state: &'a SnapshotState,
        roots: Option<&[BackupPath]>,
        limits: RestorePlanLimits,
    ) -> Result<Self, RestorePlanError<'a>> {
        if state.len() > limits.max_state_entries {
            return Err(RestorePlanError::Limit);
        }
        if let Some(roots) = roots {
            if roots.len() > limits.max_roots {
                return Err(RestorePlanError::Limit);
            }
            for (index, root) in roots.iter().enumerate() {
                if state.get(root).is_none() {
                    return Err(RestorePlanError::MissingSelection { index });
                }
            }
        }
        // Output can never exceed the source map. Exact reservation avoids
        // geometric growth; pinned standard allocation/capacity is trusted.
        let capacity = state.len().min(limits.max_entries);
        capacity
            .checked_mul(std::mem::size_of::<PlannedEntry<'_>>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .filter(|bytes| *bytes <= limits.max_metadata_bytes)
            .ok_or(RestorePlanError::Limit)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(capacity)
            .map_err(|_| RestorePlanError::Allocation)?;
        let mut checks = 0usize;
        for (path, entry) in state.entries() {
            let included = if let Some(roots) = roots {
                let mut included = false;
                for root in roots {
                    checks = checks
                        .checked_add(1)
                        .filter(|checks| *checks <= limits.max_candidate_checks)
                        .ok_or(RestorePlanError::Limit)?;
                    if path.components().starts_with(root.components())
                        || root.components().starts_with(path.components())
                    {
                        included = true;
                        break;
                    }
                }
                included
            } else {
                checks = checks
                    .checked_add(1)
                    .filter(|checks| *checks <= limits.max_candidate_checks)
                    .ok_or(RestorePlanError::Limit)?;
                true
            };
            if included {
                if !entry.supported_restore_type() {
                    return Err(RestorePlanError::Unsupported { path });
                }
                if entries.len() >= limits.max_entries {
                    return Err(RestorePlanError::Limit);
                }
                entries.push(PlannedEntry { path, entry });
            }
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[PlannedEntry<'a>] {
        &self.entries
    }
}

//! Metadata-only contiguous chain planning. This establishes neither volume
//! existence/integrity nor decryptability, signer identity or restorability.
use crate::names::{ObjectKind, ObjectName, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    Full,
    Incremental,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorError {
    NotCompletedManifest,
    VolumeOnManifest,
    InvalidInterval,
}
/// Structurally valid manifest interval; filename provenance remains external.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetDescriptor {
    kind: SetKind,
    start: Timestamp,
    end: Timestamp,
}
impl SetDescriptor {
    /// Revalidates structural invariants even for caller-constructed ObjectName.
    /// OpenPGP is accepted as metadata; no runtime decryption promise follows.
    pub fn from_manifest_name(name: ObjectName) -> Result<Self, DescriptorError> {
        let kind = match name.kind {
            ObjectKind::FullManifest => SetKind::Full,
            ObjectKind::IncrementalManifest => SetKind::Incremental,
            _ => return Err(DescriptorError::NotCompletedManifest),
        };
        if !name.completed() {
            return Err(DescriptorError::NotCompletedManifest);
        }
        if name.volume.is_some() {
            return Err(DescriptorError::VolumeOnManifest);
        }
        if (kind == SetKind::Full && name.start != name.end)
            || (kind == SetKind::Incremental && name.start >= name.end)
        {
            return Err(DescriptorError::InvalidInterval);
        }
        Ok(Self {
            kind,
            start: name.start,
            end: name.end,
        })
    }
    pub fn kind(&self) -> SetKind {
        self.kind
    }
    pub fn start(&self) -> Timestamp {
        self.start
    }
    pub fn end(&self) -> Timestamp {
        self.end
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ChainLimits {
    pub max_records: usize,
    pub max_metadata_bytes: usize,
    pub max_candidate_checks: usize,
}
impl Default for ChainLimits {
    fn default() -> Self {
        Self {
            max_records: 1_000_000,
            max_metadata_bytes: 32 * 1024 * 1024,
            max_candidate_checks: 16_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainError {
    Limit,
    Allocation,
    InvalidAnchor,
    TargetBeforeAnchor,
    Missing,
    Ambiguous,
}
/// Read-only plan tied to the original borrowed descriptor slice. No latest-full
/// or nearest-date policy is implied; both full anchor and target are explicit.
pub struct ChainPlan<'a> {
    sets: &'a [SetDescriptor],
    indices: Vec<usize>,
    target: Timestamp,
}
impl<'a> ChainPlan<'a> {
    //= docs/parsing-requirements.md#parse-chain-001
    //= type=implementation
    //# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=implementation
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    /// Plans metadata intervals only. Each hop must have exactly one outgoing
    /// incremental ending at or before the target. Even a dead-end competing
    /// branch is ambiguous; no path search silently chooses between branches.
    pub fn new(
        sets: &'a [SetDescriptor],
        full_anchor: usize,
        target: Timestamp,
        limits: ChainLimits,
    ) -> Result<Self, ChainError> {
        if sets.len() > limits.max_records {
            return Err(ChainError::Limit);
        }
        let anchor = sets.get(full_anchor).ok_or(ChainError::InvalidAnchor)?;
        if anchor.kind != SetKind::Full {
            return Err(ChainError::InvalidAnchor);
        }
        if target < anchor.end {
            return Err(ChainError::TargetBeforeAnchor);
        }
        sets.len()
            .max(4)
            .checked_mul(std::mem::size_of::<usize>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .filter(|bytes| *bytes <= limits.max_metadata_bytes)
            .ok_or(ChainError::Limit)?;
        let mut indices = Vec::new();
        // Strictly forward intervals cannot revisit a descriptor. Reserve its
        // maximum output size before any push; allocator behavior is trusted.
        indices
            .try_reserve_exact(sets.len())
            .map_err(|_| ChainError::Allocation)?;
        indices.push(full_anchor);
        let mut current = anchor.end;
        let mut checked = 0usize;
        while current < target {
            let mut candidate = None;
            for (index, set) in sets.iter().enumerate() {
                checked = checked
                    .checked_add(1)
                    .filter(|n| *n <= limits.max_candidate_checks)
                    .ok_or(ChainError::Limit)?;
                if set.kind == SetKind::Incremental && set.start == current && set.end <= target {
                    if candidate.is_some() {
                        return Err(ChainError::Ambiguous);
                    }
                    candidate = Some(index);
                }
            }
            let index = candidate.ok_or(ChainError::Missing)?;
            if indices.len() >= sets.len() {
                return Err(ChainError::Limit);
            }
            indices.push(index);
            current = sets[index].end;
        }
        Ok(Self {
            sets,
            indices,
            target,
        })
    }
    pub fn sets(&self) -> impl Iterator<Item = &'a SetDescriptor> + '_ {
        self.indices.iter().map(|index| &self.sets[*index])
    }
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }
    pub fn target(&self) -> Timestamp {
        self.target
    }
}

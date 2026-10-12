//! Metadata-only contiguous chain planning. This establishes neither volume
//! existence/integrity nor decryptability, signer identity or restorability.
use crate::names::{ObjectKind, ObjectName, Timestamp};
use vstd::prelude::*;

verus! {

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
pub closed spec fn descriptor_valid(set: SetDescriptor) -> bool {
    match set.kind {
        SetKind::Full => set.start.0 == set.end.0,
        SetKind::Incremental => set.start.0 < set.end.0,
    }
}

pub closed spec fn descriptor_model(name: ObjectName) -> Result<SetDescriptor, DescriptorError> {
    if !(name.kind == ObjectKind::FullManifest || name.kind == ObjectKind::IncrementalManifest)
        || name.partial || name.envelope == crate::names::Envelope::Pkcs7 {
        Err(DescriptorError::NotCompletedManifest)
    } else if name.volume.is_some() {
        Err(DescriptorError::VolumeOnManifest)
    } else {
        let kind = if name.kind == ObjectKind::FullManifest { SetKind::Full }
            else { SetKind::Incremental };
        if (kind == SetKind::Full && name.start.0 != name.end.0)
            || (kind == SetKind::Incremental && name.start.0 >= name.end.0) {
            Err(DescriptorError::InvalidInterval)
        } else {
            Ok(SetDescriptor { kind, start: name.start, end: name.end })
        }
    }
}

pub closed spec fn eligible(set: SetDescriptor, current: i128, target: i128) -> bool {
    set.kind == SetKind::Incremental && set.start.0 == current && set.end.0 <= target
}

// The scan charges each examined descriptor before inspecting it. A second
// candidate wins over any later exhaustion; the model preserves that priority.
pub closed spec fn scan_model(sets: Seq<SetDescriptor>, current: i128, target: i128,
    limit: nat, index: nat, checked: nat, candidate: Option<usize>)
    -> Result<(usize, nat), ChainError>
    decreases sets.len() - index,
{
    if index >= sets.len() {
        match candidate {
            None => Err(ChainError::Missing),
            Some(i) => Ok((i, checked)),
        }
    } else if checked >= limit || checked >= usize::MAX {
        Err(ChainError::Limit)
    } else if eligible(sets[index as int], current, target) {
        match candidate {
            Some(_) => Err(ChainError::Ambiguous),
            None => scan_model(sets, current, target, limit, index + 1,
                checked + 1, Some(index as usize)),
        }
    } else {
        scan_model(sets, current, target, limit, index + 1, checked + 1, candidate)
    }
}

// Explicit output-slot descent makes this model total even for inaccessible
// malformed descriptors. Constructor validity establishes strict time progress.
pub closed spec fn hops_model(sets: Seq<SetDescriptor>, target: i128, limit: nat,
    current: i128, checked: nat, indices: Seq<usize>) -> Result<Seq<usize>, ChainError>
    decreases sets.len() - indices.len(),
{
    if current >= target {
        Ok(indices)
    } else {
        match scan_model(sets, current, target, limit, 0, checked, None) {
            Err(e) => Err(e),
            Ok((i, next_checked)) => {
                if indices.len() >= sets.len() || i >= sets.len() {
                    Err(ChainError::Limit)
                } else {
                    hops_model(sets, target, limit, sets[i as int].end.0,
                        next_checked, indices.push(i))
                }
            }
        }
    }
}

pub closed spec fn chain_header(sets: Seq<SetDescriptor>, anchor: usize, target: Timestamp,
    limits: ChainLimits) -> Result<(), ChainError> {
    if sets.len() > limits.max_records {
        Err(ChainError::Limit)
    } else if anchor >= sets.len() || sets[anchor as int].kind != SetKind::Full {
        Err(ChainError::InvalidAnchor)
    } else if target.0 < sets[anchor as int].end.0 {
        Err(ChainError::TargetBeforeAnchor)
    } else {
        let slots = if sets.len() < 4 { 4nat } else { sets.len() };
        let bytes = slots * vstd::layout::size_of::<usize>();
        let total = bytes + vstd::layout::size_of::<ChainPlan>();
        if bytes > usize::MAX || total > usize::MAX || total > limits.max_metadata_bytes {
            Err(ChainError::Limit)
        } else { Ok(()) }
    }
}

pub closed spec fn valid_descriptors(sets: Seq<SetDescriptor>) -> bool {
    forall|i: int| 0 <= i < sets.len() ==> descriptor_valid(sets[i])
}

pub closed spec fn prefix_shape(sets: Seq<SetDescriptor>, indices: Seq<usize>, anchor: usize,
    current: i128) -> bool {
    0 < indices.len() <= sets.len()
        && (forall|i: int| 0 <= i < indices.len() ==> indices[i] < sets.len())
        && indices[0] == anchor && sets[anchor as int].kind == SetKind::Full
        && sets[indices[indices.len() - 1] as int].end.0 == current
        && (forall|i: int| 0 < i < indices.len() ==>
            sets[indices[i] as int].kind == SetKind::Incremental
            && sets[indices[i] as int].start.0 == sets[indices[i - 1] as int].end.0
            && sets[indices[i] as int].end.0 > sets[indices[i - 1] as int].end.0)
}

pub closed spec fn chain_model(sets: Seq<SetDescriptor>, anchor: usize, target: Timestamp,
    limits: ChainLimits) -> Result<Seq<usize>, ChainError> {
    match chain_header(sets, anchor, target, limits) {
        Err(e) => Err(e),
        Ok(()) => hops_model(sets, target.0, limits.max_candidate_checks as nat,
            sets[anchor as int].end.0, 0, seq![anchor]),
    }
}

impl SetDescriptor {
    pub closed spec fn kind_value(&self) -> SetKind { self.kind }
    pub closed spec fn start_value(&self) -> Timestamp { self.start }
    pub closed spec fn end_value(&self) -> Timestamp { self.end }
    /// Revalidates structural invariants even for caller-constructed ObjectName.
    /// OpenPGP is accepted as metadata; no runtime decryption promise follows.
    pub fn from_manifest_name(name: ObjectName) -> (result: Result<Self, DescriptorError>)
        ensures result == descriptor_model(name),
            match result { Ok(set) => descriptor_valid(set), Err(_) => true },
    {
        proof { reveal(descriptor_model); reveal(descriptor_valid); }
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
        if (matches!(kind, SetKind::Full) && name.start.0 != name.end.0)
            || (matches!(kind, SetKind::Incremental) && name.start.0 >= name.end.0)
        {
            return Err(DescriptorError::InvalidInterval);
        }
        Ok(Self {
            kind,
            start: name.start,
            end: name.end,
        })
    }
    pub fn kind(&self) -> (result: SetKind)
        ensures result == self.kind_value(),
    {
        self.kind
    }
    pub fn start(&self) -> (result: Timestamp)
        ensures result == self.start_value(),
    {
        self.start
    }
    pub fn end(&self) -> (result: Timestamp)
        ensures result == self.end_value(),
    {
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
    pub closed spec fn original_sets(&self) -> Seq<SetDescriptor> { self.sets@ }
    pub closed spec fn index_values(&self) -> Seq<usize> { self.indices@ }
    pub closed spec fn target_value(&self) -> Timestamp { self.target }
    pub closed spec fn bounded_indices(&self) -> bool {
        0 < self.indices@.len() <= self.sets@.len()
            && forall|i: int| 0 <= i < self.indices@.len()
                ==> self.indices@[i] < self.sets@.len()
    }
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
    ) -> (result: Result<Self, ChainError>)
        ensures match result {
            Ok(plan) => plan.original_sets() == sets@
                && plan.target_value() == target && plan.bounded_indices()
                && Ok(plan.index_values()) == chain_model(sets@, full_anchor, target, limits)
                && (valid_descriptors(sets@) ==> prefix_shape(sets@, plan.index_values(), full_anchor, target.0)),
            Err(ChainError::Allocation) => chain_header(sets@, full_anchor, target, limits) == Ok(()),
            Err(e) => Err(e) == chain_model(sets@, full_anchor, target, limits),
        },
    {
        proof {
            reveal(chain_model); reveal(chain_header); reveal(hops_model);
            reveal(prefix_shape); reveal(valid_descriptors); reveal(descriptor_valid);
            reveal(eligible);
        }
        if sets.len() > limits.max_records {
            return Err(ChainError::Limit);
        }
        let anchor = sets.get(full_anchor).ok_or(ChainError::InvalidAnchor)?;
        if !matches!(anchor.kind, SetKind::Full) {
            return Err(ChainError::InvalidAnchor);
        }
        if target.0 < anchor.end.0 {
            return Err(ChainError::TargetBeforeAnchor);
        }
        let slots = if sets.len() < 4 { 4 } else { sets.len() };
        let bytes = match slots.checked_mul(std::mem::size_of::<usize>()) {
            Some(bytes) => bytes,
            None => return Err(ChainError::Limit),
        };
        let total = match bytes.checked_add(std::mem::size_of::<Self>()) {
            Some(total) => total,
            None => return Err(ChainError::Limit),
        };
        if total > limits.max_metadata_bytes { return Err(ChainError::Limit); }
        let mut indices = Vec::new();
        // Strictly forward intervals cannot revisit a descriptor. Reserve its
        // maximum output size before any push; allocator behavior is trusted.
        match indices.try_reserve(sets.len()) {
            Ok(()) => {},
            Err(_) => return Err(ChainError::Allocation),
        }
        indices.push(full_anchor);
        let mut current = anchor.end;
        let mut checked = 0usize;
        while current.0 < target.0
            invariant
                0 < indices@.len() <= sets@.len(),
                forall|i: int| 0 <= i < indices@.len() ==> indices@[i] < sets@.len(),
                checked <= limits.max_candidate_checks,
                valid_descriptors(sets@) ==> current.0 <= target.0
                    && prefix_shape(sets@, indices@, full_anchor, current.0),
                chain_model(sets@, full_anchor, target, limits)
                    == hops_model(sets@, target.0, limits.max_candidate_checks as nat,
                        current.0, checked as nat, indices@),
            decreases sets@.len() - indices@.len(),
        {
            let ghost initial_checked = checked;
            let ghost initial_indices = indices@;
            let mut candidate: Option<usize> = None;
            let mut scan_index = 0usize;
            while scan_index < sets.len()
                invariant
                    current.0 < target.0,
                    chain_model(sets@, full_anchor, target, limits)
                        == hops_model(sets@, target.0, limits.max_candidate_checks as nat,
                            current.0, initial_checked as nat, initial_indices),
                    initial_indices == indices@,
                    scan_index <= sets@.len(),
                    checked <= limits.max_candidate_checks,
                    match candidate {
                        None => true,
                        Some(i) => i < scan_index && eligible(sets@[i as int], current.0, target.0),
                    },
                    scan_model(sets@, current.0, target.0, limits.max_candidate_checks as nat,
                        0, initial_checked as nat, None)
                    == scan_model(sets@, current.0, target.0, limits.max_candidate_checks as nat,
                        scan_index as nat, checked as nat, candidate),
                decreases sets@.len() - scan_index,
            {
                proof { reveal_with_fuel(scan_model, 2); reveal(hops_model); reveal(eligible); }
                let index = scan_index;
                let set = &sets[index];
                checked = match checked.checked_add(1) {
                    Some(n) if n <= limits.max_candidate_checks => n,
                    _ => return Err(ChainError::Limit),
                };
                if matches!(set.kind, SetKind::Incremental) && set.start.0 == current.0 && set.end.0 <= target.0 {
                    if candidate.is_some() {
                        return Err(ChainError::Ambiguous);
                    }
                    candidate = Some(index);
                }
                scan_index += 1;
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
    pub fn indices(&self) -> (result: &[usize])
        ensures result@ == self.index_values(),
    {
        &self.indices
    }
    pub fn target(&self) -> (result: Timestamp)
        ensures result == self.target_value(),
    {
        self.target
    }
}

}

// Thin native iterator adapter: its standard-library combinators remain outside
// the verified controller. The controller proves every stored index is bounded.
impl<'a> ChainPlan<'a> {
    pub fn sets(&self) -> impl Iterator<Item = &'a SetDescriptor> + '_ {
        self.indices.iter().map(|index| &self.sets[*index])
    }
}

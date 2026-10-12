//! Manifest-aware volume candidates. Raw repeated-number evidence disables
//! interval pruning; ordinary declared hints require their supported profile.
//! Allocation/sort and controller refinement remain trusted/unproved.
use crate::manifest::{Manifest, VolumeRecord};
use crate::path::BackupPath;

#[derive(Clone, Copy, Debug)]
pub struct SelectionLimits {
    pub max_raw_records: usize,
    pub max_metadata_bytes: usize,
}
impl Default for SelectionLimits {
    fn default() -> Self {
        Self {
            max_raw_records: 1_000_000,
            max_metadata_bytes: 64 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    Limit,
    EmptyManifest,
}
pub struct VolumeSelection<'a> {
    records: Vec<&'a VolumeRecord>,
    requires_full_scan: bool,
}
impl<'a> VolumeSelection<'a> {
    pub fn records(&self) -> &[&'a VolumeRecord] {
        &self.records
    }
    pub fn requires_full_scan(&self) -> bool {
        self.requires_full_scan
    }
}
impl Manifest {
    //= docs/parsing-requirements.md#parse-man-010
    //= type=implementation
    //# A reader MUST scan every normalized volume of a restart-rewritten set when raw manifest intervals have not been certified to cover decoded entries by an object-identity-bound content index.
    /// Returns every normalized record if any raw volume number is repeated.
    /// Otherwise uses declared interval hints under the normal producer profile;
    /// a record-only hint never certifies that a decoded container obeys it.
    pub fn select_volumes<'a>(
        &'a self,
        selected: &BackupPath,
        limits: SelectionLimits,
    ) -> Result<VolumeSelection<'a>, SelectionError> {
        if self.records.is_empty() {
            return Err(SelectionError::EmptyManifest);
        }
        if self.records.len() > limits.max_raw_records {
            return Err(SelectionError::Limit);
        }
        // Precharge simultaneous raw number storage, normalized Vec capacity,
        // filtered Vec capacity and return object before any allocation. Vec
        // normalization/filter growth is bounded by 2N with four minimum slots;
        // sorting the exact-capacity u64 buffer is in-place (std trusted).
        let bound = self
            .records
            .len()
            .max(4)
            .checked_mul(std::mem::size_of::<u64>() + 4 * std::mem::size_of::<&VolumeRecord>())
            .and_then(|n| n.checked_add(std::mem::size_of::<VolumeSelection<'_>>()))
            .filter(|n| *n <= limits.max_metadata_bytes)
            .ok_or(SelectionError::Limit)?;
        let _ = bound;
        let mut numbers: Vec<u64> = Vec::with_capacity(self.records.len());
        for record in &self.records {
            numbers.push(record.number);
        }
        numbers.sort_unstable();
        let requires_full_scan = numbers.windows(2).any(|pair| pair[0] == pair[1]);
        let mut records = self.volumes();
        if records.is_empty() {
            return Err(SelectionError::EmptyManifest);
        }
        if !requires_full_scan {
            records.retain(|record| record.may_contain(selected));
        }
        Ok(VolumeSelection {
            records,
            requires_full_scan,
        })
    }
}

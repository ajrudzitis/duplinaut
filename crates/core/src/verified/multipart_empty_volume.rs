//! Verified admission for a tar volume that emits no logical data entries.
use crate::manifest::Boundary;
use crate::path::BackupPath;
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyVolumeStateView<'a> {
    pub previous: Option<&'a Boundary>,
    pub pending: Option<(&'a BackupPath, u64)>,
    pub completed_entries: usize,
    pub payload_bytes: usize,
}

pub open spec fn same_path_model(left: &BackupPath, right: &BackupPath) -> bool {
    left@ == right@
}

pub fn same_path(left: &BackupPath, right: &BackupPath) -> (result: bool)
    ensures result == same_path_model(left, right),
{
    left.compare(right) == 0
}

pub open spec fn same_boundary_model(left: &Boundary, right: &Boundary) -> bool {
    same_path_model(&left.path, &right.path) && left.block == right.block
}

pub fn same_boundary(left: &Boundary, right: &Boundary) -> (result: bool)
    ensures result == same_boundary_model(left, right),
{
    same_path(&left.path, &right.path) && left.block == right.block
}

pub open spec fn empty_volume_admission_model<'a>(
    root_none: &'a Boundary,
    start: &'a Boundary,
    end: &'a Boundary,
    state: EmptyVolumeStateView<'a>,
) -> Option<EmptyVolumeStateView<'a>> {
    let root_is_none = root_none.path@.len() == 0 && root_none.block.is_none();
    let adjacent = same_boundary_model(start, root_none)
        && match state.previous {
            Some(previous) => same_boundary_model(end, previous),
            None => same_boundary_model(end, root_none),
        };
    let pending_boundary_matches = match state.pending {
        Some((path, next_part)) => {
            next_part > 0
                && same_path_model(&end.path, path)
                && end.block == Some((next_part - 1) as u64)
        },
        None => end.block.is_none(),
    };
    if root_is_none && adjacent && pending_boundary_matches {
        Some(EmptyVolumeStateView {
            previous: Some(end),
            pending: state.pending,
            completed_entries: state.completed_entries,
            payload_bytes: state.payload_bytes,
        })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-multi-004
//= type=implementation
//# Assembly MUST preserve pending logical-entry state across an empty volume while validating its supported empty-boundary marker shape.
pub fn admit_empty_volume<'a>(
    root_none: &'a Boundary,
    start: &'a Boundary,
    end: &'a Boundary,
    state: EmptyVolumeStateView<'a>,
) -> (result: Option<EmptyVolumeStateView<'a>>)
    ensures result == empty_volume_admission_model(root_none, start, end, state),
{
    if !root_none.path.is_root() {
        assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
            reveal(empty_volume_admission_model);
        }
        return None;
    }
    if root_none.block.is_some() {
        assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
            reveal(empty_volume_admission_model);
        }
        return None;
    }
    if !same_boundary(start, root_none) {
        assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
            reveal(empty_volume_admission_model);
        }
        return None;
    }
    match state.previous {
        Some(previous) => {
            if !same_boundary(end, previous) {
                assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
                    reveal(empty_volume_admission_model);
                }
                return None;
            }
        }
        None => {
            if !same_boundary(end, root_none) {
                assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
                    reveal(empty_volume_admission_model);
                }
                return None;
            }
        }
    }
    match state.pending {
        Some((path, next_part)) => {
            let last_part = next_part.checked_sub(1)?;
            if !same_path(&end.path, path) || end.block != Some(last_part) {
                assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
                    reveal(empty_volume_admission_model);
                }
                return None;
            }
        }
        None => {
            if end.block.is_some() {
                assert(empty_volume_admission_model(root_none, start, end, state) == None) by {
                    reveal(empty_volume_admission_model);
                }
                return None;
            }
        }
    }
    Some(EmptyVolumeStateView {
        previous: Some(end),
        pending: state.pending,
        completed_entries: state.completed_entries,
        payload_bytes: state.payload_bytes,
    })
}

}

#[cfg(test)]
mod tests {
    use super::{admit_empty_volume, EmptyVolumeStateView};
    use crate::manifest::Boundary;
    use crate::path::BackupPath;

    fn path(bytes: &[u8]) -> BackupPath {
        BackupPath::parse(bytes, false, 4096, 32).unwrap()
    }

    fn boundary(path: &BackupPath, block: Option<u64>) -> Boundary {
        Boundary {
            path: path.clone(),
            block,
        }
    }

    fn state<'a>(
        previous: Option<&'a Boundary>,
        pending: Option<(&'a BackupPath, u64)>,
    ) -> EmptyVolumeStateView<'a> {
        EmptyVolumeStateView {
            previous,
            pending,
            completed_entries: 7,
            payload_bytes: 1234,
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-multi-004
    //= type=test
    //# Assembly MUST preserve pending logical-entry state across an empty volume while validating its supported empty-boundary marker shape.
    fn empty_volume_transition_truth_table_uses_project_boundaries_and_preserves_state() {
        let root_path = path(b".");
        let root_none = boundary(&root_path, None);
        let file = path(b"file");
        let file_part_one = boundary(&file, Some(1));

        let initial = state(None, None);
        let accepted = admit_empty_volume(&root_none, &root_none, &root_none, initial).unwrap();
        assert_eq!(accepted.previous, Some(&root_none));
        assert_eq!(accepted.pending, None);
        assert_eq!(accepted.completed_entries, 7);
        assert_eq!(accepted.payload_bytes, 1234);

        let pending = state(Some(&file_part_one), Some((&file, 2)));
        let accepted = admit_empty_volume(&root_none, &root_none, &file_part_one, pending).unwrap();
        assert_eq!(accepted.previous, Some(&file_part_one));
        assert_eq!(accepted.pending, Some((&file, 2)));
        assert_eq!(accepted.completed_entries, 7);
        assert_eq!(accepted.payload_bytes, 1234);

        // Each negative case changes just the named boundary relation.
        let non_root_start = boundary(&file, None);
        assert!(admit_empty_volume(&root_none, &non_root_start, &root_none, initial).is_none());
        assert!(admit_empty_volume(
            &root_none,
            &root_none,
            &root_none,
            state(Some(&file_part_one), None)
        )
        .is_none());
        assert!(admit_empty_volume(
            &root_none,
            &root_none,
            &file_part_one,
            state(Some(&file_part_one), Some((&root_path, 2))),
        )
        .is_none());
        assert!(admit_empty_volume(
            &root_none,
            &root_none,
            &file_part_one,
            state(Some(&file_part_one), Some((&file, 3))),
        )
        .is_none());
        assert!(admit_empty_volume(
            &root_none,
            &root_none,
            &file_part_one,
            state(Some(&file_part_one), None),
        )
        .is_none());

        // A present pending part with next_part zero is not the absent case.
        assert!(admit_empty_volume(
            &root_none,
            &root_none,
            &root_none,
            state(None, Some((&file, 0))),
        )
        .is_none());
    }
}

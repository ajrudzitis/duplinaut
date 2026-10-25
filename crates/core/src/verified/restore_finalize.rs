//! Reverse plan-position cursor for Linux directory finalization.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalizationCursor {
    Nodes { next_index: usize, pending_directory: Option<usize> },
    RootReady,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalizationEvent {
    BeginDirectory { index: usize },
    CommitDirectory { index: usize },
    SkipNonDirectory { index: usize },
    BeginRoot,
    CommitRoot,
}

pub open spec fn finalization_cursor_model(
    cursor: FinalizationCursor,
    event: FinalizationEvent,
) -> Option<FinalizationCursor> {
    match (cursor, event) {
        (
            FinalizationCursor::Nodes { next_index, pending_directory: None },
            FinalizationEvent::BeginDirectory { index },
        ) => {
            if index as int + 1 == next_index as int {
                Some(FinalizationCursor::Nodes {
                    next_index,
                    pending_directory: Some(index),
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index, pending_directory: Some(pending) },
            FinalizationEvent::CommitDirectory { index },
        ) => {
            if index == pending && index as int + 1 == next_index as int {
                Some(FinalizationCursor::Nodes {
                    next_index: index,
                    pending_directory: None,
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index, pending_directory: None },
            FinalizationEvent::SkipNonDirectory { index },
        ) => {
            if index as int + 1 == next_index as int {
                Some(FinalizationCursor::Nodes {
                    next_index: index,
                    pending_directory: None,
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index: 0, pending_directory: None },
            FinalizationEvent::BeginRoot,
        ) => Some(FinalizationCursor::RootReady),
        (FinalizationCursor::RootReady, FinalizationEvent::CommitRoot) => {
            Some(FinalizationCursor::Complete)
        },
        _ => None,
    }
}

//= docs/parsing-requirements.md#restore-018
//= type=implementation
//# Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.
pub(crate) fn advance_finalization_cursor(
    cursor: FinalizationCursor,
    event: FinalizationEvent,
) -> (next: Option<FinalizationCursor>)
    ensures next == finalization_cursor_model(cursor, event),
{
    match (cursor, event) {
        (
            FinalizationCursor::Nodes { next_index, pending_directory: None },
            FinalizationEvent::BeginDirectory { index },
        ) => {
            if index.checked_add(1) == Some(next_index) {
                Some(FinalizationCursor::Nodes {
                    next_index,
                    pending_directory: Some(index),
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index, pending_directory: Some(pending) },
            FinalizationEvent::CommitDirectory { index },
        ) => {
            if index == pending && index.checked_add(1) == Some(next_index) {
                Some(FinalizationCursor::Nodes {
                    next_index: index,
                    pending_directory: None,
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index, pending_directory: None },
            FinalizationEvent::SkipNonDirectory { index },
        ) => {
            if index.checked_add(1) == Some(next_index) {
                Some(FinalizationCursor::Nodes {
                    next_index: index,
                    pending_directory: None,
                })
            } else {
                None
            }
        },
        (
            FinalizationCursor::Nodes { next_index: 0, pending_directory: None },
            FinalizationEvent::BeginRoot,
        ) => Some(FinalizationCursor::RootReady),
        (FinalizationCursor::RootReady, FinalizationEvent::CommitRoot) => {
            Some(FinalizationCursor::Complete)
        },
        _ => None,
    }
}

pub(crate) fn start_finalization(node_count: usize) -> FinalizationCursor {
    FinalizationCursor::Nodes { next_index: node_count, pending_directory: None }
}

}

#[cfg(test)]
mod tests {
    use super::{
        advance_finalization_cursor, start_finalization, FinalizationCursor, FinalizationEvent,
    };

    #[test]
    //= docs/parsing-requirements.md#restore-018
    //= type=test
    //# Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.
    fn finalization_cursor_requires_reverse_commits_and_root_last() {
        let mut cursor = start_finalization(3);
        assert_eq!(
            advance_finalization_cursor(cursor, FinalizationEvent::BeginDirectory { index: 2 }),
            Some(FinalizationCursor::Nodes {
                next_index: 3,
                pending_directory: Some(2),
            })
        );
        cursor =
            advance_finalization_cursor(cursor, FinalizationEvent::BeginDirectory { index: 2 })
                .unwrap();
        assert!(advance_finalization_cursor(cursor, FinalizationEvent::BeginRoot).is_none());
        assert!(advance_finalization_cursor(
            cursor,
            FinalizationEvent::CommitDirectory { index: 1 },
        )
        .is_none());
        cursor =
            advance_finalization_cursor(cursor, FinalizationEvent::CommitDirectory { index: 2 })
                .unwrap();
        cursor =
            advance_finalization_cursor(cursor, FinalizationEvent::SkipNonDirectory { index: 1 })
                .unwrap();
        cursor =
            advance_finalization_cursor(cursor, FinalizationEvent::BeginDirectory { index: 0 })
                .unwrap();
        cursor =
            advance_finalization_cursor(cursor, FinalizationEvent::CommitDirectory { index: 0 })
                .unwrap();
        assert!(advance_finalization_cursor(
            cursor,
            FinalizationEvent::SkipNonDirectory { index: 0 },
        )
        .is_none());
        cursor = advance_finalization_cursor(cursor, FinalizationEvent::BeginRoot).unwrap();
        assert!(advance_finalization_cursor(cursor, FinalizationEvent::BeginRoot).is_none());
        assert_eq!(
            advance_finalization_cursor(cursor, FinalizationEvent::CommitRoot),
            Some(FinalizationCursor::Complete)
        );
    }
}

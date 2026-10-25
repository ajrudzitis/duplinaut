//! Phase transitions for local reads guarded by before/after identity checks.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalReadPhase {
    Start { offset: u64 },
    Prevalidated { offset: u64 },
    ReadObserved { offset: u64, requested: u64, count: u64 },
    Postvalidated { offset: u64, requested: u64, count: u64 },
    Committed { offset: u64, requested: u64, count: u64, next_offset: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalReadEvent {
    PrevalidationSucceeded,
    ReadSucceeded { requested: u64, count: u64 },
    PostvalidationSucceeded,
    Commit,
}

pub open spec fn local_read_phase_model(
    phase: LocalReadPhase,
    event: LocalReadEvent,
) -> Option<LocalReadPhase> {
    match (phase, event) {
        (LocalReadPhase::Start { offset }, LocalReadEvent::PrevalidationSucceeded) =>
            Some(LocalReadPhase::Prevalidated { offset }),
        (
            LocalReadPhase::Prevalidated { offset },
            LocalReadEvent::ReadSucceeded { requested, count },
        ) => {
            if count <= requested {
                Some(LocalReadPhase::ReadObserved { offset, requested, count })
            } else {
                None
            }
        },
        (
            LocalReadPhase::ReadObserved { offset, requested, count },
            LocalReadEvent::PostvalidationSucceeded,
        ) => Some(LocalReadPhase::Postvalidated { offset, requested, count }),
        (
            LocalReadPhase::Postvalidated { offset, requested, count },
            LocalReadEvent::Commit,
        ) => {
            if offset as int + count as int <= u64::MAX as int {
                Some(LocalReadPhase::Committed {
                    offset,
                    requested,
                    count,
                    next_offset: (offset + count) as u64,
                })
            } else {
                None
            }
        },
        _ => None,
    }
}

//= docs/parsing-requirements.md#parse-source-001
//= type=implementation
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
pub(crate) fn advance_local_read_phase(
    phase: LocalReadPhase,
    event: LocalReadEvent,
) -> (next: Option<LocalReadPhase>)
    ensures next == local_read_phase_model(phase, event),
{
    match (phase, event) {
        (LocalReadPhase::Start { offset }, LocalReadEvent::PrevalidationSucceeded) =>
            Some(LocalReadPhase::Prevalidated { offset }),
        (
            LocalReadPhase::Prevalidated { offset },
            LocalReadEvent::ReadSucceeded { requested, count },
        ) => {
            if count <= requested {
                Some(LocalReadPhase::ReadObserved { offset, requested, count })
            } else {
                None
            }
        },
        (
            LocalReadPhase::ReadObserved { offset, requested, count },
            LocalReadEvent::PostvalidationSucceeded,
        ) => Some(LocalReadPhase::Postvalidated { offset, requested, count }),
        (
            LocalReadPhase::Postvalidated { offset, requested, count },
            LocalReadEvent::Commit,
        ) => {
            let next_offset = offset.checked_add(count)?;
            Some(LocalReadPhase::Committed { offset, requested, count, next_offset })
        },
        _ => None,
    }
}

}

#[cfg(test)]
mod tests {
    use super::{advance_local_read_phase, LocalReadEvent, LocalReadPhase};

    #[test]
    //= docs/parsing-requirements.md#parse-source-001
    //= type=test
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    fn local_read_phase_requires_both_validations_before_checked_commit() {
        let start = LocalReadPhase::Start { offset: 12 };
        let prevalidated =
            advance_local_read_phase(start, LocalReadEvent::PrevalidationSucceeded).unwrap();
        assert!(advance_local_read_phase(
            start,
            LocalReadEvent::ReadSucceeded {
                requested: 4,
                count: 4
            }
        )
        .is_none());
        let observed = advance_local_read_phase(
            prevalidated,
            LocalReadEvent::ReadSucceeded {
                requested: 4,
                count: 3,
            },
        )
        .unwrap();
        assert!(advance_local_read_phase(observed, LocalReadEvent::Commit).is_none());
        let postvalidated =
            advance_local_read_phase(observed, LocalReadEvent::PostvalidationSucceeded).unwrap();
        assert!(
            advance_local_read_phase(postvalidated, LocalReadEvent::PostvalidationSucceeded)
                .is_none()
        );
        assert_eq!(
            advance_local_read_phase(postvalidated, LocalReadEvent::Commit),
            Some(LocalReadPhase::Committed {
                offset: 12,
                requested: 4,
                count: 3,
                next_offset: 15
            }),
        );
        let max = LocalReadPhase::Postvalidated {
            offset: u64::MAX,
            requested: 1,
            count: 1,
        };
        assert!(advance_local_read_phase(max, LocalReadEvent::Commit).is_none());
        let invalid_count = advance_local_read_phase(
            prevalidated,
            LocalReadEvent::ReadSucceeded {
                requested: 2,
                count: 3,
            },
        );
        assert!(invalid_count.is_none());
    }
}

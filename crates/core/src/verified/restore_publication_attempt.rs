//! One-shot authorization state for the Linux publication attempt.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationAttemptPhase {
    Ready,
    Attempted,
    Published,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationAttemptEvent {
    Begin,
    FinishSuccess,
    FinishError,
}

pub open spec fn publication_attempt_model(
    phase: PublicationAttemptPhase,
    event: PublicationAttemptEvent,
) -> Option<PublicationAttemptPhase> {
    match (phase, event) {
        (PublicationAttemptPhase::Ready, PublicationAttemptEvent::Begin) => {
            Some(PublicationAttemptPhase::Attempted)
        },
        (PublicationAttemptPhase::Attempted, PublicationAttemptEvent::FinishSuccess) => {
            Some(PublicationAttemptPhase::Published)
        },
        (PublicationAttemptPhase::Attempted, PublicationAttemptEvent::FinishError) => {
            Some(PublicationAttemptPhase::Indeterminate)
        },
        _ => None,
    }
}

//= docs/parsing-requirements.md#restore-020
//= type=implementation
//# The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.
pub(crate) fn advance_publication_attempt(
    phase: PublicationAttemptPhase,
    event: PublicationAttemptEvent,
) -> (next: Option<PublicationAttemptPhase>)
    ensures next == publication_attempt_model(phase, event),
{
    match (phase, event) {
        (PublicationAttemptPhase::Ready, PublicationAttemptEvent::Begin) => {
            Some(PublicationAttemptPhase::Attempted)
        },
        (PublicationAttemptPhase::Attempted, PublicationAttemptEvent::FinishSuccess) => {
            Some(PublicationAttemptPhase::Published)
        },
        (PublicationAttemptPhase::Attempted, PublicationAttemptEvent::FinishError) => {
            Some(PublicationAttemptPhase::Indeterminate)
        },
        _ => None,
    }
}

}

#[cfg(test)]
mod tests {
    use super::{
        advance_publication_attempt, PublicationAttemptEvent as Event,
        PublicationAttemptPhase as Phase,
    };

    #[test]
    //= docs/parsing-requirements.md#restore-020
    //= type=test
    //# The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.
    fn publication_attempt_is_one_shot_and_terminal() {
        let attempted = advance_publication_attempt(Phase::Ready, Event::Begin).unwrap();
        assert_eq!(attempted, Phase::Attempted);
        assert!(advance_publication_attempt(attempted, Event::Begin).is_none());

        let published = advance_publication_attempt(attempted, Event::FinishSuccess).unwrap();
        assert_eq!(published, Phase::Published);
        assert!(advance_publication_attempt(published, Event::Begin).is_none());
        assert!(advance_publication_attempt(published, Event::FinishError).is_none());

        let indeterminate = advance_publication_attempt(attempted, Event::FinishError).unwrap();
        assert_eq!(indeterminate, Phase::Indeterminate);
        assert!(advance_publication_attempt(indeterminate, Event::Begin).is_none());
        assert!(advance_publication_attempt(indeterminate, Event::FinishSuccess).is_none());
    }
}

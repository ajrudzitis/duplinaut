//! Cleanup authority after the Linux publication linearization point.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationCleanup {
    None,
    EmptyWrapper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationDecision {
    Indeterminate,
    Published(PublicationCleanup),
}

pub(crate) open spec fn publication_decision_model(
    rename_succeeded: bool,
    cancellation_requested: bool,
    has_wrapper: bool,
) -> PublicationDecision {
    if rename_succeeded {
        if has_wrapper {
            PublicationDecision::Published(PublicationCleanup::EmptyWrapper)
        } else {
            PublicationDecision::Published(PublicationCleanup::None)
        }
    } else {
        PublicationDecision::Indeterminate
    }
}

//= docs/parsing-requirements.md#restore-021
//= type=implementation
//# Cancellation MUST be observed before each entry, each bounded write attempt and the final publication attempt, with successful publication taking precedence over a racing cancellation request.
//= docs/parsing-requirements.md#restore-023
//= type=implementation
//# The result MUST distinguish unpublished failure/cancellation, published success, and publication-indeterminate outcomes, retaining post-success wrapper-removal faults as cleanup warnings on published success.
//= docs/parsing-requirements.md#restore-026
//= type=implementation
//# If a publication rename is attempted and returns an error, the backend MUST report publication uncertainty and refrain from modifying cleanup of the restore root, its descendants or its staging wrapper.
pub(crate) fn decide_publication_cleanup(
    rename_succeeded: bool,
    _cancellation_requested: bool,
    has_wrapper: bool,
) -> (decision: PublicationDecision)
    ensures decision == publication_decision_model(
        rename_succeeded,
        _cancellation_requested,
        has_wrapper,
    ),
{
    if rename_succeeded {
        if has_wrapper {
            PublicationDecision::Published(PublicationCleanup::EmptyWrapper)
        } else {
            PublicationDecision::Published(PublicationCleanup::None)
        }
    } else {
        PublicationDecision::Indeterminate
    }
}

}

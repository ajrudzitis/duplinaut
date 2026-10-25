//! Authority decision for the first outer staging-directory creation result.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OuterStageObservation {
    Created,
    ExistingCollision,
    OtherError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OuterStageDecision {
    OwnedNamespace,
    RetryCandidate,
    UncertainCandidate,
}

pub open spec fn outer_stage_decision_model(
    observation: OuterStageObservation,
) -> OuterStageDecision {
    match observation {
        OuterStageObservation::Created => OuterStageDecision::OwnedNamespace,
        OuterStageObservation::ExistingCollision => OuterStageDecision::RetryCandidate,
        OuterStageObservation::OtherError => OuterStageDecision::UncertainCandidate,
    }
}

//= docs/parsing-requirements.md#restore-027
//= type=implementation
//# If creation of the first outer staging leaf returns an error other than a definite existing-entry collision, the backend MUST report its candidate as uncertain and leave that candidate untouched because exclusive ownership was not established.
pub(crate) fn decide_outer_stage(
    observation: OuterStageObservation,
) -> (decision: OuterStageDecision)
    ensures decision == outer_stage_decision_model(observation),
{
    match observation {
        OuterStageObservation::Created => OuterStageDecision::OwnedNamespace,
        OuterStageObservation::ExistingCollision => OuterStageDecision::RetryCandidate,
        OuterStageObservation::OtherError => OuterStageDecision::UncertainCandidate,
    }
}

}

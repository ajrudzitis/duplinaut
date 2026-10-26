use crate::names::ObjectKind;
use vstd::prelude::*;

verus! {
pub open spec fn short_full_signatures_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullSignatures && short
}

/// Identify exactly the short FullSignatures route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub(crate) fn short_full_signatures_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_full_signatures_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullSignatures, true) => true,
        _ => false,
    }
}
}

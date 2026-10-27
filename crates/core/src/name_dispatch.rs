use crate::names::ObjectKind;
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BodyRoute {
    ShortIncrementalSignatures,
    ShortIncrementalManifest,
    ShortFullManifest,
    LongFullManifest,
    LongFullSignatures,
    LongIncrementalManifest,
    LongIncrementalSignatures,
    ShortFullSignatures,
    ShortIncrementalVolume,
    ShortFullVolume,
    LongFullVolume,
    LongIncrementalVolume,
    Unmatched,
}

pub closed spec fn selected_body_route_model(kind: ObjectKind, short: bool) -> BodyRoute {
    if short_incremental_signatures_route_model(kind, short) {
        BodyRoute::ShortIncrementalSignatures
    } else if short_incremental_manifest_route_model(kind, short) {
        BodyRoute::ShortIncrementalManifest
    } else if short_full_manifest_route_model(kind, short) {
        BodyRoute::ShortFullManifest
    } else if long_full_manifest_route_model(kind, short) {
        BodyRoute::LongFullManifest
    } else if long_full_signatures_route_model(kind, short) {
        BodyRoute::LongFullSignatures
    } else if long_incremental_manifest_route_model(kind, short) {
        BodyRoute::LongIncrementalManifest
    } else if long_incremental_signatures_route_model(kind, short) {
        BodyRoute::LongIncrementalSignatures
    } else if short_full_signatures_route_model(kind, short) {
        BodyRoute::ShortFullSignatures
    } else if short_incremental_volume_route_model(kind, short) {
        BodyRoute::ShortIncrementalVolume
    } else if short_full_volume_route_model(kind, short) {
        BodyRoute::ShortFullVolume
    } else if long_full_volume_route_model(kind, short) {
        BodyRoute::LongFullVolume
    } else if long_incremental_volume_route_model(kind, short) {
        BodyRoute::LongIncrementalVolume
    } else {
        BodyRoute::Unmatched
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub(crate) fn select_body_route(kind: ObjectKind, short: bool) -> (route: BodyRoute)
    ensures route == selected_body_route_model(kind, short),
        route != BodyRoute::Unmatched,
        match route {
            BodyRoute::ShortIncrementalSignatures => {
                kind == ObjectKind::IncrementalSignatures && short
            }
            BodyRoute::ShortIncrementalManifest => {
                kind == ObjectKind::IncrementalManifest && short
            }
            BodyRoute::ShortFullManifest => kind == ObjectKind::FullManifest && short,
            BodyRoute::LongFullManifest => kind == ObjectKind::FullManifest && !short,
            BodyRoute::LongFullSignatures => kind == ObjectKind::FullSignatures && !short,
            BodyRoute::LongIncrementalManifest => {
                kind == ObjectKind::IncrementalManifest && !short
            }
            BodyRoute::LongIncrementalSignatures => {
                kind == ObjectKind::IncrementalSignatures && !short
            }
            BodyRoute::ShortFullSignatures => kind == ObjectKind::FullSignatures && short,
            BodyRoute::ShortIncrementalVolume => {
                kind == ObjectKind::IncrementalVolume && short
            }
            BodyRoute::ShortFullVolume => kind == ObjectKind::FullVolume && short,
            BodyRoute::LongFullVolume => kind == ObjectKind::FullVolume && !short,
            BodyRoute::LongIncrementalVolume => {
                kind == ObjectKind::IncrementalVolume && !short
            }
            BodyRoute::Unmatched => false,
        },
{
    let _count = body_route_partition(kind, short);
    let route = if short_incremental_signatures_route(kind, short) {
        BodyRoute::ShortIncrementalSignatures
    } else if short_incremental_manifest_route(kind, short) {
        BodyRoute::ShortIncrementalManifest
    } else if short_full_manifest_route(kind, short) {
        BodyRoute::ShortFullManifest
    } else if long_full_manifest_route(kind, short) {
        BodyRoute::LongFullManifest
    } else if long_full_signatures_route(kind, short) {
        BodyRoute::LongFullSignatures
    } else if long_incremental_manifest_route(kind, short) {
        BodyRoute::LongIncrementalManifest
    } else if long_incremental_signatures_route(kind, short) {
        BodyRoute::LongIncrementalSignatures
    } else if short_full_signatures_route(kind, short) {
        BodyRoute::ShortFullSignatures
    } else if short_incremental_volume_route(kind, short) {
        BodyRoute::ShortIncrementalVolume
    } else if short_full_volume_route(kind, short) {
        BodyRoute::ShortFullVolume
    } else if long_full_volume_route(kind, short) {
        BodyRoute::LongFullVolume
    } else if long_incremental_volume_route(kind, short) {
        BodyRoute::LongIncrementalVolume
    } else {
        BodyRoute::Unmatched
    };
    proof {
        reveal(selected_body_route_model);
        reveal(body_route_count_model);
        assert(_count == 1);
        assert(route != BodyRoute::Unmatched);
        assert(match route {
            BodyRoute::ShortIncrementalSignatures => {
                kind == ObjectKind::IncrementalSignatures && short
            }
            BodyRoute::ShortIncrementalManifest => {
                kind == ObjectKind::IncrementalManifest && short
            }
            BodyRoute::ShortFullManifest => kind == ObjectKind::FullManifest && short,
            BodyRoute::LongFullManifest => kind == ObjectKind::FullManifest && !short,
            BodyRoute::LongFullSignatures => kind == ObjectKind::FullSignatures && !short,
            BodyRoute::LongIncrementalManifest => {
                kind == ObjectKind::IncrementalManifest && !short
            }
            BodyRoute::LongIncrementalSignatures => {
                kind == ObjectKind::IncrementalSignatures && !short
            }
            BodyRoute::ShortFullSignatures => kind == ObjectKind::FullSignatures && short,
            BodyRoute::ShortIncrementalVolume => {
                kind == ObjectKind::IncrementalVolume && short
            }
            BodyRoute::ShortFullVolume => kind == ObjectKind::FullVolume && short,
            BodyRoute::LongFullVolume => kind == ObjectKind::FullVolume && !short,
            BodyRoute::LongIncrementalVolume => {
                kind == ObjectKind::IncrementalVolume && !short
            }
            BodyRoute::Unmatched => false,
        }) by (compute);
    }
    route
}

pub open spec fn short_full_signatures_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullSignatures && short
}

/// Identify exactly the short FullSignatures route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_full_signatures_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_full_signatures_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullSignatures, true) => true,
        _ => false,
    }
}

pub open spec fn long_full_signatures_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullSignatures && !short
}

/// Identify exactly the long FullSignatures route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_full_signatures_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_full_signatures_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullSignatures, false) => true,
        _ => false,
    }
}

pub open spec fn short_full_manifest_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullManifest && short
}

/// Identify exactly the short FullManifest route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_full_manifest_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_full_manifest_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullManifest, true) => true,
        _ => false,
    }
}

pub open spec fn long_full_manifest_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullManifest && !short
}

/// Identify exactly the long FullManifest route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_full_manifest_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_full_manifest_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullManifest, false) => true,
        _ => false,
    }
}

pub open spec fn short_incremental_manifest_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalManifest && short
}

/// Identify exactly the short IncrementalManifest route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_incremental_manifest_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_incremental_manifest_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalManifest, true) => true,
        _ => false,
    }
}

pub open spec fn long_incremental_manifest_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalManifest && !short
}

/// Identify exactly the long IncrementalManifest route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_incremental_manifest_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_incremental_manifest_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalManifest, false) => true,
        _ => false,
    }
}

pub open spec fn short_incremental_signatures_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalSignatures && short
}

/// Identify exactly the short IncrementalSignatures route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_incremental_signatures_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_incremental_signatures_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalSignatures, true) => true,
        _ => false,
    }
}

pub open spec fn long_incremental_signatures_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalSignatures && !short
}

/// Identify exactly the long IncrementalSignatures route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_incremental_signatures_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_incremental_signatures_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalSignatures, false) => true,
        _ => false,
    }
}

pub open spec fn short_full_volume_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullVolume && short
}

/// Identify exactly the short FullVolume route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_full_volume_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_full_volume_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullVolume, true) => true,
        _ => false,
    }
}

pub open spec fn long_full_volume_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::FullVolume && !short
}

/// Identify exactly the long FullVolume route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_full_volume_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_full_volume_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::FullVolume, false) => true,
        _ => false,
    }
}

pub open spec fn short_incremental_volume_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalVolume && short
}

/// Identify exactly the short IncrementalVolume route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn short_incremental_volume_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == short_incremental_volume_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalVolume, true) => true,
        _ => false,
    }
}

pub open spec fn long_incremental_volume_route_model(kind: ObjectKind, short: bool) -> bool {
    kind == ObjectKind::IncrementalVolume && !short
}

/// Identify exactly the long IncrementalVolume route in the body dispatcher.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[allow(clippy::match_like_matches_macro)]
pub(crate) fn long_incremental_volume_route(kind: ObjectKind, short: bool) -> (route: bool)
    ensures route == long_incremental_volume_route_model(kind, short),
{
    match (kind, short) {
        (ObjectKind::IncrementalVolume, false) => true,
        _ => false,
    }
}

pub open spec fn body_route_count_model(kind: ObjectKind, short: bool) -> int {
    (if short_full_signatures_route_model(kind, short) { 1int } else { 0int })
        + (if long_full_signatures_route_model(kind, short) { 1int } else { 0int })
        + (if short_full_manifest_route_model(kind, short) { 1int } else { 0int })
        + (if long_full_manifest_route_model(kind, short) { 1int } else { 0int })
        + (if short_incremental_manifest_route_model(kind, short) { 1int } else { 0int })
        + (if long_incremental_manifest_route_model(kind, short) { 1int } else { 0int })
        + (if short_incremental_signatures_route_model(kind, short) { 1int } else { 0int })
        + (if long_incremental_signatures_route_model(kind, short) { 1int } else { 0int })
        + (if short_full_volume_route_model(kind, short) { 1int } else { 0int })
        + (if long_full_volume_route_model(kind, short) { 1int } else { 0int })
        + (if short_incremental_volume_route_model(kind, short) { 1int } else { 0int })
        + (if long_incremental_volume_route_model(kind, short) { 1int } else { 0int })
}

/// Prove the body dispatcher selects exactly one object-kind/profile route.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub(crate) fn body_route_partition(kind: ObjectKind, short: bool) -> (count: u8)
    ensures count == 1,
        count as int == body_route_count_model(kind, short),
{
    let count = short_full_signatures_route(kind, short) as u8
        + long_full_signatures_route(kind, short) as u8
        + short_full_manifest_route(kind, short) as u8
        + long_full_manifest_route(kind, short) as u8
        + short_incremental_manifest_route(kind, short) as u8
        + long_incremental_manifest_route(kind, short) as u8
        + short_incremental_signatures_route(kind, short) as u8
        + long_incremental_signatures_route(kind, short) as u8
        + short_full_volume_route(kind, short) as u8
        + long_full_volume_route(kind, short) as u8
        + short_incremental_volume_route(kind, short) as u8
        + long_incremental_volume_route(kind, short) as u8;
    proof {
        reveal(body_route_count_model);
        assert(count as int == body_route_count_model(kind, short));
        match (kind, short) {
            (ObjectKind::FullSignatures, true)
            | (ObjectKind::FullSignatures, false)
            | (ObjectKind::FullManifest, true)
            | (ObjectKind::FullManifest, false)
            | (ObjectKind::IncrementalManifest, true)
            | (ObjectKind::IncrementalManifest, false)
            | (ObjectKind::IncrementalSignatures, true)
            | (ObjectKind::IncrementalSignatures, false)
            | (ObjectKind::FullVolume, true)
            | (ObjectKind::FullVolume, false)
            | (ObjectKind::IncrementalVolume, true)
            | (ObjectKind::IncrementalVolume, false) => {
                assert(body_route_count_model(kind, short) == 1);
            }
        }
    }
    count
}
}

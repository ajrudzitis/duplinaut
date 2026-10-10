// Verus erases the fact activations in native builds, leaving an unused group
// declaration. This module contains proof/model definitions only.
#[cfg_attr(not(verus_keep_ghost), allow(dead_code))]
pub(crate) mod token_refinement;

//! Verified projection from the catalog manifest-index table to objects and descriptors.
use crate::chain::SetDescriptor;
use vstd::prelude::*;

verus! {

pub open spec fn manifest_tables_well_formed_model(
    object_count: usize,
    manifest_objects: Seq<usize>,
    descriptors: Seq<SetDescriptor>,
) -> bool {
    manifest_objects.len() == descriptors.len()
        && forall|i: int| 0 <= i < manifest_objects.len()
            ==> manifest_objects[i] < object_count
}

pub open spec fn manifest_position_model(
    object_count: usize,
    manifest_objects: Seq<usize>,
    descriptors: Seq<SetDescriptor>,
    position: usize,
) -> Option<(usize, usize, SetDescriptor)> {
    if manifest_tables_well_formed_model(object_count, manifest_objects, descriptors)
        && position < manifest_objects.len()
    {
        Some((position, manifest_objects[position as int], descriptors[position as int]))
    } else {
        None
    }
}

/// Validate the shape and object bounds once before the native iterator maps
/// positions. This is linear in the manifest table length.
pub fn manifest_tables_well_formed(
    object_count: usize,
    manifest_objects: &[usize],
    descriptors: &[SetDescriptor],
) -> (result: bool)
    ensures result == manifest_tables_well_formed_model(object_count, manifest_objects@, descriptors@),
{
    if manifest_objects.len() != descriptors.len() {
        return false;
    }
    let mut i = 0;
    while i < manifest_objects.len()
        invariant
            i <= manifest_objects.len(),
            manifest_objects.len() == descriptors.len(),
            forall|j: int| 0 <= j < i ==> manifest_objects@[j] < object_count,
        decreases manifest_objects.len() - i,
    {
        if manifest_objects[i] >= object_count {
            return false;
        }
        i += 1;
    }
    true
}

/// Return the descriptor-table position, object index, and exact descriptor
/// for an in-range position. The caller supplies the fact established by the
/// single table validation pass above.
pub fn manifest_position(
    _object_count: usize,
    manifest_objects: &[usize],
    descriptors: &[SetDescriptor],
    position: usize,
    tables_well_formed: bool,
) -> (result: Option<(usize, usize, SetDescriptor)>)
    requires tables_well_formed
        == manifest_tables_well_formed_model(_object_count, manifest_objects@, descriptors@),
    ensures result == manifest_position_model(_object_count, manifest_objects@, descriptors@, position),
{
    if !tables_well_formed || position >= manifest_objects.len() {
        return None;
    }
    Some((position, manifest_objects[position], descriptors[position]))
}

#[cfg(test)]
mod tests {
    use super::{manifest_position, manifest_tables_well_formed};
    use crate::chain::SetDescriptor;
    use crate::names::{parse_name, NameConfig};

    fn descriptor(name: &[u8]) -> SetDescriptor {
        SetDescriptor::from_manifest_name(
            parse_name(name, NameConfig::default()).unwrap().unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn manifest_position_checks_table_shape_bounds_and_position_edges() {
        let full = descriptor(b"duplicity-full.20231114T221320Z.manifest");
        let incremental = descriptor(
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        );
        let objects = [2usize, 1];
        let descriptors = [full, incremental];
        let valid = manifest_tables_well_formed(3, &objects, &descriptors);
        assert!(valid);
        assert_eq!(
            manifest_position(3, &objects, &descriptors, 0, valid),
            Some((0, 2, full))
        );
        assert_eq!(
            manifest_position(3, &objects, &descriptors, 1, valid),
            Some((1, 1, incremental))
        );
        assert_eq!(manifest_position(3, &objects, &descriptors, 2, valid), None);

        let short_descriptors = [full];
        let mismatched = manifest_tables_well_formed(3, &objects, &short_descriptors);
        assert!(!mismatched);
        assert_eq!(
            manifest_position(3, &objects, &short_descriptors, 0, mismatched),
            None
        );

        let out_of_bounds = [3usize];
        let descriptor = [full];
        let invalid_index = manifest_tables_well_formed(3, &out_of_bounds, &descriptor);
        assert!(!invalid_index);
        assert_eq!(
            manifest_position(3, &out_of_bounds, &descriptor, 0, invalid_index),
            None
        );
    }
}

} // verus!

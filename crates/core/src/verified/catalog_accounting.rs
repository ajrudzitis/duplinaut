//! Checked discovery metadata charging before catalog-owned allocations.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogMetadataCharge {
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogObjectMetadata {
    pub key_capacity: usize,
    pub etag_capacity: usize,
    pub version_capacity: usize,
    pub catalog_object_bytes: usize,
    pub descriptor_bytes: usize,
    pub index_bytes: usize,
}

pub open spec fn initialize_catalog_metadata_model(
    listed_capacity: usize,
    listed_entry_bytes: usize,
    catalog_overhead: usize,
    limit: usize,
) -> Option<CatalogMetadataCharge> {
    let listed_bytes = listed_capacity as int * listed_entry_bytes as int;
    let total = listed_bytes + catalog_overhead as int;
    if listed_bytes <= usize::MAX as int
        && total <= usize::MAX as int
        && total <= limit as int
    {
        Some(CatalogMetadataCharge { bytes: total as usize })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn initialize_catalog_metadata(
    listed_capacity: usize,
    listed_entry_bytes: usize,
    catalog_overhead: usize,
    limit: usize,
) -> (result: Option<CatalogMetadataCharge>)
    ensures result == initialize_catalog_metadata_model(
        listed_capacity,
        listed_entry_bytes,
        catalog_overhead,
        limit,
    ),
{
    let listed_bytes = listed_capacity.checked_mul(listed_entry_bytes)?;
    let bytes = listed_bytes.checked_add(catalog_overhead)?;
    if bytes > limit {
        return None;
    }
    Some(CatalogMetadataCharge { bytes })
}

pub open spec fn charge_catalog_object_metadata_model(
    current: CatalogMetadataCharge,
    object: CatalogObjectMetadata,
    limit: usize,
) -> Option<CatalogMetadataCharge> {
    let identities = object.etag_capacity as int + object.version_capacity as int;
    let object_total = object.key_capacity as int
        + identities
        + object.catalog_object_bytes as int
        + object.descriptor_bytes as int
        + object.index_bytes as int;
    let total = current.bytes as int + object_total;
    if current.bytes <= limit
        && identities <= usize::MAX as int
        && object_total <= usize::MAX as int
        && total <= usize::MAX as int
        && total <= limit as int
    {
        Some(CatalogMetadataCharge { bytes: total as usize })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn charge_catalog_object_metadata(
    current: CatalogMetadataCharge,
    object: CatalogObjectMetadata,
    limit: usize,
) -> (result: Option<CatalogMetadataCharge>)
    ensures result == charge_catalog_object_metadata_model(current, object, limit),
{
    if current.bytes > limit {
        return None;
    }
    let identities = object.etag_capacity.checked_add(object.version_capacity)?;
    let mut object_total = object.key_capacity.checked_add(identities)?;
    object_total = object_total.checked_add(object.catalog_object_bytes)?;
    object_total = object_total.checked_add(object.descriptor_bytes)?;
    object_total = object_total.checked_add(object.index_bytes)?;
    let bytes = current.bytes.checked_add(object_total)?;
    if bytes > limit {
        return None;
    }
    Some(CatalogMetadataCharge { bytes })
}

}

#[cfg(test)]
mod tests {
    use super::{
        charge_catalog_object_metadata, initialize_catalog_metadata, CatalogMetadataCharge,
        CatalogObjectMetadata,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn catalog_metadata_initial_charge_checks_exact_limit_and_overflow() {
        assert_eq!(
            initialize_catalog_metadata(3, 4, 5, 17),
            Some(CatalogMetadataCharge { bytes: 17 })
        );
        assert_eq!(initialize_catalog_metadata(3, 4, 5, 16), None);
        assert_eq!(
            initialize_catalog_metadata(usize::MAX, 2, 0, usize::MAX),
            None
        );
        assert_eq!(
            initialize_catalog_metadata(1, usize::MAX, 1, usize::MAX),
            None
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn catalog_metadata_object_charge_is_atomic_and_bounded() {
        let initial = CatalogMetadataCharge { bytes: 10 };
        assert_eq!(
            charge_catalog_object_metadata(
                initial,
                CatalogObjectMetadata {
                    key_capacity: 2,
                    etag_capacity: 3,
                    version_capacity: 4,
                    catalog_object_bytes: 5,
                    descriptor_bytes: 6,
                    index_bytes: 7,
                },
                37,
            ),
            Some(CatalogMetadataCharge { bytes: 37 })
        );
        assert_eq!(
            charge_catalog_object_metadata(
                initial,
                CatalogObjectMetadata {
                    key_capacity: 2,
                    etag_capacity: 3,
                    version_capacity: 4,
                    catalog_object_bytes: 5,
                    descriptor_bytes: 6,
                    index_bytes: 7,
                },
                36,
            ),
            None
        );
        assert_eq!(
            charge_catalog_object_metadata(
                initial,
                CatalogObjectMetadata {
                    key_capacity: 2,
                    etag_capacity: usize::MAX,
                    version_capacity: 1,
                    catalog_object_bytes: 0,
                    descriptor_bytes: 0,
                    index_bytes: 0,
                },
                usize::MAX,
            ),
            None
        );
        assert_eq!(
            charge_catalog_object_metadata(
                CatalogMetadataCharge { bytes: usize::MAX },
                CatalogObjectMetadata {
                    key_capacity: 0,
                    etag_capacity: 0,
                    version_capacity: 0,
                    catalog_object_bytes: 0,
                    descriptor_bytes: 0,
                    index_bytes: 1,
                },
                usize::MAX,
            ),
            None
        );
    }
}

use super::*;

fn object(key: &str) -> StoredObject {
    StoredObject {
        key: key.into(),
        size: 17,
        identity: crate::source::ObjectIdentity {
            version_id: Some("v1".into()),
            etag: Some("etag".into()),
        },
    }
}
fn decoded() -> openpgp::DecodedObject {
    openpgp::DecodedObject {
        bytes: Vec::new(),
        authorized_signer: Some(openpgp::AuthorizedSigner {
            primary_fingerprint: [7; 20],
            component_fingerprint: [8; 20],
        }),
        certificates: vec![OpenPgpCertificateMetadata {
            primary_fingerprint: [7; 20],
            created_seconds: 9,
            signatures: vec![
                crate::openpgp::CertificateSignatureMetadata {
                    created_seconds: Some(10),
                    key_expiry_seconds: Some(11),
                    signature_expiry_seconds: None,
                    revocation: false,
                },
                crate::openpgp::CertificateSignatureMetadata {
                    created_seconds: Some(12),
                    key_expiry_seconds: None,
                    signature_expiry_seconds: Some(13),
                    revocation: true,
                },
            ],
        }],
    }
}
fn limits() -> EvidenceLimits {
    EvidenceLimits {
        max_objects: 4,
        max_metadata_bytes: 4096,
        max_certificate_versions: 4,
        max_comparisons: 4,
    }
}
fn record(
    evidence: &mut AuthenticationEvidence,
    used: &mut usize,
    comparisons: &mut usize,
    item: &StoredObject,
    limits: EvidenceLimits,
) -> Result<(), ReaderError> {
    let decoded = decoded();
    record_with(evidence, used, comparisons, item, &decoded, limits)
}
fn record_with(
    evidence: &mut AuthenticationEvidence,
    used: &mut usize,
    comparisons: &mut usize,
    item: &StoredObject,
    decoded: &openpgp::DecodedObject,
    limits: EvidenceLimits,
) -> Result<(), ReaderError> {
    record_evidence(
        evidence,
        used,
        comparisons,
        item,
        Timestamp(1),
        Timestamp(2),
        false,
        1,
        Envelope::OpenPgp,
        decoded,
        limits,
    )
}

#[test]
//= docs/parsing-requirements.md#parse-env-022
//= type=test
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
//= docs/parsing-requirements.md#parse-env-023
//= type=test
//# Snapshot authentication evidence MUST distinguish integrity-only objects from configured signer authorization and preserve available complete signer fingerprints and certificate creation/expiry/revocation observations without implying current validity or Web-of-Trust acceptance.
fn evidence_limits_precede_retention_and_exact_certificate_versions_are_interned() {
    let item = object("backup/a.gpg");
    let mut evidence = AuthenticationEvidence::default();
    let (mut bytes, mut comparisons) = (0, 0);
    assert!(matches!(
        record(
            &mut evidence,
            &mut bytes,
            &mut comparisons,
            &item,
            EvidenceLimits {
                max_objects: 0,
                ..limits()
            }
        ),
        Err(ReaderError::Limit)
    ));
    assert!(matches!(
        record(
            &mut evidence,
            &mut bytes,
            &mut comparisons,
            &item,
            EvidenceLimits {
                max_certificate_versions: 0,
                ..limits()
            }
        ),
        Err(ReaderError::Limit)
    ));
    let source_charge = source_metadata_charge(&item).unwrap();
    assert!(matches!(
        record(
            &mut evidence,
            &mut bytes,
            &mut comparisons,
            &item,
            EvidenceLimits {
                max_metadata_bytes: source_charge,
                ..limits()
            }
        ),
        Err(ReaderError::Limit)
    ));

    record(&mut evidence, &mut bytes, &mut comparisons, &item, limits()).unwrap();
    let next = object("backup/b.gpg");
    let too_few_comparisons = comparisons + evidence.certificates.len() - 1;
    assert!(matches!(
        record(
            &mut evidence,
            &mut bytes,
            &mut comparisons,
            &next,
            EvidenceLimits {
                max_comparisons: too_few_comparisons,
                ..limits()
            }
        ),
        Err(ReaderError::Limit)
    ));
    record(&mut evidence, &mut bytes, &mut comparisons, &next, limits()).unwrap();
    assert_eq!(evidence.certificates.len(), 1);
    assert_eq!(evidence.objects.len(), 2);
    assert_eq!(
        evidence.objects[0].certificate_versions,
        evidence.objects[1].certificate_versions
    );
    assert_eq!(
        evidence.objects[0].kind,
        AuthenticationKind::OpenPgpAuthorized
    );
    let mut changed_version = decoded();
    changed_version.certificates[0].signatures.reverse();
    record_with(
        &mut evidence,
        &mut bytes,
        &mut comparisons,
        &object("backup/c.gpg"),
        &changed_version,
        limits(),
    )
    .unwrap();
    assert_eq!(evidence.certificates.len(), 2);
    assert_eq!(
        evidence.certificates[0].primary_fingerprint,
        evidence.certificates[1].primary_fingerprint
    );
    assert_ne!(
        evidence.objects[0].certificate_versions,
        evidence.objects[2].certificate_versions
    );
}

#[test]
//= docs/parsing-requirements.md#parse-gen-005
//= type=test
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=test
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
fn evidence_comparison_budget_is_charged_before_certificate_lookup() {
    let item = object("backup/compare.gpg");
    let decoded = decoded();
    let certificate = decoded.certificates[0].clone();

    let mut accepted = AuthenticationEvidence::default();
    accepted.certificates.push(certificate.clone());
    let (mut bytes, mut comparisons) = (0, 0);
    record_with(
        &mut accepted,
        &mut bytes,
        &mut comparisons,
        &item,
        &decoded,
        EvidenceLimits {
            max_comparisons: 1,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(comparisons, 1);
    assert_eq!(accepted.objects[0].certificate_versions, [0]);

    let mut rejected = AuthenticationEvidence::default();
    rejected.certificates.push(certificate);
    let (mut bytes, mut comparisons) = (0, 0);
    assert!(matches!(
        record_with(
            &mut rejected,
            &mut bytes,
            &mut comparisons,
            &item,
            &decoded,
            EvidenceLimits {
                max_comparisons: 0,
                ..limits()
            },
        ),
        Err(ReaderError::Limit)
    ));
    assert_eq!(comparisons, 0);
    assert!(rejected.objects.is_empty());
}

#[test]
//= docs/parsing-requirements.md#parse-gen-005
//= type=test
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=test
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
fn evidence_record_metadata_reservation_precedes_plain_and_crypto_retention() {
    let item = object("backup/metadata.gpg");
    let source_bytes = source_metadata_charge(&item).unwrap();

    let mut plain = AuthenticationEvidence::default();
    let mut plain_used = 0;
    record_plain(
        &mut plain,
        &mut plain_used,
        &item,
        Timestamp(1),
        Timestamp(2),
        false,
        1,
        Envelope::Plain,
        Some(EvidenceLimits {
            max_metadata_bytes: source_bytes,
            ..limits()
        }),
    )
    .unwrap();
    assert_eq!(plain_used, source_bytes);
    assert_eq!(plain.objects.len(), 1);

    let mut rejected_plain = AuthenticationEvidence::default();
    let mut rejected_plain_used = 0;
    assert!(matches!(
        record_plain(
            &mut rejected_plain,
            &mut rejected_plain_used,
            &item,
            Timestamp(1),
            Timestamp(2),
            false,
            1,
            Envelope::Plain,
            Some(EvidenceLimits {
                max_metadata_bytes: source_bytes - 1,
                ..limits()
            }),
        ),
        Err(ReaderError::Limit)
    ));
    assert_eq!(rejected_plain_used, 0);
    assert!(rejected_plain.objects.is_empty());

    let decoded = decoded();
    let index_bytes = decoded.certificates.len() * std::mem::size_of::<usize>();
    let record_bytes = source_bytes + index_bytes;
    let mut crypto = AuthenticationEvidence {
        certificates: decoded.certificates.clone(),
        ..AuthenticationEvidence::default()
    };
    let (mut crypto_used, mut comparisons) = (0, 0);
    record_with(
        &mut crypto,
        &mut crypto_used,
        &mut comparisons,
        &item,
        &decoded,
        EvidenceLimits {
            max_metadata_bytes: record_bytes,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(crypto_used, record_bytes);
    assert_eq!(crypto.objects.len(), 1);

    let mut rejected_crypto = AuthenticationEvidence {
        certificates: decoded.certificates.clone(),
        ..AuthenticationEvidence::default()
    };
    let (mut rejected_crypto_used, mut rejected_comparisons) = (0, 0);
    assert!(matches!(
        record_with(
            &mut rejected_crypto,
            &mut rejected_crypto_used,
            &mut rejected_comparisons,
            &item,
            &decoded,
            EvidenceLimits {
                max_metadata_bytes: record_bytes - 1,
                ..limits()
            },
        ),
        Err(ReaderError::Limit)
    ));
    assert_eq!(rejected_crypto_used, 0);
    assert_eq!(rejected_comparisons, 0);
    assert!(rejected_crypto.objects.is_empty());
}

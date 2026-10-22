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

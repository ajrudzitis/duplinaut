use duplicity_core::{
    names::{NameConfig, Timestamp},
    reader::{
        Catalog, CryptoCredentials, CryptoReadContext, DiscoveryStatus, EvidenceLimits,
        OpenPgpLimits, OpenPgpPassword, ReaderError, ReaderLimits, SecretExport,
    },
    source::{Cancellation, ObjectIdentity, ObjectSource, SourceError, SourceLimits, StoredObject},
};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

struct FixtureSource {
    values: Vec<(StoredObject, Vec<u8>)>,
    opens: AtomicUsize,
}
impl FixtureSource {
    fn new(mode: &str, tamper_key: Option<&str>) -> Self {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7")
            .join(mode);
        let mut values = Vec::new();
        for item in std::fs::read_dir(directory).unwrap() {
            let item = item.unwrap();
            let key = format!("backup/{}", item.file_name().to_string_lossy());
            let mut bytes = std::fs::read(item.path()).unwrap();
            if tamper_key.is_some_and(|fragment| key.contains(fragment)) {
                let offset = bytes.len() / 2;
                bytes[offset] ^= 1;
            }
            values.push((
                StoredObject {
                    key,
                    size: bytes.len() as u64,
                    identity: ObjectIdentity {
                        version_id: Some("fixture-v1".into()),
                        etag: Some(format!("{:x}", Sha256::digest(&bytes))),
                    },
                },
                bytes,
            ));
        }
        Self {
            values,
            opens: AtomicUsize::new(0),
        }
    }
}
impl ObjectSource for FixtureSource {
    fn list(
        &self,
        prefix: &str,
        _: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError> {
        if cancel.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        Ok(self
            .values
            .iter()
            .filter(|(item, _)| item.key.starts_with(prefix))
            .map(|(item, _)| item.clone())
            .collect())
    }
    fn open(
        &self,
        object: &StoredObject,
        _: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        if cancel.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        let (expected, bytes) = self
            .values
            .iter()
            .find(|(item, _)| item.key == object.key)
            .ok_or(SourceError::Missing)?;
        if expected != object {
            return Err(SourceError::Changed);
        }
        Ok(Box::new(Cursor::new(bytes.clone())))
    }
}
#[test]
//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-018
//= type=test
//# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
//= docs/parsing-requirements.md#parse-env-021
//= type=test
//# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
//= docs/parsing-requirements.md#parse-env-023
//= type=test
//# Snapshot authentication evidence MUST distinguish integrity-only objects from configured signer authorization and preserve available complete signer fingerprints and certificate creation/expiry/revocation observations without implying current validity or Web-of-Trust acceptance.
fn original_encrypted_chains_reconstruct_each_official_date_with_bounded_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
    let secret = std::fs::read(root.join("test-keys/secret.asc")).unwrap();
    let public = std::fs::read(root.join("test-keys/public.asc")).unwrap();
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let oracle: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("expected.json")).unwrap()).unwrap();
    for mode in ["symmetric", "public-key", "signed"] {
        let source = FixtureSource::new(mode, None);
        let catalog = Catalog::discover(
            &source,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(catalog
            .objects()
            .iter()
            .all(|object| object.status() == DiscoveryStatus::Completed));
        let anchor = catalog
            .manifests()
            .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
            .unwrap()
            .0;
        let secret_exports = [SecretExport {
            bytes: &secret,
            password: Some(&password),
        }];
        let signer_exports = [public.as_slice()];
        let credentials = CryptoCredentials {
            message_passwords: if mode == "symmetric" {
                std::slice::from_ref(&password)
            } else {
                &[]
            },
            secret_exports: if mode == "symmetric" {
                &[]
            } else {
                &secret_exports
            },
            signer_exports: if mode == "signed" {
                &signer_exports
            } else {
                &[]
            },
            permitted_signers: if mode == "signed" {
                &[[
                    0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f,
                    0x23, 0x17, 0x3b, 0xad, 0x8a, 0x76, 0x56,
                ]]
            } else {
                &[]
            },
            require_signature: mode == "signed",
        };
        let context = CryptoReadContext {
            credentials,
            crypto_limits: OpenPgpLimits::default(),
            evidence_limits: EvidenceLimits::default(),
        };
        for day in 0..3 {
            let snapshot = catalog
                .read_snapshot_with_crypto(
                    anchor,
                    Timestamp(1700000000 + day * 86400),
                    &Cancellation::default(),
                    &context,
                )
                .unwrap();
            let expected = oracle["snapshots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["mode"] == mode && item["index"] == day as u64)
                .unwrap()["entries"]
                .as_array()
                .unwrap();
            assert_eq!(snapshot.state().len(), expected.len(), "{mode}/{day}");
            let expected_files: std::collections::BTreeMap<_, _> = expected
                .iter()
                .filter(|entry| entry["kind"] == "file")
                .map(|entry| {
                    (
                        entry["path_hex"].as_str().unwrap().to_owned(),
                        entry["sha256"].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            let actual_files: std::collections::BTreeMap<_, _> = snapshot
                .state()
                .entries()
                .filter_map(|(path, entry)| {
                    let content = entry.content()?;
                    let raw = path
                        .components()
                        .iter()
                        .map(|part| part.as_slice())
                        .collect::<Vec<_>>()
                        .join(&b'/');
                    Some((
                        raw.iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>(),
                        format!("{:x}", Sha256::digest(content)),
                    ))
                })
                .collect();
            assert_eq!(actual_files, expected_files, "{mode}/{day} source oracle");
            let evidence = snapshot.authentication_evidence();
            assert!(!evidence.objects().is_empty());
            let policy = evidence.policy().expect("recorded per-read policy");
            assert_eq!(policy.authorization_configured, mode == "signed");
            assert_eq!(policy.require_signature, mode == "signed");
            assert_eq!(
                policy.permitted_primary_fingerprints.len(),
                usize::from(mode == "signed")
            );
            assert!(evidence.objects().iter().all(|record| record.kind
                == if mode == "signed" {
                    duplicity_core::reader::AuthenticationKind::OpenPgpAuthorized
                } else {
                    duplicity_core::reader::AuthenticationKind::OpenPgpIntegrity
                }));
            if mode == "signed" {
                assert!(evidence.objects().iter().all(|record| {
                    record.authorized_signer.as_ref().is_some_and(|signer| {
                        signer.primary_fingerprint
                            == [
                                0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b,
                                0xda, 0x6f, 0x23, 0x17, 0x3b, 0xad, 0x8a, 0x76, 0x56,
                            ]
                    }) && !record.certificate_versions.is_empty()
                }));
            } else {
                assert!(evidence
                    .objects()
                    .iter()
                    .all(|record| record.authorized_signer.is_none()));
            }
            assert!(evidence
                .objects()
                .iter()
                .all(|record| record.identity.version_id.as_deref() == Some("fixture-v1")));
        }
    }
}

#[test]
//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
fn corrupted_ciphertext_fails_declared_digest_before_missing_credentials() {
    let source = FixtureSource::new(
        "symmetric",
        Some("duplicity-full.20231114T221320Z.vol1.difftar.gpg"),
    );
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .unwrap()
        .0;
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let context = CryptoReadContext {
        credentials: CryptoCredentials {
            message_passwords: std::slice::from_ref(&password),
            ..CryptoCredentials::default()
        },
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits::default(),
    };
    let error = match catalog.read_snapshot_with_crypto(
        anchor,
        Timestamp(1700000000),
        &Cancellation::default(),
        &context,
    ) {
        Ok(_) => panic!("tampered volume accepted"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            ReaderError::Assembly(duplicity_core::multipart::AssemblyError::Envelope(
                duplicity_core::envelope::EnvelopeError::DigestMismatch
            ))
        ),
        "{error:?}"
    );
}

#[test]
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn late_incremental_integrity_failure_returns_no_snapshot() {
    let source = FixtureSource::new(
        "symmetric",
        Some("20231115T221320Z.to.20231116T221320Z.vol1.difftar.gpg"),
    );
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .unwrap()
        .0;
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let context = CryptoReadContext {
        credentials: CryptoCredentials {
            message_passwords: std::slice::from_ref(&password),
            ..CryptoCredentials::default()
        },
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits::default(),
    };
    let result = catalog.read_snapshot_with_crypto(
        anchor,
        Timestamp(1700172800),
        &Cancellation::default(),
        &context,
    );
    assert!(matches!(
        result,
        Err(ReaderError::Assembly(
            duplicity_core::multipart::AssemblyError::Envelope(
                duplicity_core::envelope::EnvelopeError::DigestMismatch
            )
        ))
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-env-019
//= type=test
//# A snapshot read with signer authorization configured MUST require that authorization for every required manifest and data volume throughout the selected full/incremental chain, rejecting unsigned or plain/gzip required objects without policy downgrade.
//= docs/parsing-requirements.md#parse-env-022
//= type=test
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
//= docs/parsing-requirements.md#parse-env-023
//= type=test
//# Snapshot authentication evidence MUST distinguish integrity-only objects from configured signer authorization and preserve available complete signer fingerprints and certificate creation/expiry/revocation observations without implying current validity or Web-of-Trust acceptance.
fn global_signature_policy_and_evidence_limits_fail_closed() {
    let plain_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain");
    let source = FixtureSource {
        values: std::fs::read_dir(plain_root)
            .unwrap()
            .map(|item| {
                let item = item.unwrap();
                let key = format!("backup/{}", item.file_name().to_string_lossy());
                let bytes = std::fs::read(item.path()).unwrap();
                (
                    StoredObject {
                        key,
                        size: bytes.len() as u64,
                        identity: ObjectIdentity {
                            version_id: None,
                            etag: None,
                        },
                    },
                    bytes,
                )
            })
            .collect(),
        opens: AtomicUsize::new(0),
    };
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .unwrap()
        .0;
    let credentials = CryptoCredentials {
        require_signature: true,
        ..CryptoCredentials::default()
    };
    let context = CryptoReadContext {
        credentials,
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits::default(),
    };
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context
        ),
        Err(ReaderError::Crypto(
            duplicity_core::reader::OpenPgpError::Unsigned
        ))
    ));
    let limited = CryptoReadContext {
        credentials: CryptoCredentials::default(),
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits {
            max_objects: 0,
            ..EvidenceLimits::default()
        },
    };
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &limited
        ),
        Err(ReaderError::Limit)
    ));
}

#[test]
fn ciphertext_limit_is_checked_before_object_open() {
    let source = FixtureSource::new("symmetric", None);
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .unwrap()
        .0;
    let context = CryptoReadContext {
        credentials: CryptoCredentials::default(),
        crypto_limits: OpenPgpLimits {
            stored_bytes: 0,
            ..OpenPgpLimits::default()
        },
        evidence_limits: EvidenceLimits::default(),
    };
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context
        ),
        Err(ReaderError::Limit)
    ));
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);

    let source = FixtureSource::new("symmetric", None);
    let reader_limits = ReaderLimits {
        envelope: duplicity_core::envelope::EnvelopeLimits {
            stored_bytes: 0,
            ..duplicity_core::envelope::EnvelopeLimits::default()
        },
        ..ReaderLimits::default()
    };
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        reader_limits,
        &Cancellation::default(),
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .unwrap()
        .0;
    let context = CryptoReadContext {
        credentials: CryptoCredentials::default(),
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits::default(),
    };
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context
        ),
        Err(ReaderError::Limit)
    ));
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
}

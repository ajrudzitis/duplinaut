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
    open_behavior: Option<(String, OpenBehavior)>,
}
#[derive(Clone, Copy)]
enum OpenBehavior {
    IdentityChanged,
    Truncate,
    Excess,
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
            open_behavior: None,
        }
    }
    fn with_open_behavior(mut self, fragment: &str, behavior: OpenBehavior) -> Self {
        self.open_behavior = Some((fragment.to_owned(), behavior));
        self
    }
    fn signed_full_plain_incrementals() -> Self {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
        let mut values = Vec::new();
        for (mode, predicate) in [("signed", "duplicity-full."), ("plain", "duplicity-inc.")] {
            for item in std::fs::read_dir(root.join(mode)).unwrap() {
                let item = item.unwrap();
                let filename = item.file_name().to_string_lossy().into_owned();
                if !filename.starts_with(predicate)
                    || filename.contains("signatures")
                    || !filename.contains("manifest") && !filename.contains("vol")
                {
                    continue;
                }
                let key = format!("backup/{filename}");
                let bytes = std::fs::read(item.path()).unwrap();
                values.push((
                    StoredObject {
                        key,
                        size: bytes.len() as u64,
                        identity: ObjectIdentity {
                            version_id: Some(format!("{mode}-v1")),
                            etag: Some(format!("{:x}", Sha256::digest(&bytes))),
                        },
                    },
                    bytes,
                ));
            }
        }
        Self {
            values,
            opens: AtomicUsize::new(0),
            open_behavior: None,
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
        if let Some((fragment, behavior)) = &self.open_behavior {
            if object.key.contains(fragment) {
                match behavior {
                    OpenBehavior::IdentityChanged => return Err(SourceError::Changed),
                    OpenBehavior::Truncate => {
                        return Ok(Box::new(Cursor::new(bytes[..bytes.len() - 1].to_vec())))
                    }
                    OpenBehavior::Excess => {
                        let mut excess = bytes.clone();
                        excess.push(0);
                        return Ok(Box::new(Cursor::new(excess)));
                    }
                }
            }
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
            assert_eq!(policy.crypto_limits.stored_bytes, 256 * 1024 * 1024);
            assert_eq!(policy.crypto_limits.authenticated_bytes, 256 * 1024 * 1024);
            assert_eq!(policy.crypto_limits.decoded_bytes, 8 * 1024 * 1024 * 1024);
            assert_eq!(policy.crypto_limits.expanded_bytes, 8 * 1024 * 1024 * 1024);
            assert_eq!(policy.evidence_limits.max_objects, 1_000_000);
            assert_eq!(policy.evidence_limits.max_metadata_bytes, 32 * 1024 * 1024);
            assert_eq!(policy.evidence_limits.max_certificate_versions, 64);
            assert_eq!(policy.evidence_limits.max_comparisons, 16_000_000);
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
                assert_eq!(
                    evidence.certificates().len(),
                    1,
                    "exact versions are interned"
                );
                let certificate = &evidence.certificates()[0];
                assert_eq!(
                    certificate.primary_fingerprint,
                    [
                        0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda,
                        0x6f, 0x23, 0x17, 0x3b, 0xad, 0x8a, 0x76, 0x56,
                    ]
                );
                assert!(certificate.created_seconds > 1_600_000_000);
                assert!(!certificate.signatures.is_empty());
                assert!(certificate
                    .signatures
                    .iter()
                    .all(|signature| signature.created_seconds.is_some()));
                // The official fixture certificate has no expiration or revocation
                // observations; assert those explicit absences are retained too.
                assert!(certificate
                    .signatures
                    .iter()
                    .all(|signature| signature.key_expiry_seconds.is_none()));
                assert!(certificate
                    .signatures
                    .iter()
                    .all(|signature| signature.signature_expiry_seconds.is_none()));
                assert!(certificate
                    .signatures
                    .iter()
                    .all(|signature| !signature.revocation));
                assert!(evidence.objects().iter().all(|record| {
                    record.authorized_signer.as_ref().is_some_and(|signer| {
                        signer.primary_fingerprint
                            == [
                                0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b,
                                0xda, 0x6f, 0x23, 0x17, 0x3b, 0xad, 0x8a, 0x76, 0x56,
                            ]
                    }) && record.certificate_versions == [0]
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
            assert!(evidence.objects().iter().all(|record| {
                record.key.starts_with("backup/")
                    && record.set_start <= record.set_end
                    && (record.manifest || record.ordinal > 0)
            }));
            assert!(evidence.objects().iter().all(|record| record
                .certificate_versions
                .iter()
                .all(|index| *index < evidence.certificates().len())));
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
//= docs/parsing-requirements.md#parse-env-018
//= type=test
//# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
//= docs/parsing-requirements.md#parse-env-021
//= type=test
//# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn public_crypto_reader_rejects_wrong_symmetric_credentials_without_snapshot_or_secret_disclosure()
{
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
    let wrong_secret = "duplinaut-intentionally-wrong-fixture-password";
    let password = OpenPgpPassword::from(wrong_secret);
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
        Ok(_) => panic!("reader accepted a wrong symmetric credential"),
        Err(ReaderError::Crypto(error)) => error,
        Err(error) => panic!("wrong credential returned an unexpected error: {error:?}"),
    };
    assert!(matches!(
        error,
        duplicity_core::reader::OpenPgpError::WrongCredentials
            | duplicity_core::reader::OpenPgpError::AuthenticationFailed
    ));
    assert!(!format!("{error:?}").contains(wrong_secret));
    assert!(!error.to_string().contains(wrong_secret));
}

#[test]
//= docs/parsing-requirements.md#parse-env-018
//= type=test
//# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
//= docs/parsing-requirements.md#parse-env-019
//= type=test
//# A snapshot read with signer authorization configured MUST require that authorization for every required manifest and data volume throughout the selected full/incremental chain, rejecting unsigned or plain/gzip required objects without policy downgrade.
//= docs/parsing-requirements.md#parse-env-021
//= type=test
//# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn public_crypto_reader_rejects_valid_signature_from_disallowed_primary_signer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
    let source = FixtureSource::new("signed", None);
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

    // Supply the actual fixture recipient secret and its password, plus the
    // correct signer certificate. Only the authorization allowlist is wrong.
    let secret = std::fs::read(root.join("test-keys/secret.asc")).unwrap();
    let signer = std::fs::read(root.join("test-keys/public.asc")).unwrap();
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let secret_exports = [SecretExport {
        bytes: &secret,
        password: Some(&password),
    }];
    let signer_exports = [signer.as_slice()];
    let disallowed_signer = [[0x55; 20]];
    let context = CryptoReadContext {
        credentials: CryptoCredentials {
            secret_exports: &secret_exports,
            signer_exports: &signer_exports,
            permitted_signers: &disallowed_signer,
            require_signature: true,
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
        Ok(_) => panic!("reader accepted a valid signature from a disallowed signer"),
        Err(ReaderError::Crypto(error)) => error,
        Err(error) => panic!("wrong signer returned an unexpected error: {error:?}"),
    };
    assert_eq!(
        error,
        duplicity_core::reader::OpenPgpError::SignerNotAllowed
    );
    assert!(!format!("{error:?}").contains("duplicity-reader-public-test-fixture"));
    assert!(!error
        .to_string()
        .contains("duplicity-reader-public-test-fixture"));
}

#[test]
//= docs/parsing-requirements.md#parse-env-018
//= type=test
//# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn public_crypto_reader_rejects_authenticated_manifest_ciphertext_mutation() {
    let manifest_name = "duplicity-full.20231114T221320Z.manifest.gpg";
    // Mutate the stored OpenPGP MDC byte before discovery. This preserves the
    // filename and object length, so catalog structure remains valid and there
    // is no data-volume digest that could mask native message authentication.
    let mut source = FixtureSource::new("symmetric", None);
    let (_, bytes) = source
        .values
        .iter_mut()
        .find(|(object, _)| object.key.ends_with(manifest_name))
        .expect("official full manifest fixture");
    *bytes.last_mut().expect("nonempty OpenPGP fixture") ^= 0x01;
    let object = source
        .values
        .iter_mut()
        .find(|(object, _)| object.key.ends_with(manifest_name))
        .unwrap();
    object.0.identity.etag = Some(format!("{:x}", Sha256::digest(&object.1)));

    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .expect("same-size ciphertext mutation does not alter catalog listing");
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .expect("full manifest remains discoverable")
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
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context,
        ),
        Err(ReaderError::Crypto(
            duplicity_core::reader::OpenPgpError::AuthenticationFailed
        ))
    ));
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
        open_behavior: None,
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

#[test]
//= docs/parsing-requirements.md#parse-source-001
//= type=test
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn crypto_reader_rejects_source_identity_change_and_inexact_manifest_bodies() {
    for (behavior, expected_kind) in [
        (OpenBehavior::IdentityChanged, None),
        (
            OpenBehavior::Truncate,
            Some(std::io::ErrorKind::UnexpectedEof),
        ),
        (OpenBehavior::Excess, Some(std::io::ErrorKind::InvalidData)),
    ] {
        let source = FixtureSource::new("symmetric", None)
            .with_open_behavior("duplicity-full.20231114T221320Z.manifest.gpg", behavior);
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
            Timestamp(1700000000),
            &Cancellation::default(),
            &context,
        );
        match expected_kind {
            None => assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Changed))
            )),
            Some(kind) => match result {
                Err(ReaderError::Source(SourceError::Io(error))) => {
                    assert_eq!(error.kind(), kind)
                }
                _ => panic!("inexact source body unexpectedly succeeded"),
            },
        }
        assert_eq!(source.opens.load(Ordering::SeqCst), 1);
    }
}

#[test]
//= docs/parsing-requirements.md#parse-env-019
//= type=test
//# A snapshot read with signer authorization configured MUST require that authorization for every required manifest and data volume throughout the selected full/incremental chain, rejecting unsigned or plain/gzip required objects without policy downgrade.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
fn signer_policy_rejects_plain_incremental_after_signed_encrypted_full() {
    let source = FixtureSource::signed_full_plain_incrementals();
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/test-keys");
    let public = std::fs::read(root.join("public.asc")).unwrap();
    let secret = std::fs::read(root.join("secret.asc")).unwrap();
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let secret_exports = [SecretExport {
        bytes: &secret,
        password: Some(&password),
    }];
    let signer_exports = [public.as_slice()];
    let permitted_signers = [[
        0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23, 0x17,
        0x3b, 0xad, 0x8a, 0x76, 0x56,
    ]];
    let context = CryptoReadContext {
        credentials: CryptoCredentials {
            secret_exports: &secret_exports,
            signer_exports: &signer_exports,
            permitted_signers: &permitted_signers,
            require_signature: true,
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
    match result {
        Err(error) => assert!(
            matches!(
                error,
                ReaderError::Crypto(duplicity_core::reader::OpenPgpError::Unsigned)
            ),
            "mixed-chain signer policy failed as {error:?}"
        ),
        Ok(_) => panic!("mixed-chain signer policy returned a snapshot"),
    }
}

#[test]
//= docs/parsing-requirements.md#parse-env-018
//= type=test
//# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
//= docs/parsing-requirements.md#parse-env-022
//= type=test
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
fn crypto_decoded_and_each_receipt_bound_fail_through_catalog_reader() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
    let secret = std::fs::read(root.join("test-keys/secret.asc")).unwrap();
    let public = std::fs::read(root.join("test-keys/public.asc")).unwrap();
    let password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let secret_exports = [SecretExport {
        bytes: &secret,
        password: Some(&password),
    }];
    let signer_exports = [public.as_slice()];
    let permitted_signers = [[
        0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23, 0x17,
        0x3b, 0xad, 0x8a, 0x76, 0x56,
    ]];
    let cases = [
        ("symmetric", false, 0, 1_000_000, 64, 16_000_000, "decoded"),
        ("symmetric", false, usize::MAX, 1, 64, 16_000_000, "objects"),
        (
            "signed",
            true,
            usize::MAX,
            1_000_000,
            0,
            16_000_000,
            "versions",
        ),
        ("signed", true, usize::MAX, 1_000_000, 64, 0, "comparisons"),
    ];
    for (mode, signed, decoded_bytes, max_objects, versions, comparisons, expected) in cases {
        let source = FixtureSource::new(mode, None);
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
        let evidence_limits = EvidenceLimits {
            max_objects,
            max_metadata_bytes: usize::MAX,
            max_certificate_versions: versions,
            max_comparisons: comparisons,
        };
        let context = CryptoReadContext {
            credentials: if signed {
                CryptoCredentials {
                    secret_exports: &secret_exports,
                    signer_exports: &signer_exports,
                    permitted_signers: &permitted_signers,
                    require_signature: true,
                    ..CryptoCredentials::default()
                }
            } else {
                CryptoCredentials {
                    message_passwords: std::slice::from_ref(&password),
                    ..CryptoCredentials::default()
                }
            },
            crypto_limits: OpenPgpLimits {
                decoded_bytes,
                ..OpenPgpLimits::default()
            },
            evidence_limits,
        };
        let result = catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context,
        );
        match (expected, result) {
            ("decoded", Err(ReaderError::Crypto(error))) => {
                assert_eq!(format!("{error:?}"), "Limit(Decoded)")
            }
            ("objects" | "versions" | "comparisons", Err(ReaderError::Limit)) => {}
            (_, Err(error)) => panic!("{expected} bound returned {error:?}"),
            (_, Ok(_)) => panic!("{expected} bound returned a completed snapshot"),
        }
    }

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
        credentials: CryptoCredentials {
            message_passwords: std::slice::from_ref(&password),
            ..CryptoCredentials::default()
        },
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: EvidenceLimits {
            max_metadata_bytes: std::mem::size_of::<
                duplicity_core::reader::AuthenticationPolicySummary,
            >(),
            ..EvidenceLimits::default()
        },
    };
    assert!(matches!(
        catalog.read_snapshot_with_crypto(
            anchor,
            Timestamp(1700000000),
            &Cancellation::default(),
            &context,
        ),
        Err(ReaderError::Limit)
    ));
}

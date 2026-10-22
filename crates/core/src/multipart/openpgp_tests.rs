use super::*;
use crate::manifest::ManifestLimits;
use crate::openpgp::{Credentials, CryptoLimits};
use pgp::types::Password;
use std::{
    io::{Cursor, Read},
    path::Path,
};

struct OfficialSymmetricSource(Vec<(crate::source::StoredObject, Vec<u8>)>);

impl crate::source::ObjectSource for OfficialSymmetricSource {
    fn list(
        &self,
        prefix: &str,
        _: crate::source::SourceLimits,
        cancel: &crate::source::Cancellation,
    ) -> Result<Vec<crate::source::StoredObject>, crate::source::SourceError> {
        if cancel.is_cancelled() {
            return Err(crate::source::SourceError::Cancelled);
        }
        Ok(self
            .0
            .iter()
            .filter(|(object, _)| object.key.starts_with(prefix))
            .map(|(object, _)| object.clone())
            .collect())
    }

    fn open(
        &self,
        object: &crate::source::StoredObject,
        _: crate::source::SourceLimits,
        cancel: &crate::source::Cancellation,
    ) -> Result<Box<dyn Read + Send>, crate::source::SourceError> {
        if cancel.is_cancelled() {
            return Err(crate::source::SourceError::Cancelled);
        }
        let (_, bytes) = self
            .0
            .iter()
            .find(|(candidate, _)| candidate == object)
            .ok_or(crate::source::SourceError::Missing)?;
        Ok(Box::new(Cursor::new(bytes.clone())))
    }
}

fn official_symmetric_source() -> OfficialSymmetricSource {
    use sha2::Digest;

    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
    let objects = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let bytes = std::fs::read(entry.path()).unwrap();
            let key = format!("backup/{}", entry.file_name().to_string_lossy());
            let identity = crate::source::ObjectIdentity {
                version_id: Some("official-fixture-v1".into()),
                etag: Some(format!("{:x}", sha2::Sha256::digest(&bytes))),
            };
            (
                crate::source::StoredObject {
                    key,
                    size: bytes.len() as u64,
                    identity,
                },
                bytes,
            )
        })
        .collect();
    OfficialSymmetricSource(objects)
}

fn official_symmetric_catalog<'a>(
    source: &'a OfficialSymmetricSource,
) -> crate::reader::Catalog<'a> {
    crate::reader::Catalog::discover(
        source,
        "backup/",
        crate::names::NameConfig::default(),
        crate::reader::ReaderLimits::default(),
        &crate::source::Cancellation::default(),
    )
    .unwrap()
}

fn read_official_symmetric_with_cancel_hook(
    checkpoint: super::openpgp_cancel_test_hook::Checkpoint,
) -> (
    Result<crate::reader::ValidatedSnapshot, crate::reader::ReaderError>,
    usize,
) {
    let source = official_symmetric_source();
    let catalog = official_symmetric_catalog(&source);
    let (anchor, _, _) = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == crate::chain::SetKind::Full)
        .unwrap();
    let password = crate::reader::OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let context = crate::reader::CryptoReadContext {
        credentials: crate::reader::CryptoCredentials {
            message_passwords: std::slice::from_ref(&password),
            ..crate::reader::CryptoCredentials::default()
        },
        crypto_limits: crate::reader::OpenPgpLimits::default(),
        evidence_limits: crate::reader::EvidenceLimits::default(),
    };
    let cancellation = crate::source::Cancellation::default();
    let guard = super::openpgp_cancel_test_hook::Guard::cancel_at(checkpoint);
    let result = catalog.read_snapshot_with_crypto(
        anchor,
        crate::names::Timestamp(1_700_000_000),
        &cancellation,
        &context,
    );
    (result, guard.hits())
}

fn digest_permit<'a>(
    cursor: &mut crate::verified::openpgp_admission::OpenPgpAdmission,
    position: crate::verified::openpgp_admission::Position,
    stored: &'a [u8],
) -> crate::verified::openpgp_admission::DigestPermit<'a> {
    use crate::manifest::{DeclaredDigest, DigestAlgorithm};
    use sha1::Digest;

    let declaration = DeclaredDigest {
        algorithm: DigestAlgorithm::Sha1,
        name: b"SHA1".to_vec(),
        hex: sha1::Sha1::digest(stored)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into_bytes(),
    };
    cursor
        .verify_stored_digest(position, stored, &[declaration], stored.len())
        .unwrap()
}

//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn admission_cursor_advances_only_after_digest_authentication_and_tar_stages() {
    let stored = b"native digest gate test object";
    let mut cursor = crate::verified::openpgp_admission::OpenPgpAdmission::new(2);
    let first = cursor.position().unwrap();
    assert_eq!(first.ordinal, 0);
    assert!(!first.final_volume);
    let digest = digest_permit(&mut cursor, first, stored);
    assert_eq!(cursor.ordinal(), 0);
    let authenticated = cursor.authenticated(digest);
    assert_eq!(cursor.ordinal(), 0);
    let validated = cursor.tar_validated(authenticated);
    assert_eq!(cursor.ordinal(), 0);
    assert!(cursor.commit_openpgp(validated));
    let second = cursor.position().unwrap();
    assert_eq!(second.ordinal, 1);
    assert!(second.final_volume);
    let digest = digest_permit(&mut cursor, second, stored);
    let authenticated = cursor.authenticated(digest);
    let validated = cursor.tar_validated(authenticated);
    assert!(cursor.commit_openpgp(validated));
    assert!(cursor.is_complete());
}

#[test]
fn rejected_authenticated_stage_keeps_the_normalized_ordinal() {
    let stored = b"native digest gate test object";
    let mut cursor = crate::verified::openpgp_admission::OpenPgpAdmission::new(2);
    let position = cursor.position().unwrap();
    let digest = digest_permit(&mut cursor, position, stored);
    let _authenticated = cursor.authenticated(digest);
    cursor.reject();
    assert_eq!(cursor.ordinal(), 0);
    assert!(cursor.position().is_none());
}

//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
#[test]
fn digest_failure_and_precancel_do_not_advance_admission() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
    let stored_manifest =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.manifest.gpg")).unwrap();
    let stored_volume =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.vol1.difftar.gpg")).unwrap();
    let password = Password::from("duplicity-reader-public-test-fixture");
    let decoded = crate::openpgp::decode(
        &stored_manifest,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
    )
    .unwrap();
    let manifest = Manifest::parse(&decoded.bytes, ManifestLimits::default()).unwrap();

    let mut wrong_digest = manifest.clone();
    wrong_digest.records[0].hashes[0].hex = b"0000000000000000000000000000000000000000".to_vec();
    let mut rejected = MultipartAssembler::new(&wrong_digest, AssemblyLimits::default()).unwrap();
    let digest_result = rejected.push_openpgp_volume(
        1,
        &stored_volume,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
        &crate::source::Cancellation::default(),
    );
    assert!(matches!(digest_result, Err(AssemblyError::Envelope(_))));
    assert_eq!(rejected.admission.ordinal(), 0);
    assert!(rejected.admission.position().is_none());

    let mut wrong_password = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let wrong = Password::from("wrong-password");
    assert!(matches!(
        wrong_password.push_openpgp_volume(
            1,
            &stored_volume,
            Credentials {
                message_passwords: std::slice::from_ref(&wrong),
                ..Credentials::default()
            },
            CryptoLimits::default(),
            &crate::source::Cancellation::default(),
        ),
        Err(AssemblyError::Crypto(_))
    ));
    assert_eq!(wrong_password.admission.ordinal(), 0);
    assert!(wrong_password.admission.position().is_none());

    let mut cancelled = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let cancel = crate::source::Cancellation::default();
    cancel.cancel();
    assert!(matches!(
        cancelled.push_openpgp_volume(
            1,
            &stored_volume,
            Credentials {
                message_passwords: std::slice::from_ref(&password),
                ..Credentials::default()
            },
            CryptoLimits::default(),
            &cancel,
        ),
        Err(AssemblyError::Cancelled)
    ));
    assert_eq!(cancelled.admission.ordinal(), 0);
    assert!(cancelled.admission.position().is_none());
}

//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn encrypted_queued_restart_start_exception_is_rejected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
    let stored_manifest =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.manifest.gpg")).unwrap();
    let stored_volume =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.vol1.difftar.gpg")).unwrap();
    let password = Password::from("duplicity-reader-public-test-fixture");
    let decoded = crate::openpgp::decode(
        &stored_manifest,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
    )
    .unwrap();
    let mut manifest = Manifest::parse(&decoded.bytes, ManifestLimits::default()).unwrap();
    let decoded_volume = crate::openpgp::decode(
        &stored_volume,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
    )
    .unwrap();
    let parsed_volume = crate::tar::TarVolume::parse(
        &decoded_volume.bytes,
        crate::tar::Termination::Encrypted {
            stored_ciphertext: &stored_volume,
            nonfinal: false,
        },
        TarLimits::default(),
    )
    .unwrap();
    let paths: Vec<_> = parsed_volume
        .entries
        .iter()
        .filter_map(|entry| {
            entry
                .classify(ArchiveKind::Data, 4096, 1024)
                .ok()
                .map(|entry| entry.path)
        })
        .collect();
    assert!(paths.len() >= 2);
    let mut rewrite = manifest.records.last().unwrap().clone();
    rewrite.start = Boundary {
        path: paths[1].clone(),
        block: None,
    };
    manifest.records.push(rewrite);
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let result = assembler.push_openpgp_volume(
        1,
        &stored_volume,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
        &crate::source::Cancellation::default(),
    );
    let error = match result {
        Ok(_) => panic!("encrypted queued restart was accepted"),
        Err(error) => error,
    };
    assert_eq!(error, AssemblyError::UnsupportedBoundaryProfile);
    assert_eq!(assembler.admission.ordinal(), 0);
    assert!(assembler.admission.position().is_none());
}

#[test]
//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
fn authenticated_malformed_tar_is_rejected_before_admission_advances() {
    use crate::manifest::{DeclaredDigest, DigestAlgorithm};
    use sha1::Digest;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
    let stored_manifest =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.manifest.gpg")).unwrap();
    let password = Password::from("duplicity-reader-public-test-fixture");
    let decoded = crate::openpgp::decode(
        &stored_manifest,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
    )
    .unwrap();
    let mut manifest = Manifest::parse(&decoded.bytes, ManifestLimits::default()).unwrap();

    // Produce a valid, MDC-authenticated OpenPGP object whose literal plaintext
    // cannot be parsed as a tar volume. The test-only sealer has a fixed RNG.
    let ciphertext = crate::openpgp::adversarial_tests::seal_literal(b"not a tar archive");
    manifest.records[0].hashes = vec![DeclaredDigest {
        algorithm: DigestAlgorithm::Sha1,
        name: b"SHA1".to_vec(),
        hex: sha1::Sha1::digest(&ciphertext)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into_bytes(),
    }];
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let test_password = Password::from("test only");
    let result = assembler.push_openpgp_volume(
        1,
        &ciphertext,
        Credentials {
            message_passwords: std::slice::from_ref(&test_password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
        &crate::source::Cancellation::default(),
    );
    let error = match result {
        Ok(_) => panic!("malformed authenticated tar was accepted"),
        Err(error) => error,
    };
    assert!(
        matches!(error, AssemblyError::Tar(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(assembler.admission.ordinal(), 0);
    assert!(assembler.admission.position().is_none());
}

fn official_symmetric_volume() -> (Manifest, Vec<u8>, Password) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/symmetric");
    let stored_manifest =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.manifest.gpg")).unwrap();
    let stored_volume =
        std::fs::read(root.join("duplicity-full.20231114T221320Z.vol1.difftar.gpg")).unwrap();
    let password = Password::from("duplicity-reader-public-test-fixture");
    let decoded = crate::openpgp::decode(
        &stored_manifest,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
    )
    .unwrap();
    let manifest = Manifest::parse(&decoded.bytes, ManifestLimits::default()).unwrap();
    (manifest, stored_volume, password)
}

//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn cancellation_at_stored_digest_checkpoint_prevents_openpgp_decode() {
    let (manifest, stored, _correct_password) = official_symmetric_volume();
    let wrong_password = Password::from("wrong-password");
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let cancellation = crate::source::Cancellation::default();
    let guard = super::openpgp_cancel_test_hook::Guard::cancel_at(
        super::openpgp_cancel_test_hook::Checkpoint::AfterStoredDigest,
    );

    let result = assembler.push_openpgp_volume(
        1,
        &stored,
        Credentials {
            message_passwords: std::slice::from_ref(&wrong_password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
        &cancellation,
    );

    assert!(matches!(result, Err(AssemblyError::Cancelled)));
    assert_eq!(guard.hits(), 1);
    assert_eq!(assembler.admission.ordinal(), 0);
    assert!(assembler.admission.position().is_none());
    assert!(assembler.failed);
    assert!(matches!(assembler.finish(), Err(AssemblyError::Poisoned)));
}

//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn cancellation_after_native_decode_prevents_tar_validation_and_commit() {
    let (manifest, stored, password) = official_symmetric_volume();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let cancellation = crate::source::Cancellation::default();
    let guard = super::openpgp_cancel_test_hook::Guard::cancel_at(
        super::openpgp_cancel_test_hook::Checkpoint::AfterNativeDecode,
    );

    let result = assembler.push_openpgp_volume(
        1,
        &stored,
        Credentials {
            message_passwords: std::slice::from_ref(&password),
            ..Credentials::default()
        },
        CryptoLimits::default(),
        &cancellation,
    );

    assert!(matches!(result, Err(AssemblyError::Cancelled)));
    assert_eq!(guard.hits(), 1);
    assert_eq!(assembler.admission.ordinal(), 0);
    assert!(assembler.admission.position().is_none());
    assert!(assembler.entries.is_empty());
    assert!(assembler.pending.is_none());
    assert!(assembler.failed);
    assert!(matches!(assembler.finish(), Err(AssemblyError::Poisoned)));
}

//= docs/parsing-requirements.md#parse-env-017
//= type=test
//# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
#[test]
fn public_catalog_cancellation_after_stored_digest_returns_no_snapshot() {
    let (result, hits) = read_official_symmetric_with_cancel_hook(
        super::openpgp_cancel_test_hook::Checkpoint::AfterStoredDigest,
    );
    match result {
        Err(crate::reader::ReaderError::Source(crate::source::SourceError::Cancelled)) => {}
        Err(error) => panic!("public reader returned the wrong error: {error:?}"),
        Ok(_) => panic!("public reader returned a snapshot after cancellation"),
    }
    assert_eq!(hits, 1);
}

//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
#[test]
fn public_catalog_cancellation_after_native_decode_returns_no_snapshot() {
    let (result, hits) = read_official_symmetric_with_cancel_hook(
        super::openpgp_cancel_test_hook::Checkpoint::AfterNativeDecode,
    );
    assert!(matches!(
        result,
        Err(crate::reader::ReaderError::Source(
            crate::source::SourceError::Cancelled
        ))
    ));
    assert_eq!(hits, 1);
}

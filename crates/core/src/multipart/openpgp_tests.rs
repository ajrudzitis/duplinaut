use super::*;
use crate::manifest::ManifestLimits;
use crate::openpgp::{Credentials, CryptoLimits};
use pgp::types::Password;
use std::path::Path;

//= docs/parsing-requirements.md#parse-env-020
//= type=test
//# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
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
}

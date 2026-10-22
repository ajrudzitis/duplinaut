use crate::openpgp::{decode, Credentials, CryptoError, CryptoLimits, Limit, SecretExport};
use pgp::types::Password;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};
const PASSPHRASE: &str = "duplicity-reader-public-test-fixture";
const FINGERPRINT: [u8; 20] = [
    0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23, 0x17, 0x3b,
    0xad, 0x8a, 0x76, 0x56,
];
fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn source_oracle(mode: &str, objects: BTreeMap<String, Vec<u8>>) {
    let oracle: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture().join("expected.json")).unwrap()).unwrap();
    let mut files = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let mut snapshot_index = 0;
    // An independent test-only tar decoder and source recipe SHA256 oracle check
    // decrypted payloads, rather than comparing the implementation to itself.
    for (name, bytes) in objects
        .into_iter()
        .filter(|(name, _)| name.ends_with("difftar.gpg"))
    {
        let mut archive = tar::Archive::new(Cursor::new(bytes));
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let mut path = entry.path_bytes().into_owned();
            while path.last() == Some(&b'/') {
                path.pop();
            }
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            if let Some(path) = path.strip_prefix(b"deleted/") {
                files.retain(|name, _| {
                    !(name == path
                        || name
                            .strip_prefix(path)
                            .is_some_and(|rest| rest.starts_with(b"/")))
                });
            } else if let Some(path) = path.strip_prefix(b"snapshot/") {
                if entry.header().entry_type().is_file() {
                    files.insert(path.to_vec(), data);
                } else {
                    files.remove(path);
                }
            } else if let Some(path) = path.strip_prefix(b"diff/") {
                let basis = files.get(path).expect("incremental basis file");
                let reconstructed =
                    crate::delta::apply_delta(&data, basis, 8 * 1024 * 1024).unwrap();
                files.insert(path.to_vec(), reconstructed);
            } else {
                panic!("unexpected official archive member {:?} in {name}", path);
            }
        }
        let snapshot = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|snapshot| snapshot["mode"] == mode && snapshot["index"] == snapshot_index)
            .unwrap();
        let expected: BTreeMap<String, String> = snapshot["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["kind"] == "file")
            .map(|entry| {
                (
                    entry["path_hex"].as_str().unwrap().to_owned(),
                    entry["sha256"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let actual: BTreeMap<String, String> = files
            .iter()
            .map(|(path, bytes)| (hex(path), format!("{:x}", Sha256::digest(bytes))))
            .collect();
        assert_eq!(
            actual, expected,
            "{mode} original source snapshot {snapshot_index}"
        );
        snapshot_index += 1;
    }
    assert_eq!(snapshot_index, 3);
}
#[test]
fn original_official_symmetric_public_key_and_signed_objects_decrypt_natively() {
    let root = fixture();
    let password = Password::from(PASSPHRASE);
    let secret = std::fs::read(root.join("test-keys/secret.asc")).unwrap();
    let public = std::fs::read(root.join("test-keys/public.asc")).unwrap();
    let mut total = 0;
    for mode in ["symmetric", "public-key", "signed"] {
        let mut decoded_objects = BTreeMap::new();
        for object in std::fs::read_dir(root.join(mode)).unwrap() {
            let object = object.unwrap();
            let stored = std::fs::read(object.path()).unwrap();
            let exports = [SecretExport {
                bytes: &secret,
                password: Some(&password),
            }];
            let signer_exports = [public.as_slice()];
            let credentials = Credentials {
                message_passwords: if mode == "symmetric" {
                    std::slice::from_ref(&password)
                } else {
                    &[]
                },
                secret_exports: if mode == "symmetric" { &[] } else { &exports },
                signer_exports: if mode == "signed" {
                    &signer_exports
                } else {
                    &[]
                },
                permitted_signers: if mode == "signed" {
                    &[FINGERPRINT]
                } else {
                    &[]
                },
                require_signature: mode == "signed",
            };
            let decoded = decode(&stored, credentials, CryptoLimits::default())
                .unwrap_or_else(|error| panic!("{}: {error}", object.path().display()));
            assert!(!decoded.bytes.is_empty());
            assert_eq!(decoded.authorized_signer.is_some(), mode == "signed");
            if mode == "signed" {
                assert_eq!(
                    decoded.authorized_signer.unwrap().primary_fingerprint,
                    FINGERPRINT
                );
            }
            decoded_objects.insert(object.file_name().into_string().unwrap(), decoded.bytes);
            total += 1;
        }
        source_oracle(mode, decoded_objects);
    }
    assert_eq!(total, 27);
}
#[test]
fn missing_wrong_credentials_mutation_and_ciphertext_bounds_reject() {
    let root = fixture();
    let stored =
        std::fs::read(root.join("symmetric/duplicity-full.20231114T221320Z.manifest.gpg")).unwrap();
    assert!(matches!(
        decode(&stored, Credentials::default(), CryptoLimits::default()),
        Err(CryptoError::MissingPassword)
    ));
    let wrong = [Password::from("wrong public fixture password")];
    assert!(matches!(
        decode(
            &stored,
            Credentials {
                message_passwords: &wrong,
                ..Credentials::default()
            },
            CryptoLimits::default()
        ),
        Err(CryptoError::AuthenticationFailed | CryptoError::WrongCredentials)
    ));
    let password = [Password::from(PASSPHRASE)];
    let mut corrupted = stored.clone();
    *corrupted.last_mut().unwrap() ^= 1;
    assert!(matches!(
        decode(
            &corrupted,
            Credentials {
                message_passwords: &password,
                ..Credentials::default()
            },
            CryptoLimits::default()
        ),
        Err(CryptoError::AuthenticationFailed)
    ));
    let limits = CryptoLimits {
        stored_bytes: stored.len() - 1,
        ..CryptoLimits::default()
    };
    assert!(matches!(
        decode(&stored, Credentials::default(), limits),
        Err(CryptoError::Limit(Limit::Stored))
    ));
}

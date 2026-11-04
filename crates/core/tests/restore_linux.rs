#![cfg(target_os = "linux")]
use duplicity_core::{
    local::LocalSource,
    names::{NameConfig, Timestamp},
    reader::{
        AuthenticationKind, Catalog, CryptoCredentials, CryptoReadContext, OpenPgpLimits,
        OpenPgpPassword, ReaderLimits, SecretExport,
    },
    restore_linux::{restore, Cause, Profile, RestoreLimits, RestoreOutcome},
    restore_plan::RestorePlanLimits,
    source::Cancellation,
    tar::EntryType,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::fd::AsFd,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new(mode: u32) -> Self {
        let p = std::env::temp_dir().join(format!(
            "duplicity-restore-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct PreserveOnIndeterminateTemp {
    path: PathBuf,
    preserve: bool,
}
impl PreserveOnIndeterminateTemp {
    fn new(mode: u32) -> Self {
        let path = std::env::temp_dir().join(format!(
            "duplicity-restore-encrypted-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        Self {
            path,
            preserve: false,
        }
    }

    fn preserve(&mut self) {
        self.preserve = true;
    }
}
impl Drop for PreserveOnIndeterminateTemp {
    fn drop(&mut self) {
        if !self.preserve {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservedKind {
    Directory,
    Regular,
    Symlink,
}

struct ObservedEntry {
    kind: ObservedKind,
    mode: u32,
    mtime_ns: i128,
    size: Option<u64>,
    sha256: Option<String>,
    symlink_target: Option<Vec<u8>>,
}

fn observe_tree(
    path: &Path,
    components: &mut Vec<Vec<u8>>,
    entries: &mut std::collections::BTreeMap<Vec<Vec<u8>>, ObservedEntry>,
) {
    let metadata = fs::symlink_metadata(path).unwrap();
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        ObservedKind::Symlink
    } else if file_type.is_dir() {
        ObservedKind::Directory
    } else if file_type.is_file() {
        ObservedKind::Regular
    } else {
        panic!("restore produced an unsupported filesystem entry type");
    };
    let mut observed = ObservedEntry {
        kind,
        mode: metadata.mode() & 0o777,
        mtime_ns: i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec()),
        size: None,
        sha256: None,
        symlink_target: None,
    };
    match kind {
        ObservedKind::Directory => {}
        ObservedKind::Regular => {
            observed.size = Some(metadata.len());
            let mut file = fs::File::open(path).unwrap();
            let mut digest = Sha256::new();
            let mut buffer = [0u8; 16 * 1024];
            loop {
                let read = file.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                digest.update(&buffer[..read]);
            }
            observed.sha256 = Some(format!("{:x}", digest.finalize()));
        }
        ObservedKind::Symlink => {
            observed.symlink_target =
                Some(fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec());
        }
    }
    assert!(entries.insert(components.clone(), observed).is_none());
    if kind == ObservedKind::Directory {
        for child in fs::read_dir(path).unwrap() {
            let child = child.unwrap();
            components.push(child.file_name().as_bytes().to_vec());
            observe_tree(&child.path(), components, entries);
            components.pop();
        }
    }
}

fn decode_hex_bytes(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty(), "oracle hex component has even length");
    pairs
        .iter()
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("oracle contains invalid hex"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

fn fixture_fingerprint(directory: &Path) -> std::collections::BTreeMap<Vec<u8>, (u64, String)> {
    let mut files = std::collections::BTreeMap::new();
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().as_bytes().to_vec();
        let mut file = fs::File::open(entry.path()).unwrap();
        let size = file.metadata().unwrap().len();
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let read = file.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        assert!(files
            .insert(name, (size, format!("{:x}", digest.finalize())))
            .is_none());
    }
    files
}
#[test]
//= docs/parsing-requirements.md#restore-001
//= type=test
//# A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.
//= docs/parsing-requirements.md#restore-003
//= type=test
//# Restoration filesystem operations MUST prevent traversal through symlinks outside the private staging tree.
//= docs/parsing-requirements.md#restore-004
//= type=test
//# The reader MUST restore supported regular files, directories, and symlinks with ordinary permissions and supported modification times.
//= docs/parsing-requirements.md#restore-015
//= type=test
//# Symlinks MUST preserve the exact supported target bytes and receive their exact mtime through a no-follow operation relative to their retained parent FD.
//= docs/parsing-requirements.md#restore-017
//= type=test
//# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
//= docs/parsing-requirements.md#restore-018
//= type=test
//# Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.
fn official_three_dates_publish_both_profiles_exact_supported_metadata() {
    let source = LocalSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let cancel = Cancellation::default();
    let catalog = Catalog::discover(
        &source,
        "duplicity-",
        NameConfig::default(),
        ReaderLimits::default(),
        &cancel,
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, d)| d.start() == Timestamp(1700000000) && d.end() == Timestamp(1700000000))
        .unwrap()
        .0;
    let oracle: serde_json::Value = serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/duplicity-3.0.7/expected.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for mode in [0o700, 0o755] {
        let temp = Temp::new(mode);
        for day in 0..3 {
            let snapshot = catalog
                .read_snapshot(anchor, Timestamp(1700000000 + day * 86400), &cancel)
                .unwrap();
            let plan = snapshot.restore_all(RestorePlanLimits::default()).unwrap();
            let destination = format!("day{day}");
            let result = restore(
                &plan,
                &temp.0,
                destination.as_bytes(),
                &cancel,
                RestoreLimits::default(),
            );
            let receipt = match result {
                RestoreOutcome::Published(r) => r,
                other => panic!("{other:?}"),
            };
            assert_eq!(
                receipt.profile,
                if mode == 0o700 {
                    Profile::PrivateParentSameParent
                } else {
                    Profile::ProtectedParentWrapper
                }
            );
            assert_eq!(receipt.cleanup.failures, 0);
            eprintln!(
                "native restore filesystem={:#x} profile={:?}",
                receipt.filesystem, receipt.profile
            );
            let expected = oracle["snapshots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["mode"] == "plain" && item["index"] == day as u64)
                .unwrap()["entries"]
                .as_array()
                .unwrap();
            assert_eq!(snapshot.state().len(), expected.len());
            for (path, entry) in snapshot.state().entries() {
                let mut actual = temp.0.join(&destination);
                for part in path.components() {
                    actual.push(std::ffi::OsString::from_vec(part.clone()));
                }
                let raw = path
                    .components()
                    .iter()
                    .map(|p| p.as_slice())
                    .collect::<Vec<_>>()
                    .join(&b'/');
                let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
                let source = expected
                    .iter()
                    .find(|item| item["path_hex"] == hex)
                    .unwrap();
                let metadata = fs::symlink_metadata(&actual).unwrap();
                assert_eq!(
                    metadata.mode() & 0o777,
                    source["mode"].as_u64().unwrap() as u32
                );
                if source["kind"] != "symlink" {
                    assert_eq!(
                        i128::from(metadata.mtime()) * 1_000_000_000
                            + i128::from(metadata.mtime_nsec()),
                        i128::from(source["mtime_ns"].as_i64().unwrap())
                    );
                }
                assert_eq!(metadata.mode() & 0o777, entry.mode() & 0o777);
                assert_eq!(metadata.mtime(), entry.mtime().seconds);
                assert_eq!(metadata.mtime_nsec(), i64::from(entry.mtime().nanoseconds));
                match entry.entry_type() {
                    EntryType::Directory => {
                        assert!(metadata.is_dir());
                        assert_eq!(source["kind"], "directory");
                    }
                    EntryType::Regular => {
                        assert!(metadata.is_file());
                        assert_eq!(source["kind"], "file");
                        use sha2::Digest;
                        assert_eq!(
                            format!("{:x}", sha2::Sha256::digest(fs::read(&actual).unwrap())),
                            source["sha256"].as_str().unwrap()
                        );
                        assert_eq!(fs::read(&actual).unwrap(), entry.content().unwrap());
                    }
                    EntryType::Symlink => {
                        assert!(metadata.file_type().is_symlink());
                        assert_eq!(source["kind"], "symlink");
                        use std::os::unix::ffi::OsStrExt;
                        assert_eq!(
                            fs::read_link(&actual).unwrap().as_os_str().as_bytes(),
                            entry.linkname()
                        );
                    }
                    _ => panic!("unexpectedtype"),
                }
            }
            assert_eq!(fs::read_dir(&temp.0).unwrap().count(), day as usize + 1);
        }
    }
}

#[test]
//= docs/parsing-requirements.md#parse-env-024
//= type=test
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
//= docs/parsing-requirements.md#restore-001
//= type=test
//# A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.
//= docs/parsing-requirements.md#restore-004
//= type=test
//# The reader MUST restore supported regular files, directories, and symlinks with ordinary permissions and supported modification times.
//= docs/parsing-requirements.md#restore-017
//= type=test
//# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
fn official_signed_encrypted_snapshots_publish_exact_oracle_tree() {
    const FULL_KEYS: [&str; 2] = [
        "duplicity-full.20231114T221320Z.manifest.gpg",
        "duplicity-full.20231114T221320Z.vol1.difftar.gpg",
    ];
    const FIRST_INCREMENTAL: [&str; 2] = [
        "duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gpg",
        "duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gpg",
    ];
    const SECOND_INCREMENTAL: [&str; 2] = [
        "duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.gpg",
        "duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gpg",
    ];
    const EXPECTED_FINGERPRINT: [u8; 20] = [
        0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23, 0x17,
        0x3b, 0xad, 0x8a, 0x76, 0x56,
    ];

    let fixture_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
    let signed_fixtures = fixture_root.join("signed");
    let original_fingerprint = fixture_fingerprint(&signed_fixtures);
    let secret = fs::read(fixture_root.join("test-keys/secret.asc")).unwrap();
    let signer = fs::read(fixture_root.join("test-keys/public.asc")).unwrap();
    let secret_password = OpenPgpPassword::from("duplicity-reader-public-test-fixture");
    let secret_exports = [SecretExport {
        bytes: &secret,
        password: Some(&secret_password),
    }];
    let signer_exports = [signer.as_slice()];
    let permitted_signers = [EXPECTED_FINGERPRINT];
    let context = CryptoReadContext {
        credentials: CryptoCredentials {
            message_passwords: &[],
            secret_exports: &secret_exports,
            signer_exports: &signer_exports,
            permitted_signers: &permitted_signers,
            require_signature: true,
        },
        crypto_limits: OpenPgpLimits::default(),
        evidence_limits: duplicity_core::reader::EvidenceLimits::default(),
    };

    let source = LocalSource::new(&signed_fixtures).unwrap();
    let cancel = Cancellation::default();
    let catalog = Catalog::discover(
        &source,
        "duplicity-",
        NameConfig::default(),
        ReaderLimits::default(),
        &cancel,
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.kind() == duplicity_core::chain::SetKind::Full)
        .expect("the official signed fixture contains a full set")
        .0;
    let oracle: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_root.join("expected.json")).unwrap()).unwrap();
    let temp = PreserveOnIndeterminateTemp::new(0o700);
    let parent_file = fs::File::open(&temp.path).unwrap();
    let target_filesystem = rustix::fs::fstatfs(parent_file.as_fd()).unwrap().f_type;
    assert!(
        [0xef53, 0x9123683e, 0x58465342, 0x01021994].contains(&target_filesystem),
        "the Linux restore test parent must use an allowlisted local filesystem"
    );
    let parent_metadata = fs::metadata(&temp.path).unwrap();
    let expected_parent_identity = (parent_metadata.dev(), parent_metadata.ino());

    for day in 0..3 {
        let target = Timestamp(1700000000 + day * 86400);
        let snapshot = catalog
            .read_snapshot_with_crypto(anchor, target, &cancel, &context)
            .unwrap();
        let evidence = snapshot.authentication_evidence();
        let policy = evidence
            .policy()
            .expect("the encrypted snapshot records its authentication policy");
        assert!(
            policy.authorization_configured,
            "signed authorization is configured"
        );
        assert!(policy.require_signature, "the signature is required");
        assert_eq!(
            policy.permitted_primary_fingerprints,
            vec![EXPECTED_FINGERPRINT],
            "the configured signer allowlist is exact"
        );

        let mut expected_keys = FULL_KEYS.to_vec();
        if day >= 1 {
            expected_keys.extend(FIRST_INCREMENTAL);
        }
        if day >= 2 {
            expected_keys.extend(SECOND_INCREMENTAL);
        }
        expected_keys.sort_unstable();
        let mut observed_keys = evidence
            .objects()
            .iter()
            .map(|record| record.key.as_str())
            .collect::<Vec<_>>();
        observed_keys.sort_unstable();
        assert_eq!(
            evidence.objects().len(),
            2 + day as usize * 2,
            "signed chain has exactly the required evidence record count for date {day}"
        );
        assert_eq!(
            observed_keys, expected_keys,
            "signed chain evidence keys exactly match the required sets for date {day}"
        );
        for record in evidence.objects() {
            assert_eq!(record.kind, AuthenticationKind::OpenPgpAuthorized);
            assert_eq!(
                record
                    .authorized_signer
                    .as_ref()
                    .map(|signer| signer.primary_fingerprint),
                Some(EXPECTED_FINGERPRINT),
                "every required object is authorized by the exact primary fingerprint"
            );
        }

        // Only an explicitly authorized, complete snapshot reaches restore planning.
        let plan = snapshot.restore_all(RestorePlanLimits::default()).unwrap();
        let destination = format!("encrypted-day-{day}").into_bytes();
        let destination_path = temp
            .path
            .join(std::ffi::OsString::from_vec(destination.clone()));
        assert!(
            !destination_path.exists(),
            "the destination leaf is absent before publication"
        );
        let receipt = match restore(
            &plan,
            &temp.path,
            &destination,
            &cancel,
            RestoreLimits::default(),
        ) {
            RestoreOutcome::Published(receipt) => receipt,
            RestoreOutcome::PublicationIndeterminate(_) => {
                let preserved_path = temp.path.clone();
                let mut temp = temp;
                temp.preserve();
                panic!(
                    "restore publication was indeterminate; preserved test parent at {}",
                    preserved_path.display()
                );
            }
            RestoreOutcome::Unpublished(_) => {
                panic!("expected encrypted snapshot publication to succeed")
            }
        };
        assert_eq!(receipt.destination.as_bytes(), destination.as_slice());
        assert_eq!(receipt.profile, Profile::PrivateParentSameParent);
        assert_eq!(
            (receipt.parent.device, receipt.parent.inode),
            expected_parent_identity
        );
        assert_eq!(receipt.filesystem, target_filesystem);
        assert_eq!(receipt.cleanup.failures, 0);
        assert!(destination_path.is_dir());

        let expected_snapshot = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["mode"] == "signed" && item["index"] == day as u64)
            .expect("official signed snapshot oracle exists");
        let expected_entries = expected_snapshot["entries"].as_array().unwrap();
        let mut expected_by_components = std::collections::BTreeMap::new();
        for entry in expected_entries {
            let components = entry["components_hex"]
                .as_array()
                .expect("oracle raw path components")
                .iter()
                .map(|component| decode_hex_bytes(component.as_str().unwrap()))
                .collect::<Vec<_>>();
            assert!(
                expected_by_components.insert(components, entry).is_none(),
                "the official oracle has unique raw component paths"
            );
        }
        let mut observed = std::collections::BTreeMap::new();
        observe_tree(&destination_path, &mut Vec::new(), &mut observed);
        assert_eq!(
            observed.keys().collect::<Vec<_>>(),
            expected_by_components.keys().collect::<Vec<_>>(),
            "the published tree has exactly the oracle's raw component paths for date {day}"
        );
        for (components, expected) in &expected_by_components {
            let actual = observed
                .get(components)
                .expect("each oracle entry is present in the published tree");
            let expected_kind = match expected["kind"].as_str().unwrap() {
                "directory" => ObservedKind::Directory,
                "file" => ObservedKind::Regular,
                "symlink" => ObservedKind::Symlink,
                _ => panic!("the oracle contains only supported entry kinds"),
            };
            assert_eq!(actual.kind, expected_kind, "entry kind for date {day}");
            assert_eq!(
                actual.mode,
                expected["mode"].as_u64().unwrap() as u32 & 0o777,
                "ordinary mode bits for date {day}"
            );
            match actual.kind {
                ObservedKind::Directory | ObservedKind::Regular => assert_eq!(
                    actual.mtime_ns,
                    i128::from(expected["mtime_ns"].as_i64().unwrap()),
                    "exact file/directory mtime for date {day}"
                ),
                ObservedKind::Symlink => {
                    // Duplicity's producer oracle explicitly excludes symlink mtime.
                    let expected_target =
                        decode_hex_bytes(expected["symlink_target_hex"].as_str().unwrap());
                    assert_eq!(
                        actual.symlink_target.as_deref(),
                        Some(expected_target.as_slice())
                    );
                }
            }
            if actual.kind == ObservedKind::Regular {
                assert_eq!(
                    actual.size,
                    Some(expected["size"].as_u64().unwrap()),
                    "regular-file size for date {day}"
                );
                assert_eq!(
                    actual.sha256.as_deref(),
                    expected["sha256"].as_str(),
                    "regular-file SHA-256 for date {day}"
                );
            }
        }
    }

    let expected_leaves = (0..3)
        .map(|day| format!("encrypted-day-{day}").into_bytes())
        .collect::<std::collections::BTreeSet<_>>();
    let actual_leaves = fs::read_dir(&temp.path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        actual_leaves, expected_leaves,
        "the destination parent has no staging or wrapper residue"
    );
    assert_eq!(fixture_fingerprint(&signed_fixtures), original_fingerprint);
}
#[test]
//= docs/parsing-requirements.md#restore-011
//= type=test
//# The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.
//= docs/parsing-requirements.md#restore-012
//= type=test
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
fn existing_destinations_never_replaced_and_preflight_limits_do_not_stage() {
    let source = LocalSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let cancel = Cancellation::default();
    let catalog = Catalog::discover(
        &source,
        "duplicity-",
        NameConfig::default(),
        ReaderLimits::default(),
        &cancel,
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, d)| d.start() == d.end() && d.start() == Timestamp(1700000000))
        .unwrap()
        .0;
    let snapshot = catalog
        .read_snapshot(anchor, Timestamp(1700000000), &cancel)
        .unwrap();
    let plan = snapshot.restore_all(RestorePlanLimits::default()).unwrap();
    for kind in 0..3 {
        let temp = Temp::new(0o700);
        let dest = temp.0.join("dest");
        match kind {
            0 => fs::write(&dest, b"sentinel").unwrap(),
            1 => fs::create_dir(&dest).unwrap(),
            _ => std::os::unix::fs::symlink("absent", &dest).unwrap(),
        };
        assert!(matches!(
            restore(&plan, &temp.0, b"dest", &cancel, RestoreLimits::default()),
            RestoreOutcome::PublicationIndeterminate(_)
        ));
        match kind {
            0 => assert_eq!(fs::read(&dest).unwrap(), b"sentinel"),
            1 => assert!(dest.is_dir()),
            _ => assert_eq!(fs::read_link(&dest).unwrap(), PathBuf::from("absent")),
        }
    }
    let temp = Temp::new(0o700);
    for dest in [b"".as_slice(), b".", b"..", b"a/b", b"a\0b"] {
        assert!(matches!(
            restore(&plan, &temp.0, dest, &cancel, RestoreLimits::default()),
            RestoreOutcome::Unpublished(_)
        ));
    }
    for limits in [
        RestoreLimits {
            entries: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            directory_fds: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            metadata_bytes: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            name_bytes: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            payload_bytes: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            forward_work: 0,
            ..RestoreLimits::default()
        },
        RestoreLimits {
            cleanup_work: 0,
            ..RestoreLimits::default()
        },
    ] {
        assert!(matches!(
            restore(&plan, &temp.0, b"dest", &cancel, limits),
            RestoreOutcome::Unpublished(_)
        ));
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
    cancel.cancel();
    assert!(
        matches!(restore(&plan,&temp.0,b"dest",&cancel,RestoreLimits::default()),RestoreOutcome::Unpublished(e)if e.cause==Cause::Cancelled)
    );
    let public = Temp::new(0o777);
    assert!(
        matches!(restore(&plan,&public.0,b"dest",&Cancellation::default(),RestoreLimits::default()),RestoreOutcome::Unpublished(e)if e.cause==Cause::ParentPolicy)
    );
}

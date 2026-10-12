#![cfg(target_os = "linux")]
use duplicity_core::{
    local::LocalSource,
    names::{NameConfig, Timestamp},
    reader::{Catalog, ReaderLimits},
    restore_linux::{restore, Cause, Profile, RestoreLimits, RestoreOutcome},
    restore_plan::RestorePlanLimits,
    source::Cancellation,
    tar::EntryType,
};
use std::{
    fs,
    os::unix::{
        ffi::OsStringExt,
        fs::{MetadataExt, PermissionsExt},
    },
    path::PathBuf,
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

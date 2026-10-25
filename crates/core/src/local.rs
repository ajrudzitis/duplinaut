//! Read-only, flat Unix directory adapter. Filesystem identity is conditional
//! change detection, not an authenticated immutable content identity.
use crate::source::{
    Cancellation, CheckedReader, ObjectIdentity, ObjectSource, SourceError, SourceLimits,
    StoredObject,
};
use crate::verified::local_read::{advance_local_read_phase, LocalReadEvent, LocalReadPhase};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, Stat};
use std::{
    fs::File,
    io::{self, Read},
    os::fd::OwnedFd,
    os::unix::{ffi::OsStrExt, fs::FileExt},
    path::Path,
    sync::Arc,
};

pub struct LocalSource {
    root: Arc<OwnedFd>,
}
fn regular(stat: &Stat) -> bool {
    FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
}
fn token(stat: &Stat) -> String {
    format!(
        "local-v1:{}:{}:{}:{}:{}:{}:{}",
        stat.st_dev,
        stat.st_ino,
        stat.st_size,
        stat.st_mtime,
        stat.st_mtime_nsec,
        stat.st_ctime,
        stat.st_ctime_nsec
    )
}
fn safe_key(key: &str) -> bool {
    !key.is_empty() && key != "." && key != ".." && !key.contains(['/', '\\', '\0'])
}
fn cancelled(cancel: &Cancellation) -> Result<(), SourceError> {
    if cancel.is_cancelled() {
        Err(SourceError::Cancelled)
    } else {
        Ok(())
    }
}
fn os_error(error: rustix::io::Errno) -> SourceError {
    if error == rustix::io::Errno::NOENT {
        SourceError::Missing
    } else {
        SourceError::Io(error.into())
    }
}
impl LocalSource {
    /// Root's final component cannot be a symlink. Ancestors are selected by the
    /// caller at construction; subsequent operations use only the retained FD.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, SourceError> {
        let root = root.as_ref();
        // A trailing slash makes Linux follow the last directory symlink even
        // with NOFOLLOW. Preserve the explicit final-component policy.
        if root != Path::new("/") && root.as_os_str().as_bytes().ends_with(b"/") {
            return Err(SourceError::Backend(
                "local root must not have a trailing slash".into(),
            ));
        }
        let fd = fs::open(
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(os_error)?;
        Ok(Self { root: Arc::new(fd) })
    }
}
impl ObjectSource for LocalSource {
    fn list(
        &self,
        prefix: &str,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError> {
        cancelled(cancel)?;
        if prefix.contains(['/', '\\', '\0']) {
            return Err(SourceError::Backend("local prefix must be flat".into()));
        }
        let mut objects = Vec::new();
        let mut charged = 0usize;
        // Open a distinct directory description so concurrent/repeated listings
        // cannot share a readdir position.
        let fd = fs::openat(
            &*self.root,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(os_error)?;
        let dir = fs::Dir::new(fd).map_err(os_error)?;
        for entry in dir {
            cancelled(cancel)?;
            let entry = entry.map_err(os_error)?;
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let key = std::str::from_utf8(bytes)
                .map_err(|_| SourceError::Backend("non-UTF8 local key is unsupported".into()))?;
            if !safe_key(key) {
                return Err(SourceError::Backend("unsafe local key".into()));
            }
            if !key.starts_with(prefix) {
                continue;
            }
            let stat = fs::statat(&*self.root, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(os_error)?;
            if !regular(&stat) {
                continue;
            }
            let size = u64::try_from(stat.st_size).map_err(|_| SourceError::Limit)?;
            if size > limits.max_stored_bytes || objects.len() >= limits.max_objects {
                return Err(SourceError::Limit);
            }
            // Eight maximum-width integers plus separators fit within 256 bytes.
            // Precharge both retained strings and one exact Vec slot before allocation.
            charged = charged
                .checked_add(key.len())
                .and_then(|n| n.checked_add(256 + std::mem::size_of::<StoredObject>()))
                .filter(|n| *n <= limits.max_metadata_bytes)
                .ok_or(SourceError::Limit)?;
            objects
                .try_reserve_exact(1)
                .map_err(|_| SourceError::Limit)?;
            objects.push(StoredObject {
                key: key.into(),
                size,
                identity: ObjectIdentity {
                    version_id: Some(token(&stat)),
                    etag: None,
                },
            });
        }
        cancelled(cancel)?;
        Ok(objects)
    }
    //= docs/parsing-requirements.md#parse-source-001
    //= type=implementation
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    fn open(
        &self,
        object: &StoredObject,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError> {
        cancelled(cancel)?;
        if !safe_key(&object.key) {
            return Err(SourceError::Backend("unsafe local key".into()));
        }
        if object.size > limits.max_stored_bytes {
            return Err(SourceError::Limit);
        }
        object
            .key
            .len()
            .checked_add(256 + std::mem::size_of::<CheckedReader<LocalReader>>())
            .filter(|n| *n <= limits.max_metadata_bytes)
            .ok_or(SourceError::Limit)?;
        let fd = fs::openat(
            &*self.root,
            object.key.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(os_error)?;
        let stat = fs::fstat(&fd).map_err(os_error)?;
        if !regular(&stat)
            || u64::try_from(stat.st_size).ok() != Some(object.size)
            || object.identity.etag.is_some()
            || object.identity.version_id.as_deref() != Some(token(&stat).as_str())
        {
            return Err(SourceError::Changed);
        }
        let reader = LocalReader {
            file: File::from(fd),
            root: self.root.clone(),
            key: object.key.clone(),
            version: object
                .identity
                .version_id
                .clone()
                .ok_or(SourceError::Changed)?,
            cancel: cancel.clone(),
            failure: None,
            offset: 0,
        };
        cancelled(cancel)?;
        Ok(Box::new(CheckedReader::new(
            reader,
            object.size,
            limits.max_stored_bytes,
            cancel.clone(),
        )?))
    }
}
struct LocalReader {
    file: File,
    root: Arc<OwnedFd>,
    key: String,
    version: String,
    cancel: Cancellation,
    failure: Option<(io::ErrorKind, String)>,
    offset: u64,
}
// Commit the logical offset only after both identity observations succeed.
// A retryable Interrupted after the positional read re-reads the same bytes.
//= docs/parsing-requirements.md#parse-source-001
//= type=implementation
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
fn read_validated(
    file: &File,
    offset: u64,
    buffer: &mut [u8],
    mut validate: impl FnMut() -> io::Result<()>,
) -> io::Result<(usize, u64)> {
    let mut phase = LocalReadPhase::Start { offset };
    validate()?;
    phase = advance_local_read_phase(phase, LocalReadEvent::PrevalidationSucceeded)
        .ok_or_else(|| io::Error::other("invalid local read prevalidation transition"))?;
    let requested = u64::try_from(buffer.len())
        .map_err(|_| io::Error::other("local read buffer length exceeds accounting range"))?;
    let count = file.read_at(buffer, offset)?;
    let count_u64 = u64::try_from(count)
        .map_err(|_| io::Error::other("local read count exceeds accounting range"))?;
    phase = advance_local_read_phase(
        phase,
        LocalReadEvent::ReadSucceeded {
            requested,
            count: count_u64,
        },
    )
    .ok_or_else(|| io::Error::other("invalid local read count"))?;
    validate()?;
    phase = advance_local_read_phase(phase, LocalReadEvent::PostvalidationSucceeded)
        .ok_or_else(|| io::Error::other("invalid local read postvalidation transition"))?;
    let Some(LocalReadPhase::Committed { next_offset, .. }) =
        advance_local_read_phase(phase, LocalReadEvent::Commit)
    else {
        return Err(io::Error::other("local read offset overflow"));
    };
    Ok((count, next_offset))
}
impl LocalReader {
    fn validate(&self) -> io::Result<()> {
        let fd = fs::fstat(&self.file)?;
        let path = fs::statat(&*self.root, self.key.as_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        if !regular(&fd)
            || !regular(&path)
            || token(&fd) != self.version
            || token(&path) != self.version
        {
            return Err(io::Error::other("local object identity changed"));
        }
        Ok(())
    }
}
impl Read for LocalReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.failure {
            return Err(io::Error::new(*kind, message.clone()));
        }
        let result = (|| {
            if self.cancel.is_cancelled() {
                return Err(io::Error::other("source operation cancelled"));
            }
            let (n, next) = read_validated(&self.file, self.offset, buffer, || self.validate())?;
            self.offset = next;
            Ok(n)
        })();
        if let Err(error) = &result {
            if error.kind() != io::ErrorKind::Interrupted {
                self.failure = Some((error.kind(), error.to_string()));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    //= docs/parsing-requirements.md#parse-source-001
    //= type=test
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    fn interrupted_post_read_validation_retries_without_losing_bytes() {
        let file = File::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.manifest",
        ))
        .unwrap();
        let mut buffer = [0u8; 4];
        let mut validations = 0;
        let failed = read_validated(&file, 0, &mut buffer, || {
            validations += 1;
            if validations == 2 {
                Err(io::Error::from(io::ErrorKind::Interrupted))
            } else {
                Ok(())
            }
        });
        assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::Interrupted);
        let (count, next) = read_validated(&file, 0, &mut buffer, || Ok(())).unwrap();
        assert_eq!(count, 4);
        assert_eq!(next, 4);
        assert_eq!(&buffer, b"Host");
        let (count, next) = read_validated(&file, next, &mut buffer, || Ok(())).unwrap();
        assert_eq!(count, 4);
        assert_eq!(next, 8);
        assert_eq!(&buffer, b"name");
    }

    #[test]
    //= docs/parsing-requirements.md#parse-source-001
    //= type=test
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    fn failed_pre_or_post_validation_returns_no_read_commit() {
        let file = File::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.manifest",
        ))
        .unwrap();
        for fail_at in [1, 2] {
            let mut buffer = [0u8; 4];
            let mut validations = 0;
            let result = read_validated(&file, 0, &mut buffer, || {
                validations += 1;
                if validations == fail_at {
                    Err(io::Error::other("identity changed"))
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());

            let (count, next) = read_validated(&file, 0, &mut buffer, || Ok(())).unwrap();
            assert_eq!(count, 4);
            assert_eq!(next, 4);
            assert_eq!(&buffer, b"Host");
        }
    }
}

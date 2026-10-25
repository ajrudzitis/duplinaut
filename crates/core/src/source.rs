//! Contracts for read-only storage adapters. I/O and identity checks are trusted
//! integration boundaries; parser correctness does not prove backend behavior.
use crate::verified::checked_read::{
    admit_checked_size, choose_checked_read_window, commit_checked_read_window,
    decide_checked_eof_probe,
};
use std::{
    io::{self, Read},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdentity {
    /// Prefer an immutable version when supplied by the backend.
    pub version_id: Option<String>,
    /// Conditional-read identity token, not a content digest.
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub key: String,
    pub size: u64,
    pub identity: ObjectIdentity,
}

#[derive(Debug, Clone, Copy)]
pub struct SourceLimits {
    pub max_objects: usize,
    pub max_metadata_bytes: usize,
    pub max_stored_bytes: u64,
}

impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            max_objects: 10_000_000,
            max_metadata_bytes: 256 * 1024 * 1024,
            max_stored_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub enum SourceError {
    Io(io::Error),
    Changed,
    Missing,
    Limit,
    Cancelled,
    Backend(String),
}
impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "source I/O failed: {error}"),
            Self::Changed => f.write_str("object changed since discovery"),
            Self::Missing => f.write_str("object is missing"),
            Self::Limit => f.write_str("source resource limit exceeded"),
            Self::Cancelled => f.write_str("source operation cancelled"),
            Self::Backend(message) => write!(f, "storage backend: {message}"),
        }
    }
}
impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for SourceError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Implementations list only and open read-only streams. They must enforce
/// listing bounds before retaining additional metadata, honor cancellation,
/// and check known version/ETag identities before delivering object data.
/// Returned streams must fail on a short/long body relative to `object.size`.
/// Callers still validate manifest digests over the original stored bytes.
pub trait ObjectSource: Send + Sync {
    fn list(
        &self,
        prefix: &str,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError>;
    fn open(
        &self,
        object: &StoredObject,
        limits: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError>;
}

/// Trusted I/O wrapper enforcing exact discovered size and cancellation.
/// Reaching the expected size requires an EOF probe rather than silently
/// accepting extra bytes from a misbehaving backend.
pub struct CheckedReader<R> {
    inner: R,
    remaining: u64,
    cancel: Cancellation,
    failure: Option<(io::ErrorKind, String)>,
}
impl<R: Read> CheckedReader<R> {
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=implementation
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    pub fn new(
        inner: R,
        expected: u64,
        maximum: u64,
        cancel: Cancellation,
    ) -> Result<Self, SourceError> {
        let remaining = admit_checked_size(expected, maximum).ok_or(SourceError::Limit)?;
        if cancel.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        Ok(Self {
            inner,
            remaining,
            cancel,
            failure: None,
        })
    }
    fn latch(&mut self, error: io::Error) -> io::Error {
        // Interrupted is the standard retryable Read error. All other failures
        // are terminal so a second EOF probe cannot conceal an invalid body.
        if error.kind() != io::ErrorKind::Interrupted {
            self.failure = Some((error.kind(), error.to_string()));
        }
        error
    }
}
impl<R: Read> Read for CheckedReader<R> {
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=implementation
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.failure {
            return Err(io::Error::new(*kind, message.clone()));
        }
        if self.cancel.is_cancelled() {
            return Err(io::Error::other("source operation cancelled"));
        }
        if self.remaining == 0 {
            let mut probe = [0u8; 1];
            let count = match self.inner.read(&mut probe) {
                Ok(count) => count,
                Err(error) => return Err(self.latch(error)),
            };
            match decide_checked_eof_probe(self.remaining, count) {
                Some(true) => return Ok(0),
                Some(false) | None => {
                    return Err(self.latch(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "object body exceeds discovered size",
                    )))
                }
            }
        }
        let buffer_len_u64 = u64::try_from(buffer.len()).map_err(|_| {
            self.latch(io::Error::new(
                io::ErrorKind::InvalidData,
                "source buffer length exceeds accounting range",
            ))
        })?;
        let requested_u64 =
            choose_checked_read_window(self.remaining, buffer_len_u64).ok_or_else(|| {
                self.latch(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "source read window is empty",
                ))
            })?;
        let requested = usize::try_from(requested_u64).map_err(|_| {
            self.latch(io::Error::new(
                io::ErrorKind::InvalidData,
                "source read window exceeds slice range",
            ))
        })?;
        let count = match self.inner.read(&mut buffer[..requested]) {
            Ok(count) => count,
            Err(error) => return Err(self.latch(error)),
        };
        if count == 0 {
            return Err(self.latch(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "object body is shorter than discovered size",
            )));
        }
        if count > requested {
            return Err(self.latch(io::Error::new(
                io::ErrorKind::InvalidData,
                "source returned an invalid byte count",
            )));
        }
        let count_u64 = u64::try_from(count).map_err(|_| {
            self.latch(io::Error::new(
                io::ErrorKind::InvalidData,
                "source byte count exceeds accounting range",
            ))
        })?;
        let Some(next_remaining) =
            commit_checked_read_window(self.remaining, buffer_len_u64, requested_u64, count_u64)
        else {
            return Err(self.latch(io::Error::new(
                io::ErrorKind::InvalidData,
                "source returned an invalid byte count",
            )));
        };
        self.remaining = next_remaining;
        Ok(count)
    }
}

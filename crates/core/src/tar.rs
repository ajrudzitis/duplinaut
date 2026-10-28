//! Native tar volume framing and duplicity operation classification.
//!
//! Header bytes use the Vest DSL. Semantic controllers have native tests but
//! await formal refinement; successful container EOF does not prove multipart
//! completion. Payloads borrow the supplied decoded volume.
use crate::generated::tar::TarHeaderFmt;
use crate::path::{BackupPath, PathError};
use vest_lib::core::exec::parser::Parser;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryType {
    Regular,
    Directory,
    Symlink,
    Hardlink,
    CharacterDevice,
    BlockDevice,
    Fifo,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TarTime {
    pub seconds: i64,
    pub nanoseconds: u32,
}
#[derive(Debug)]
pub struct TarEntry<'a> {
    pub name: Vec<u8>,
    pub linkname: Vec<u8>,
    pub entry_type: EntryType,
    pub mode: u32,
    pub uid: u64,
    pub gid: u64,
    pub mtime: TarTime,
    pub uname: Vec<u8>,
    pub gname: Vec<u8>,
    pub atime: Option<TarTime>,
    pub ctime: Option<TarTime>,
    pub device_major: u64,
    pub device_minor: u64,
    pub payload: &'a [u8],
}
#[derive(Clone, Copy, Debug)]
pub enum Termination<'a> {
    /// Inspected raw/gzip producers: complete-entry EOF or complete zero blocks.
    Data,
    /// Signature tar must have a zero footer, except an exactly empty stream.
    Signatures,
    /// Exact duplicity 10240-byte record footer. Nonfinal top-off must equal
    /// an exact prefix of the original stored ciphertext, not decoded plaintext.
    Encrypted {
        stored_ciphertext: &'a [u8],
        nonfinal: bool,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct TarLimits {
    pub max_bytes: usize,
    pub max_entries: usize,
    pub max_metadata_bytes: usize,
    pub max_path_bytes: usize,
    pub max_entry_bytes: usize,
    pub max_extended_headers: usize,
}
impl Default for TarLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024 * 1024,
            max_entries: 10_000_000,
            max_metadata_bytes: 256 * 1024 * 1024,
            max_path_bytes: 1024 * 1024,
            max_entry_bytes: 8 * 1024 * 1024 * 1024,
            max_extended_headers: 1_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TarError {
    Limit,
    Truncated,
    Checksum,
    Numeric,
    UnsupportedHeader,
    UnsupportedEntry,
    UnsupportedPax,
    Pax,
    TextEncoding,
    DanglingOverride,
    Termination,
    Operation,
    Path(PathError),
}
#[derive(Debug)]
pub struct TarVolume<'a> {
    pub entries: Vec<TarEntry<'a>>,
    pub entry_stream_bytes: usize,
}

fn text(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]
}
fn number(bytes: &[u8]) -> Result<i128, TarError> {
    crate::verified::tar_number::number(bytes).ok_or(TarError::Numeric)
}
fn unsigned(bytes: &[u8]) -> Result<u64, TarError> {
    crate::verified::tar_number::unsigned(bytes).ok_or(TarError::Numeric)
}
fn decimal(bytes: &[u8]) -> Result<u64, TarError> {
    crate::verified::tar_helpers::decimal(bytes).ok_or(TarError::Numeric)
}
fn time(bytes: &[u8]) -> Result<TarTime, TarError> {
    crate::verified::tar_time::time(bytes)
        .map(|(seconds, nanoseconds)| TarTime {
            seconds,
            nanoseconds,
        })
        .ok_or(TarError::Numeric)
}
type Pax<'a> = [Option<&'a [u8]>; 11];
fn pax(bytes: &[u8]) -> Result<Pax<'_>, TarError> {
    let mut result = [None; 11];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let (end, key, value) =
            crate::verified::tar_pax::record(bytes, cursor).ok_or(TarError::Pax)?;
        let index = crate::verified::tar_pax::key_index(key).ok_or(TarError::UnsupportedPax)?;
        if result[index].is_some() || value.contains(&0) {
            return Err(TarError::Pax);
        }
        if !value.is_empty() {
            match index {
                2 | 4 | 5 => {
                    decimal(value)?;
                }
                3 | 8 | 9 => {
                    time(value)?;
                }
                10 if !matches!(value, b"BINARY" | b"ISO-IR 10646 2000 UTF-8" | b"UTF-8") => {
                    return Err(TarError::UnsupportedPax)
                }
                _ => {}
            }
        }
        result[index] = Some(value);
        cursor = end;
    }
    Ok(result)
}
fn resolved<'a>(global: Pax<'a>, local: Pax<'a>) -> Pax<'a> {
    let mut result = global;
    for i in 0..11 {
        if let Some(value) = local[i] {
            result[i] = if value.is_empty() { None } else { Some(value) };
        }
    }
    result
}
fn charge(used: &mut usize, amount: usize, limit: usize) -> Result<(), TarError> {
    *used = used.checked_add(amount).ok_or(TarError::Limit)?;
    if *used > limit {
        return Err(TarError::Limit);
    }
    Ok(())
}
fn long(bytes: &[u8], limit: usize) -> Result<&[u8], TarError> {
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or(TarError::TextEncoding)?;
    if bytes[end..].iter().any(|b| *b != 0) {
        return Err(TarError::TextEncoding);
    }
    if end > limit {
        return Err(TarError::Limit);
    }
    Ok(&bytes[..end])
}
// Explicit remainders are retained for the forthcoming wire proof.
#[allow(clippy::manual_is_multiple_of)]
fn footer(bytes: &[u8], cursor: usize, profile: Termination<'_>) -> Result<(), TarError> {
    let (mode, stored, nonfinal) = match profile {
        Termination::Data => (0, &[][..], false),
        Termination::Signatures => (1, &[][..], false),
        Termination::Encrypted {
            stored_ciphertext,
            nonfinal,
        } => (2, stored_ciphertext, nonfinal),
    };
    if crate::verified::tar_footer::footer_valid(bytes, cursor, mode, stored, nonfinal) {
        Ok(())
    } else {
        Err(TarError::Termination)
    }
}

impl<'a> TarVolume<'a> {
    //= docs/parsing-requirements.md#parse-tar-001
    //# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

    //= docs/parsing-requirements.md#parse-tar-002
    //# The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

    //= docs/parsing-requirements.md#parse-tar-003
    //# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

    //= docs/parsing-requirements.md#parse-tar-004
    //# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

    //= docs/parsing-requirements.md#parse-tar-005
    //# The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

    pub fn parse(
        bytes: &'a [u8],
        profile: Termination<'_>,
        limits: TarLimits,
    ) -> Result<Self, TarError> {
        if bytes.len() > limits.max_bytes {
            return Err(TarError::Limit);
        }
        let mut cursor = 0usize;
        let mut entries = Vec::new();
        let mut metadata = 0usize;
        let mut extended = 0usize;
        let mut global: Pax<'a> = [None; 11];
        let mut local: Pax<'a> = [None; 11];
        let mut pending = false;
        let mut long_name = None;
        let mut long_link = None;
        while cursor < bytes.len() {
            if bytes.len() - cursor < 512 {
                return Err(TarError::Truncated);
            }
            let block = &bytes[cursor..cursor + 512];
            if block.iter().all(|b| *b == 0) {
                break;
            }
            let (consumed, h) = TarHeaderFmt
                .parse(&block)
                .map_err(|_| TarError::Truncated)?;
            if consumed != 512 {
                return Err(TarError::Truncated);
            }
            let expected = unsigned(h.checksum)?;
            if !crate::verified::tar_helpers::checksum_matches(block, expected) {
                return Err(TarError::Checksum);
            }
            let posix = h.magic_version == *b"ustar\x0000";
            if !posix && h.magic_version != *b"ustar  \0" && h.magic_version != [0; 8] {
                return Err(TarError::UnsupportedHeader);
            }
            let mode = u32::try_from(unsigned(h.mode)?).map_err(|_| TarError::Numeric)?;
            let mut uid = unsigned(h.uid)?;
            let mut gid = unsigned(h.gid)?;
            let raw_size = unsigned(h.size)?;
            let mut mtime = TarTime {
                seconds: i64::try_from(number(h.mtime)?).map_err(|_| TarError::Numeric)?,
                nanoseconds: 0,
            };
            let device_major = unsigned(h.device_major)?;
            let device_minor = unsigned(h.device_minor)?;
            let extension = matches!(h.entry_type, b'L' | b'K' | b'x' | b'g');
            let values = resolved(global, local);
            let size = if extension {
                raw_size
            } else {
                values[2].map(decimal).transpose()?.unwrap_or(raw_size)
            };
            let size = usize::try_from(size).map_err(|_| TarError::Limit)?;
            if size > limits.max_entry_bytes {
                return Err(TarError::Limit);
            }
            let (data_start, data_end, next) =
                crate::verified::tar_helpers::extent(cursor, size, bytes.len()).map_err(
                    |error| match error {
                        crate::verified::tar_helpers::ExtentError::Overflow => TarError::Limit,
                        crate::verified::tar_helpers::ExtentError::Truncated => TarError::Truncated,
                    },
                )?;
            let payload = &bytes[data_start..data_end];
            cursor = next;
            if extension {
                extended = extended.checked_add(1).ok_or(TarError::Limit)?;
                if extended > limits.max_extended_headers {
                    return Err(TarError::Limit);
                }
                charge(&mut metadata, size, limits.max_metadata_bytes)?;
                match h.entry_type {
                    b'L' => {
                        long_name = Some(long(payload, limits.max_path_bytes)?);
                        pending = true;
                    }
                    b'K' => {
                        long_link = Some(long(payload, limits.max_path_bytes)?);
                        pending = true;
                    }
                    b'x' => {
                        let parsed = pax(payload)?;
                        for i in 0..11 {
                            if parsed[i].is_some() {
                                local[i] = parsed[i];
                            }
                        }
                        pending = true;
                    }
                    b'g' => {
                        let parsed = pax(payload)?;
                        for i in 0..11 {
                            if let Some(value) = parsed[i] {
                                global[i] = if value.is_empty() { None } else { Some(value) };
                            }
                        }
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            let entry_type = match h.entry_type {
                0 | b'0' => EntryType::Regular,
                b'1' => EntryType::Hardlink,
                b'2' => EntryType::Symlink,
                b'3' => EntryType::CharacterDevice,
                b'4' => EntryType::BlockDevice,
                b'5' => EntryType::Directory,
                b'6' => EntryType::Fifo,
                _ => return Err(TarError::UnsupportedEntry),
            };
            if entry_type != EntryType::Regular && size != 0 {
                return Err(TarError::UnsupportedEntry);
            }
            if entries.len() >= limits.max_entries {
                return Err(TarError::Limit);
            }
            let raw_name = text(h.name);
            let prefix = if posix { text(h.prefix) } else { b"" };
            let name = values[0].or(long_name);
            let link = values[1].or(long_link).unwrap_or(text(h.linkname));
            let name_length = name.map_or(
                raw_name.len() + prefix.len() + usize::from(!prefix.is_empty()),
                |v| v.len(),
            );
            if name_length > limits.max_path_bytes || link.len() > limits.max_path_bytes {
                return Err(TarError::Limit);
            }
            if values[10] != Some(b"BINARY") {
                for i in [0, 1, 6, 7] {
                    if let Some(value) = values[i] {
                        std::str::from_utf8(value).map_err(|_| TarError::TextEncoding)?;
                    }
                }
            }
            let uname = values[6].unwrap_or(text(h.uname));
            let gname = values[7].unwrap_or(text(h.gname));
            if uname.len() > limits.max_path_bytes || gname.len() > limits.max_path_bytes {
                return Err(TarError::Limit);
            }
            // Conservative Vec capacity and entry storage accounting before allocation.
            let entry_charge = std::mem::size_of::<TarEntry<'_>>()
                .checked_mul(4)
                .and_then(|v| v.checked_add(name_length))
                .and_then(|v| v.checked_add(link.len()))
                .and_then(|v| v.checked_add(uname.len()))
                .and_then(|v| v.checked_add(gname.len()))
                .ok_or(TarError::Limit)?;
            charge(&mut metadata, entry_charge, limits.max_metadata_bytes)?;
            let name = if let Some(value) = name {
                value.to_vec()
            } else {
                let mut joined = Vec::with_capacity(name_length);
                if !prefix.is_empty() {
                    joined.extend_from_slice(prefix);
                    joined.push(b'/');
                }
                joined.extend_from_slice(raw_name);
                joined
            };
            if name.is_empty() || name.contains(&0) || link.contains(&0) {
                return Err(TarError::TextEncoding);
            }
            if let Some(value) = values[3] {
                mtime = time(value)?;
            }
            if let Some(value) = values[4] {
                uid = decimal(value)?;
            }
            if let Some(value) = values[5] {
                gid = decimal(value)?;
            }
            let atime = values[8].map(time).transpose()?;
            let ctime = values[9].map(time).transpose()?;
            entries.push(TarEntry {
                name,
                linkname: link.to_vec(),
                entry_type,
                mode,
                uid,
                gid,
                mtime,
                uname: uname.to_vec(),
                gname: gname.to_vec(),
                atime,
                ctime,
                device_major,
                device_minor,
                payload,
            });
            local = [None; 11];
            pending = false;
            long_name = None;
            long_link = None;
        }
        if pending {
            return Err(TarError::DanglingOverride);
        }
        footer(bytes, cursor, profile)?;
        Ok(Self {
            entries,
            entry_stream_bytes: cursor,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveKind {
    Data,
    Signatures,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Snapshot,
    Diff,
    Deleted,
    Signature,
    MultipartSnapshot,
    MultipartDiff,
}
#[derive(Debug)]
pub struct ClassifiedEntry {
    pub operation: Operation,
    pub path: BackupPath,
    pub part: Option<u64>,
}
impl TarEntry<'_> {
    //= docs/parsing-requirements.md#parse-entry-001
    //# The reader MUST classify duplicity data entries using the five data-entry prefixes defined above.

    //= docs/parsing-requirements.md#parse-entry-002
    //# The reader MUST classify signature entries using the three signature-entry prefixes defined above.

    //= docs/parsing-requirements.md#parse-entry-003
    //# The reader MUST reject an unrecognized duplicity operation prefix in an otherwise recognized data or signature archive.

    /// Classification never treats an archive name as a filesystem destination.
    pub fn classify(
        &self,
        archive: ArchiveKind,
        max_path_bytes: usize,
        max_components: usize,
    ) -> Result<ClassifiedEntry, TarError> {
        let slash = self
            .name
            .iter()
            .position(|b| *b == b'/')
            .ok_or(TarError::Operation)?;
        let prefix = &self.name[..slash];
        let mut path = &self.name[slash + 1..];
        let operation = match (archive, prefix) {
            (_, b"snapshot") => Operation::Snapshot,
            (_, b"deleted") => Operation::Deleted,
            (ArchiveKind::Data, b"diff") => Operation::Diff,
            (ArchiveKind::Data, b"multivol_snapshot") => Operation::MultipartSnapshot,
            (ArchiveKind::Data, b"multivol_diff") => Operation::MultipartDiff,
            (ArchiveKind::Signatures, b"signature") => Operation::Signature,
            _ => return Err(TarError::Operation),
        };
        let part = if matches!(
            operation,
            Operation::MultipartSnapshot | Operation::MultipartDiff
        ) {
            let end = path
                .iter()
                .rposition(|b| *b == b'/')
                .ok_or(TarError::Operation)?;
            let number = decimal(&path[end + 1..]).map_err(|_| TarError::Operation)?;
            if number == 0 || self.entry_type != EntryType::Regular {
                return Err(TarError::Operation);
            }
            path = &path[..end];
            Some(number)
        } else {
            None
        };
        let path = BackupPath::parse(
            path,
            self.entry_type == EntryType::Directory,
            max_path_bytes,
            max_components,
        )
        .map_err(TarError::Path)?;
        Ok(ClassifiedEntry {
            operation,
            path,
            part,
        })
    }
}

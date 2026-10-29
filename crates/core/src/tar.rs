//! Native tar volume framing and duplicity operation classification.
//!
//! Header bytes use the Vest DSL. Semantic controllers have native tests but
//! await formal refinement; successful container EOF does not prove multipart
//! completion. Payloads borrow the supplied decoded volume.
use crate::generated::tar::TarHeaderFmt;
use crate::path::{BackupPath, PathError};
use vest_lib::core::exec::parser::Parser;

pub use crate::verified::tar_entry::TarEntryType as EntryType;
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
    crate::verified::tar_pax::nul_terminated_prefix(bytes)
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
type Pax<'a> = [Option<&'a [u8]>; 11];
fn pax(bytes: &[u8]) -> Result<Pax<'_>, TarError> {
    crate::verified::tar_pax::ingest(bytes).map_err(|error| match error {
        crate::verified::tar_pax::PaxIngestError::Framing
        | crate::verified::tar_pax::PaxIngestError::Malformed => TarError::Pax,
        crate::verified::tar_pax::PaxIngestError::UnsupportedKey
        | crate::verified::tar_pax::PaxIngestError::UnsupportedValue => TarError::UnsupportedPax,
        crate::verified::tar_pax::PaxIngestError::Numeric => TarError::Numeric,
    })
}
fn resolved<'a>(global: Pax<'a>, local: Pax<'a>) -> Pax<'a> {
    crate::verified::tar_pax::resolve_values(global, local)
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

    //= docs/parsing-requirements.md#parse-tar-006
    //# The tar parser MUST apply PAX extension state in header order.

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
            //= docs/parsing-requirements.md#parse-tar-011
            //# Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.
            let mode = crate::verified::tar_number::narrow_mode(unsigned(h.mode)?)
                .ok_or(TarError::Numeric)?;
            let header_uid = unsigned(h.uid)?;
            let header_gid = unsigned(h.gid)?;
            let raw_size = unsigned(h.size)?;
            //= docs/parsing-requirements.md#parse-tar-011
            //# Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.
            let header_mtime_seconds = crate::verified::tar_number::narrow_mtime(number(h.mtime)?)
                .ok_or(TarError::Numeric)?;
            let device_major = unsigned(h.device_major)?;
            let device_minor = unsigned(h.device_minor)?;
            let extension = matches!(h.entry_type, b'L' | b'K' | b'x' | b'g');
            //= docs/parsing-requirements.md#parse-tar-006
            //# Local fields MUST override global fields for the next real entry.
            let values = resolved(global, local);
            //= docs/parsing-requirements.md#parse-tar-007
            //# GNU and PAX extension headers MUST use the raw size in their own tar header to determine their payload extent.
            //# A real entry MUST use the effective PAX `size` value when present, or its raw tar header size when absent.
            //= docs/parsing-requirements.md#parse-tar-008
            //# After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary.
            let (size, payload, next) =
                crate::verified::tar_entry_extent::take_selected_entry_payload(
                    bytes,
                    cursor,
                    crate::verified::tar_pax::payload_size(extension, values[2], raw_size),
                    limits.max_entry_bytes,
                )
                .map_err(|error| match error {
                    crate::verified::tar_entry_extent::SelectedEntryError::Numeric => {
                        TarError::Numeric
                    }
                    crate::verified::tar_entry_extent::SelectedEntryError::Limit => TarError::Limit,
                    crate::verified::tar_entry_extent::SelectedEntryError::Truncated => {
                        TarError::Truncated
                    }
                })?;
            cursor = next;
            if extension {
                extended = extended.checked_add(1).ok_or(TarError::Limit)?;
                if extended > limits.max_extended_headers {
                    return Err(TarError::Limit);
                }
                charge(&mut metadata, size, limits.max_metadata_bytes)?;
                //= docs/parsing-requirements.md#parse-tar-006
                //# GNU `L`/`K` and PAX `g` headers MUST NOT consume pending local fields.
                match h.entry_type {
                    b'L' => {
                        long_name = Some(long(payload, limits.max_path_bytes)?);
                    }
                    b'K' => {
                        long_link = Some(long(payload, limits.max_path_bytes)?);
                    }
                    b'x' => {
                        let parsed = pax(payload)?;
                        //= docs/parsing-requirements.md#parse-tar-006
                        //# Each present field in a local `x` header replaces the pending local value, including empty values, while omitted local fields persist across later extension headers.
                        local = crate::verified::tar_pax::apply_local_update(local, parsed);
                    }
                    b'g' => {
                        let parsed = pax(payload)?;
                        //= docs/parsing-requirements.md#parse-tar-006
                        //# Each present field in a global `g` header replaces the prior global value, an empty global value deletes it, and omitted global fields persist.
                        global = crate::verified::tar_pax::apply_global_update(global, parsed);
                    }
                    _ => unreachable!(),
                }
                //= docs/parsing-requirements.md#parse-tar-005
                //# A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling.
                pending = crate::verified::tar_pax::update_pending_override(pending, h.entry_type);
                continue;
            }
            let entry_type = crate::verified::tar_entry::classify_entry_type(h.entry_type, size)
                .ok_or(TarError::UnsupportedEntry)?;
            if entries.len() >= limits.max_entries {
                return Err(TarError::Limit);
            }
            let raw_name = text(h.name);
            let prefix = if posix { text(h.prefix) } else { b"" };
            //= docs/parsing-requirements.md#parse-tar-012
            //# The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.
            let name = crate::verified::tar_pax::effective_name_override(values[0], long_name);
            let header_link = text(h.linkname);
            let link = crate::verified::tar_pax::effective_link_override(values[1], long_link)
                .unwrap_or(header_link);
            //= docs/parsing-requirements.md#parse-tar-012
            //# The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.
            let (name_length, name_admitted) =
                crate::verified::tar_pax::effective_header_name_length_and_admission(
                    name,
                    prefix,
                    raw_name,
                    limits.max_path_bytes,
                )
                .ok_or(TarError::Limit)?;
            //= docs/parsing-requirements.md#parse-tar-013
            //# The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.
            let (link_length, link_admitted) =
                crate::verified::tar_pax::effective_link_length_and_admission(
                    link,
                    limits.max_path_bytes,
                );
            if !name_admitted || !link_admitted {
                return Err(TarError::Limit);
            }
            if crate::verified::tar_pax::owner_name_requires_utf8(values[10]) {
                for i in [0, 1, 6, 7] {
                    if let Some(value) = values[i] {
                        std::str::from_utf8(value).map_err(|_| TarError::TextEncoding)?;
                    }
                }
            }
            let uname = crate::verified::tar_pax::effective_owner_name(values[6], text(h.uname));
            let gname = crate::verified::tar_pax::effective_owner_name(values[7], text(h.gname));
            if uname.len() > limits.max_path_bytes || gname.len() > limits.max_path_bytes {
                return Err(TarError::Limit);
            }
            // Conservative Vec capacity and entry storage accounting before allocation.
            let entry_charge = std::mem::size_of::<TarEntry<'_>>()
                .checked_mul(4)
                .and_then(|v| v.checked_add(name_length))
                .and_then(|v| v.checked_add(link_length))
                .and_then(|v| v.checked_add(uname.len()))
                .and_then(|v| v.checked_add(gname.len()))
                .ok_or(TarError::Limit)?;
            charge(&mut metadata, entry_charge, limits.max_metadata_bytes)?;
            let name = if let Some(value) = name {
                value.to_vec()
            } else {
                //= docs/parsing-requirements.md#parse-tar-009
                //# When PAX path and GNU long-name overrides are absent, a POSIX ustar header MUST use its raw name field if the prefix is empty and MUST join a nonempty prefix to the raw name with one slash byte.
                crate::verified::tar_pax::compose_ustar_header_name(prefix, raw_name)
            };
            if name.is_empty() || name.contains(&0) || link.contains(&0) {
                return Err(TarError::TextEncoding);
            }
            let (mtime_seconds, mtime_nanoseconds) =
                crate::verified::tar_pax::effective_mtime(values[3], header_mtime_seconds)
                    .map_err(|_| TarError::Numeric)?;
            let mtime = TarTime {
                seconds: mtime_seconds,
                nanoseconds: mtime_nanoseconds,
            };
            let uid = crate::verified::tar_pax::effective_id(values[4], header_uid)
                .map_err(|_| TarError::Numeric)?;
            let gid = crate::verified::tar_pax::effective_id(values[5], header_gid)
                .map_err(|_| TarError::Numeric)?;
            let atime = crate::verified::tar_pax::effective_optional_time(values[8])
                .map_err(|_| TarError::Numeric)?
                .map(|(seconds, nanoseconds)| TarTime {
                    seconds,
                    nanoseconds,
                });
            let ctime = crate::verified::tar_pax::effective_optional_time(values[9])
                .map_err(|_| TarError::Numeric)?
                .map(|(seconds, nanoseconds)| TarTime {
                    seconds,
                    nanoseconds,
                });
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
            //= docs/parsing-requirements.md#parse-tar-006
            //# That local state MUST be consumed only after the real entry is accepted.
            local = crate::verified::tar_pax::consume_local_update(local);
            //= docs/parsing-requirements.md#parse-tar-005
            //# A real entry clears pending local PAX or GNU long-name/link override state only after the entry is accepted.
            pending = crate::verified::tar_pax::clear_pending_override(pending);
            long_name = None;
            long_link = None;
        }
        //= docs/parsing-requirements.md#parse-tar-005
        //# A dangling local PAX or GNU long-name/link override MUST produce `DanglingOverride` at EOF.
        if crate::verified::tar_pax::dangling_override_at_eof(pending) {
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

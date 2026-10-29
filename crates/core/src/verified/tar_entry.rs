//! Exact admission of ordinary tar entry type bytes and their payload-size rule.
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarEntryType {
    Regular,
    Directory,
    Symlink,
    Hardlink,
    CharacterDevice,
    BlockDevice,
    Fifo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarLongRecordError {
    Malformed,
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarHeaderProfile {
    PosixUstar,
    Gnu,
    V7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TarUnsignedHeaderFields {
    pub raw_size: u64,
    pub uid: u64,
    pub gid: u64,
    pub device_major: u64,
    pub device_minor: u64,
}

//= docs/parsing-requirements.md#parse-tar-031
//# For each accepted character-device or block-device entry, the reader MUST preserve the decoded raw header `device_major` and `device_minor` values in the corresponding `TarEntry` fields without normalization or substitution.
pub fn retain_device_numbers(device_major: u64, device_minor: u64) -> (retained: (u64, u64))
    ensures retained.0 == device_major && retained.1 == device_minor,
{
    (device_major, device_minor)
}

//= docs/parsing-requirements.md#parse-tar-027
//# After checksum and header-profile admission, the parser MUST admit raw tar `size`, `uid`, `gid`, `device_major`, and `device_minor` fields as nonnegative `u64` values. A negative decoded value or a value greater than `u64::MAX` MUST produce Numeric before payload-extent selection or output-metadata retention. This raw-field admission MUST still occur when an effective PAX `size` supplies a real entry's payload extent; GNU/PAX extension payload extents continue to use their raw header size as specified by PARSE-TAR-007.
pub fn admit_unsigned_header_fields(
    raw_size: &[u8],
    uid: &[u8],
    gid: &[u8],
    device_major: &[u8],
    device_minor: &[u8],
) -> (result: Option<TarUnsignedHeaderFields>)
    ensures match result {
        Some(fields) =>
            super::tar_number::unsigned_model(raw_size@) == Some(fields.raw_size as int)
            && super::tar_number::unsigned_model(uid@) == Some(fields.uid as int)
            && super::tar_number::unsigned_model(gid@) == Some(fields.gid as int)
            && super::tar_number::unsigned_model(device_major@) == Some(fields.device_major as int)
            && super::tar_number::unsigned_model(device_minor@) == Some(fields.device_minor as int),
        None => super::tar_number::unsigned_model(raw_size@) is None
            || super::tar_number::unsigned_model(uid@) is None
            || super::tar_number::unsigned_model(gid@) is None
            || super::tar_number::unsigned_model(device_major@) is None
            || super::tar_number::unsigned_model(device_minor@) is None,
    },
{
    let raw_size = super::tar_number::unsigned(raw_size)?;
    let uid = super::tar_number::unsigned(uid)?;
    let gid = super::tar_number::unsigned(gid)?;
    let device_major = super::tar_number::unsigned(device_major)?;
    let device_minor = super::tar_number::unsigned(device_minor)?;
    Some(TarUnsignedHeaderFields { raw_size, uid, gid, device_major, device_minor })
}

pub open spec fn tar_header_profile_model(magic: Seq<u8>) -> Option<TarHeaderProfile> {
    if magic.len() == 8 && magic[0] == b'u' && magic[1] == b's'
        && magic[2] == b't' && magic[3] == b'a' && magic[4] == b'r'
        && magic[5] == 0 && magic[6] == b'0' && magic[7] == b'0'
    { Some(TarHeaderProfile::PosixUstar) }
    else if magic.len() == 8 && magic[0] == b'u' && magic[1] == b's'
        && magic[2] == b't' && magic[3] == b'a' && magic[4] == b'r'
        && magic[5] == b' ' && magic[6] == b' ' && magic[7] == 0
    { Some(TarHeaderProfile::Gnu) }
    else if magic.len() == 8 && magic[0] == 0 && magic[1] == 0
        && magic[2] == 0 && magic[3] == 0 && magic[4] == 0 && magic[5] == 0
        && magic[6] == 0 && magic[7] == 0
    { Some(TarHeaderProfile::V7) }
    else { None }
}

//= docs/parsing-requirements.md#parse-tar-025
//# After checksum admission, the parser MUST accept a header only when its eight magic/version bytes are exactly POSIX ustar (`ustar\0` followed by `00`), old GNU (`ustar ` followed by ` \0`), or eight zero bytes for V7. Any other magic/version byte sequence MUST produce UnsupportedHeader. Only POSIX ustar headers may interpret bytes 345 through 499 as a path prefix; old GNU and V7 headers MUST NOT use those bytes as a prefix.
pub fn admit_tar_header_profile(magic: &[u8]) -> (profile: Option<TarHeaderProfile>)
    ensures profile == tar_header_profile_model(magic@),
{
    if magic.len() == 8 && magic[0] == b'u' && magic[1] == b's'
        && magic[2] == b't' && magic[3] == b'a' && magic[4] == b'r'
        && magic[5] == 0 && magic[6] == b'0' && magic[7] == b'0'
    {
        Some(TarHeaderProfile::PosixUstar)
    } else if magic.len() == 8 && magic[0] == b'u' && magic[1] == b's'
        && magic[2] == b't' && magic[3] == b'a' && magic[4] == b'r'
        && magic[5] == b' ' && magic[6] == b' ' && magic[7] == 0
    {
        Some(TarHeaderProfile::Gnu)
    } else if magic.len() == 8 && magic[0] == 0 && magic[1] == 0
        && magic[2] == 0 && magic[3] == 0 && magic[4] == 0 && magic[5] == 0
        && magic[6] == 0 && magic[7] == 0
    {
        Some(TarHeaderProfile::V7)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-tar-024
//# After decoding the checksum field, the parser MUST reject a header whose checksum does not match before interpreting magic/version or any other numeric header field; that mismatch MUST produce Checksum.
pub fn admit_header_checksum(matches: bool) -> (admitted: bool)
    ensures admitted == matches,
{
    matches
}

//= docs/parsing-requirements.md#parse-tar-028
//# Before comparing a tar header checksum or interpreting magic/version and other numeric header fields, the parser MUST decode the checksum field using the numeric profile in PARSE-TAR-026 as a nonnegative `u64`. If the checksum field is malformed, negative, or outside that range, the parser MUST return Numeric. A successfully decoded checksum that does not match the header MUST continue to return Checksum as specified by PARSE-TAR-024.
pub fn decode_header_checksum_field(field: &[u8]) -> (result: Option<u64>)
    ensures match result {
        Some(value) => super::tar_number::unsigned_model(field@) == Some(value as int),
        None => super::tar_number::unsigned_model(field@) is None,
    },
{
    super::tar_number::unsigned(field)
}

//= docs/parsing-requirements.md#parse-tar-023
//# GNU `L` and `K` payloads MUST contain a NUL terminator, all bytes from the first NUL onward MUST be zero, and the selected prefix length MUST NOT exceed `max_path_bytes`.
pub open spec fn gnu_long_record_model(input: Seq<u8>, limit: int)
    -> Result<Seq<u8>, TarLongRecordError>
{
    match super::tar_pax::find(input, 0, input.len() as int, 0) {
        None => Err(TarLongRecordError::Malformed),
        Some(end) => {
            if !super::tar_footer::zero_range(input, end, input.len() as int) {
                Err(TarLongRecordError::Malformed)
            } else if end > limit {
                Err(TarLongRecordError::Limit)
            } else {
                Ok(input.subrange(0, end))
            }
        },
    }
}

//= docs/parsing-requirements.md#parse-tar-023
//# GNU `L` and `K` payloads MUST contain a NUL terminator, all bytes from the first NUL onward MUST be zero, and the selected prefix length MUST NOT exceed `max_path_bytes`.
pub fn decode_gnu_long_record(input: &[u8], limit: usize)
    -> (result: Result<&[u8], TarLongRecordError>)
    ensures match result {
        Ok(prefix) => gnu_long_record_model(input@, limit as int) == Ok(prefix@),
        Err(error) => gnu_long_record_model(input@, limit as int) == Err(error),
    },
{
    let end = match super::tar_pax::find_byte(input, 0, input.len(), 0) {
        Some(end) => end,
        None => return Err(TarLongRecordError::Malformed),
    };
    if !super::tar_footer::zeros(input, end, input.len()) {
        return Err(TarLongRecordError::Malformed);
    }
    if end > limit {
        return Err(TarLongRecordError::Limit);
    }
    Ok(&input[..end])
}

pub open spec fn entry_type_model(flag: u8, size: usize) -> Option<TarEntryType> {
    if flag == 0 || flag == b'0' { Some(TarEntryType::Regular) }
    else if size != 0 { None }
    else if flag == b'1' { Some(TarEntryType::Hardlink) }
    else if flag == b'2' { Some(TarEntryType::Symlink) }
    else if flag == b'3' { Some(TarEntryType::CharacterDevice) }
    else if flag == b'4' { Some(TarEntryType::BlockDevice) }
    else if flag == b'5' { Some(TarEntryType::Directory) }
    else if flag == b'6' { Some(TarEntryType::Fifo) }
    else { None }
}

/// Return the supported ordinary entry kind, rejecting unknown types and
/// payload-bearing non-regular entries before the tar controller interprets them.
pub fn classify_entry_type(flag: u8, size: usize) -> (result: Option<TarEntryType>)
    ensures result == entry_type_model(flag, size),
{
    if flag == 0 || flag == b'0' {
        Some(TarEntryType::Regular)
    } else if size != 0 {
        None
    } else {
        match flag {
            b'1' => Some(TarEntryType::Hardlink),
            b'2' => Some(TarEntryType::Symlink),
            b'3' => Some(TarEntryType::CharacterDevice),
            b'4' => Some(TarEntryType::BlockDevice),
            b'5' => Some(TarEntryType::Directory),
            b'6' => Some(TarEntryType::Fifo),
            _ => None,
        }
    }
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarEofState {
    RejectDanglingOverride,
    ValidateFooter,
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarEofError {
    DanglingOverride,
    Termination,
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
pub open spec fn tar_eof_state_model(pending: bool) -> TarEofState {
    if pending { TarEofState::RejectDanglingOverride } else { TarEofState::ValidateFooter }
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
pub fn begin_tar_eof(pending: bool) -> (result: TarEofState)
    ensures result == tar_eof_state_model(pending),
{
    if pending { TarEofState::RejectDanglingOverride } else { TarEofState::ValidateFooter }
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
pub open spec fn finish_tar_eof_model(state: TarEofState, footer_valid: bool) -> Option<TarEofError> {
    match state {
        TarEofState::RejectDanglingOverride => Some(TarEofError::DanglingOverride),
        TarEofState::ValidateFooter => if footer_valid { None } else { Some(TarEofError::Termination) },
    }
}

//= docs/parsing-requirements.md#parse-tar-022
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
pub fn finish_tar_eof(state: TarEofState, footer_valid: bool) -> (result: Option<TarEofError>)
    ensures result == finish_tar_eof_model(state, footer_valid),
{
    match state {
        TarEofState::RejectDanglingOverride => Some(TarEofError::DanglingOverride),
        TarEofState::ValidateFooter => if footer_valid { None } else { Some(TarEofError::Termination) },
    }
}

//= docs/parsing-requirements.md#parse-tar-018
//# For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.
pub open spec fn entry_metadata_charge_model(
    entry_size: int,
    copies: int,
    name_length: int,
    link_length: int,
    uname_length: int,
    gname_length: int,
) -> int {
    entry_size * copies + name_length + link_length + uname_length + gname_length
}

//= docs/parsing-requirements.md#parse-tar-018
//# For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.
pub fn entry_metadata_charge(
    entry_size: usize,
    copies: usize,
    name_length: usize,
    link_length: usize,
    uname_length: usize,
    gname_length: usize,
) -> (result: Option<usize>)
    ensures match result {
        Some(amount) => amount as int == entry_metadata_charge_model(
            entry_size as int, copies as int, name_length as int,
            link_length as int, uname_length as int, gname_length as int,
        ),
        None => entry_metadata_charge_model(
            entry_size as int, copies as int, name_length as int,
            link_length as int, uname_length as int, gname_length as int,
        ) > usize::MAX as int,
    },
{
    let fixed = entry_size.checked_mul(copies)?;
    let with_name = fixed.checked_add(name_length)?;
    let with_link = with_name.checked_add(link_length)?;
    let with_uname = with_link.checked_add(uname_length)?;
    with_uname.checked_add(gname_length)
}

//= docs/parsing-requirements.md#parse-tar-018
//# For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.
pub open spec fn metadata_charge_admission_model(
    used: int,
    amount: int,
    limit: int,
) -> Option<int> {
    let next = used + amount;
    if next <= usize::MAX as int && next <= limit { Some(next) } else { None }
}

//= docs/parsing-requirements.md#parse-tar-018
//# For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.
pub fn admit_metadata_charge(used: usize, amount: usize, limit: usize)
    -> (result: Option<usize>)
    ensures match result {
        Some(next) => metadata_charge_admission_model(
            used as int, amount as int, limit as int,
        ) == Some(next as int),
        None => metadata_charge_admission_model(
            used as int, amount as int, limit as int,
        ) is None,
    },
{
    let next = used.checked_add(amount)?;
    if next > limit { None } else { Some(next) }
}

//= docs/parsing-requirements.md#parse-tar-019
//# GNU `L`/`K` and PAX `x`/`g` extension headers MUST charge their raw-header payload size against the cumulative metadata limit before interpreting or retaining extension metadata. Arithmetic overflow or an exceeded limit MUST reject the volume without returning partial entries.
pub fn admit_extension_metadata_charge(used: usize, raw_header_size: usize, limit: usize)
    -> (result: Option<usize>)
    ensures match result {
        Some(next) => metadata_charge_admission_model(
            used as int, raw_header_size as int, limit as int,
        ) == Some(next as int),
        None => metadata_charge_admission_model(
            used as int, raw_header_size as int, limit as int,
        ) is None,
    },
{
    admit_metadata_charge(used, raw_header_size, limit)
}

//= docs/parsing-requirements.md#parse-tar-020
//# The reader MUST checked-increment the GNU/PAX extension-header count and reject the header if the increment overflows or exceeds `max_extended_headers`, before charging or interpreting extension contents.
pub open spec fn extension_header_count_model(current: int, maximum: int) -> Option<int> {
    let next = current + 1;
    if next <= usize::MAX as int && next <= maximum { Some(next) } else { None }
}

//= docs/parsing-requirements.md#parse-tar-020
//# The reader MUST checked-increment the GNU/PAX extension-header count and reject the header if the increment overflows or exceeds `max_extended_headers`, before charging or interpreting extension contents.
pub fn admit_extension_header_count(current: usize, maximum: usize) -> (result: Option<usize>)
    ensures match result {
        Some(next) => extension_header_count_model(current as int, maximum as int)
            == Some(next as int),
        None => extension_header_count_model(current as int, maximum as int) is None,
    },
{
    let next = current.checked_add(1)?;
    if next > maximum { None } else { Some(next) }
}

//= docs/parsing-requirements.md#parse-tar-021
//# Before allocating or retaining output fields for a real tar entry, the parser MUST require the accepted entry count to be below `max_entries`. If the count equals or exceeds the limit, it MUST return Limit without returning a partial entry list.
pub open spec fn real_entry_count_admission_model(current: int, maximum: int) -> bool {
    current < maximum
}

//= docs/parsing-requirements.md#parse-tar-021
//# Before allocating or retaining output fields for a real tar entry, the parser MUST require the accepted entry count to be below `max_entries`. If the count equals or exceeds the limit, it MUST return Limit without returning a partial entry list.
pub fn admit_real_entry_count(current: usize, maximum: usize) -> (result: bool)
    ensures result == real_entry_count_admission_model(current as int, maximum as int),
{
    current < maximum
}

#[cfg(test)]
mod metadata_charge_tests {
    use super::{
        admit_extension_header_count, admit_extension_metadata_charge, admit_tar_header_profile,
        admit_unsigned_header_fields,
        decode_header_checksum_field,
        admit_metadata_charge, admit_real_entry_count, begin_tar_eof, finish_tar_eof,
        admit_header_checksum, decode_gnu_long_record, entry_metadata_charge, TarEofError, TarEofState,
        retain_device_numbers, TarHeaderProfile, TarLongRecordError, TarUnsignedHeaderFields,
    };

    //= docs/parsing-requirements.md#parse-tar-028
    //= type=test
    //# Before comparing a tar header checksum or interpreting magic/version and other numeric header fields, the parser MUST decode the checksum field using the numeric profile in PARSE-TAR-026 as a nonnegative `u64`. If the checksum field is malformed, negative, or outside that range, the parser MUST return Numeric. A successfully decoded checksum that does not match the header MUST continue to return Checksum as specified by PARSE-TAR-024.
    #[test]
    fn header_checksum_field_requires_unsigned_numeric_decode() {
        assert_eq!(decode_header_checksum_field(b"0000123\0"), Some(83));
        assert_eq!(decode_header_checksum_field(&[0xff; 8]), None);
        assert_eq!(decode_header_checksum_field(b"00000008"), None);
        assert_eq!(
            decode_header_checksum_field(&[0x80, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            None
        );
    }

    //= docs/parsing-requirements.md#parse-tar-027
    //= type=test
    //# After checksum and header-profile admission, the parser MUST admit raw tar `size`, `uid`, `gid`, `device_major`, and `device_minor` fields as nonnegative `u64` values. A negative decoded value or a value greater than `u64::MAX` MUST produce Numeric before payload-extent selection or output-metadata retention. This raw-field admission MUST still occur when an effective PAX `size` supplies a real entry's payload extent; GNU/PAX extension payload extents continue to use their raw header size as specified by PARSE-TAR-007.
    #[test]
    fn unsigned_header_field_adapter_admits_all_fields_or_none() {
        assert_eq!(
            admit_unsigned_header_fields(b"0000001", b"2", b"3", b"4", b"5"),
            Some(TarUnsignedHeaderFields {
                raw_size: 1,
                uid: 2,
                gid: 3,
                device_major: 4,
                device_minor: 5,
            })
        );
        let invalid: &[u8] = &[0xff, 0xff];
        assert_eq!(admit_unsigned_header_fields(invalid, b"2", b"3", b"4", b"5"), None);
        assert_eq!(admit_unsigned_header_fields(b"1", invalid, b"3", b"4", b"5"), None);
        assert_eq!(admit_unsigned_header_fields(b"1", b"2", invalid, b"4", b"5"), None);
        assert_eq!(admit_unsigned_header_fields(b"1", b"2", b"3", invalid, b"5"), None);
        assert_eq!(admit_unsigned_header_fields(b"1", b"2", b"3", b"4", invalid), None);
    }

    //= docs/parsing-requirements.md#parse-tar-025
    //= type=test
    //# After checksum admission, the parser MUST accept a header only when its eight magic/version bytes are exactly POSIX ustar (`ustar\0` followed by `00`), old GNU (`ustar ` followed by ` \0`), or eight zero bytes for V7. Any other magic/version byte sequence MUST produce UnsupportedHeader. Only POSIX ustar headers may interpret bytes 345 through 499 as a path prefix; old GNU and V7 headers MUST NOT use those bytes as a prefix.
    #[test]
    fn header_profile_admission_is_exact() {
        assert_eq!(admit_tar_header_profile(b"ustar\x0000"), Some(TarHeaderProfile::PosixUstar));
        assert_eq!(admit_tar_header_profile(b"ustar  \0"), Some(TarHeaderProfile::Gnu));
        assert_eq!(admit_tar_header_profile(&[0; 8]), Some(TarHeaderProfile::V7));
        for magic in [
            *b"ustar\x0001",
            *b"ustar\x0100",
            *b"ustar  0",
            *b"ustar   ",
            *b"USTAR\x0000",
        ] {
            assert_eq!(admit_tar_header_profile(&magic), None);
        }
    }

    //= docs/parsing-requirements.md#parse-tar-031
    //= type=test
    //# For each accepted character-device or block-device entry, the reader MUST preserve the decoded raw header `device_major` and `device_minor` values in the corresponding `TarEntry` fields without normalization or substitution.
    #[test]
    fn device_number_metadata_is_retained_exactly() {
        assert_eq!(retain_device_numbers(17, 29), (17, 29));
    }

    #[test]
    fn entry_metadata_charge_checked_formula_and_budget_admission() {
        assert_eq!(entry_metadata_charge(8, 4, 3, 2, 1, 4), Some(42));
        assert_eq!(entry_metadata_charge(usize::MAX, 2, 0, 0, 0, 0), None);
        assert_eq!(entry_metadata_charge(0, 1, usize::MAX, 1, 0, 0), None);
        assert_eq!(admit_metadata_charge(10, 32, 42), Some(42));
        assert_eq!(admit_metadata_charge(10, 32, 41), None);
        assert_eq!(admit_metadata_charge(usize::MAX, 1, usize::MAX), None);
    }

    //= docs/parsing-requirements.md#parse-tar-019
    //= type=test
    //# GNU `L`/`K` and PAX `x`/`g` extension headers MUST charge their raw-header payload size against the cumulative metadata limit before interpreting or retaining extension metadata. Arithmetic overflow or an exceeded limit MUST reject the volume without returning partial entries.
    #[test]
    fn extension_metadata_charge_uses_raw_size_and_remaining_limit() {
        assert_eq!(admit_extension_metadata_charge(12, 7, 19), Some(19));
        assert_eq!(admit_extension_metadata_charge(12, 7, 18), None);
        assert_eq!(admit_extension_metadata_charge(usize::MAX, 1, usize::MAX), None);
    }

    //= docs/parsing-requirements.md#parse-tar-020
    //= type=test
    //# The reader MUST checked-increment the GNU/PAX extension-header count and reject the header if the increment overflows or exceeds `max_extended_headers`, before charging or interpreting extension contents.
    #[test]
    fn extension_header_count_checks_increment_and_configured_limit() {
        assert_eq!(admit_extension_header_count(0, 1), Some(1));
        assert_eq!(admit_extension_header_count(usize::MAX - 1, usize::MAX), Some(usize::MAX));
        assert_eq!(admit_extension_header_count(1, 1), None);
        assert_eq!(admit_extension_header_count(usize::MAX, usize::MAX), None);
    }

    //= docs/parsing-requirements.md#parse-tar-021
    //= type=test
    //# Before allocating or retaining output fields for a real tar entry, the parser MUST require the accepted entry count to be below `max_entries`. If the count equals or exceeds the limit, it MUST return Limit without returning a partial entry list.
    #[test]
    fn real_entry_count_admission_checks_below_limit() {
        assert!(admit_real_entry_count(0, 1));
        assert!(!admit_real_entry_count(1, 1));
        assert!(!admit_real_entry_count(usize::MAX, usize::MAX));
    }

    //= docs/parsing-requirements.md#parse-tar-024
    //= type=test
    //# After decoding the checksum field, the parser MUST reject a header whose checksum does not match before interpreting magic/version or any other numeric header field; that mismatch MUST produce Checksum.
    #[test]
    fn header_checksum_gate_admits_only_matching_checksum() {
        assert!(admit_header_checksum(true));
        assert!(!admit_header_checksum(false));
    }

    //= docs/parsing-requirements.md#parse-tar-022
    //= type=test
    //# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
    #[test]
    fn tar_eof_decision_preserves_dangling_precedence_and_requires_valid_footer() {
        let dangling = begin_tar_eof(true);
        assert_eq!(dangling, TarEofState::RejectDanglingOverride);
        assert_eq!(finish_tar_eof(dangling, false), Some(TarEofError::DanglingOverride));
        assert_eq!(finish_tar_eof(dangling, true), Some(TarEofError::DanglingOverride));

        let footer = begin_tar_eof(false);
        assert_eq!(footer, TarEofState::ValidateFooter);
        assert_eq!(finish_tar_eof(footer, false), Some(TarEofError::Termination));
        assert_eq!(finish_tar_eof(footer, true), None);
    }

    //= docs/parsing-requirements.md#parse-tar-023
    //= type=test
    //# GNU `L` and `K` payloads MUST contain a NUL terminator, all bytes from the first NUL onward MUST be zero, and the selected prefix length MUST NOT exceed `max_path_bytes`.
    #[test]
    fn gnu_long_record_requires_zero_terminated_bounded_payload() {
        assert_eq!(decode_gnu_long_record(b"name\0", 4), Ok(&b"name"[..]));
        assert_eq!(decode_gnu_long_record(b"name\0\0\0", 4), Ok(&b"name"[..]));
        assert_eq!(decode_gnu_long_record(b"longer\0\0", 5), Err(TarLongRecordError::Limit));
        assert_eq!(decode_gnu_long_record(b"missing terminator", 100), Err(TarLongRecordError::Malformed));
        assert_eq!(decode_gnu_long_record(b"name\0x", 100), Err(TarLongRecordError::Malformed));
        assert_eq!(decode_gnu_long_record(b"long\0x", 1), Err(TarLongRecordError::Malformed));
    }
}

}

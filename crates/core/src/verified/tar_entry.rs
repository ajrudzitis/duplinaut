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
        admit_extension_header_count, admit_extension_metadata_charge,
        admit_metadata_charge, admit_real_entry_count, begin_tar_eof, finish_tar_eof,
        decode_gnu_long_record, entry_metadata_charge, TarEofError, TarEofState,
        TarLongRecordError,
    };

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

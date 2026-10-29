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

#[cfg(test)]
mod metadata_charge_tests {
    use super::{admit_extension_metadata_charge, admit_metadata_charge, entry_metadata_charge};

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
}

}

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
}

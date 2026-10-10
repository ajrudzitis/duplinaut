//! Byte identities for paths in a backup. Rendered terminal text is not identity.
use vstd::prelude::*;

verus! {
/// Lexicographic byte order, including the shorter-prefix rule.
pub open spec fn bytes_less(left: Seq<u8>, right: Seq<u8>) -> bool {
    (left.len() < right.len() && left == right.take(left.len() as int))
    || exists|i: int| 0 <= i < left.len() && i < right.len()
        && left.take(i) == right.take(i) && left[i] < right[i]
}

/// Lexicographic order on components, rather than on slash-joined rendering.
pub open spec fn components_less(left: Seq<Seq<u8>>, right: Seq<Seq<u8>>) -> bool {
    (left.len() < right.len() && left == right.take(left.len() as int))
    || exists|i: int| 0 <= i < left.len() && i < right.len()
        && left.take(i) == right.take(i) && bytes_less(left[i], right[i])
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackupPath { components: Vec<Vec<u8>> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError { Absolute, EmptyComponent, DotComponent, ParentComponent, Nul, ByteLimit, ComponentLimit }

impl BackupPath {
    pub closed spec fn view(&self) -> Seq<Seq<u8>> {
        self.components@.map_values(|component: Vec<u8>| component@)
    }
    /// Root markers are decoded here; one trailing separator is allowed only
    /// when the container explicitly identifies a directory.
    //= docs/parsing-requirements.md#parse-path-001
    //# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
    //= docs/parsing-requirements.md#parse-path-002
    //# A restore path MUST be relative to the backup root and contain no NUL, empty interior component, dot component, or parent component after format-specific root decoding.
    pub fn parse(bytes: &[u8], directory_suffix: bool, max_bytes: usize, max_components: usize) -> Result<Self, PathError> {
        if bytes.len() > max_bytes { return Err(PathError::ByteLimit); }
        if bytes.len() == 0 || (bytes.len() == 1 && bytes[0] == b'.')
            || (directory_suffix && bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'/') {
            return Ok(Self { components: Vec::new() });
        }
        if bytes[0] == b'/' { return Err(PathError::Absolute); }
        let mut end = bytes.len();
        if directory_suffix && bytes[end - 1] == b'/' { end -= 1; }
        let mut start = 0;
        let mut cursor = 0;
        let mut components: Vec<Vec<u8>> = Vec::new();
        loop
            invariant
                0 < end <= bytes.len(),
                start <= cursor <= end,
                components.len() <= max_components,
            decreases end - cursor,
        {
            if cursor == end || bytes[cursor] == b'/' {
                let count = cursor - start;
                if count == 0 { return Err(PathError::EmptyComponent); }
                if count == 1 && bytes[start] == b'.' { return Err(PathError::DotComponent); }
                if count == 2 && bytes[start] == b'.' && bytes[start + 1] == b'.' {
                    return Err(PathError::ParentComponent);
                }
                if components.len() == max_components { return Err(PathError::ComponentLimit); }
                let mut component: Vec<u8> = Vec::new();
                let mut index = start;
                while index < cursor
                    invariant start <= index <= cursor <= end <= bytes.len(),
                    decreases cursor - index,
                {
                    component.push(bytes[index]);
                    index += 1;
                }
                components.push(component);
                if cursor == end { break; }
                start = cursor + 1;
            } else if bytes[cursor] == 0 { return Err(PathError::Nul); }
            cursor += 1;
        }
        Ok(Self { components })
    }

    pub fn components(&self) -> (result: &Vec<Vec<u8>>)
        ensures result@.map_values(|component: Vec<u8>| component@) == self@,
    { &self.components }

    /// Compare byte-component sequences, independently of terminal rendering.
    /// Returns -1, 0, or 1. This kernel supplies manifest range comparisons.
    pub fn compare(&self, other: &Self) -> (result: i8)
        ensures
            result == -1 || result == 0 || result == 1,
            result == -1 ==> components_less(self@, other@),
            result == 0 ==> self@ == other@,
            result == 1 ==> components_less(other@, self@),
    {
        reveal(BackupPath::view);
        let mut component_index = 0;
        while component_index < self.components.len() && component_index < other.components.len()
            invariant
                component_index <= self.components.len(),
                component_index <= other.components.len(),
                self@.take(component_index as int) == other@.take(component_index as int),
            decreases self.components.len() - component_index,
        {
            let left = &self.components[component_index];
            let right = &other.components[component_index];
            let mut byte_index = 0;
            while byte_index < left.len() && byte_index < right.len()
                invariant
                    component_index < self.components.len(),
                    component_index < other.components.len(),
                    self@.take(component_index as int) == other@.take(component_index as int),
                    left@ == self@[component_index as int],
                    right@ == other@[component_index as int],
                    byte_index <= left.len(),
                    byte_index <= right.len(),
                    left@.take(byte_index as int) == right@.take(byte_index as int),
                decreases left.len() - byte_index,
            {
                if left[byte_index] < right[byte_index] { return -1; }
                if left[byte_index] > right[byte_index] { return 1; }
                proof {
                    assert(left@.take(byte_index as int + 1) =~= right@.take(byte_index as int + 1));
                }
                byte_index += 1;
            }
            if left.len() < right.len() {
                proof {
                    assert(left@.take(left@.len() as int) =~= left@);
                    assert(bytes_less(left@, right@));
                    assert(components_less(self@, other@)) by {
                        assert(self@.take(component_index as int) == other@.take(component_index as int));
                    }
                }
                return -1;
            }
            if left.len() > right.len() {
                proof {
                    assert(right@.take(right@.len() as int) =~= right@);
                    assert(bytes_less(right@, left@));
                    assert(components_less(other@, self@));
                }
                return 1;
            }
            proof {
                assert(left@ =~= right@);
                assert(self@.take(component_index as int + 1) =~= other@.take(component_index as int + 1));
            }
            component_index += 1;
        }
        proof {
            if component_index == self.components.len() {
                assert(self@.take(component_index as int) =~= self@);
            }
            if component_index == other.components.len() {
                assert(other@.take(component_index as int) =~= other@);
            }
        }
        if self.components.len() < other.components.len() { -1 }
        else if self.components.len() > other.components.len() { 1 }
        else { 0 }
    }
}
}

impl std::fmt::Display for BackupPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.components.is_empty() { return f.write_str("."); }
        for (index, component) in self.components.iter().enumerate() {
            if index != 0 { f.write_str("/")?; }
            for byte in component {
                match byte {
                    0x21..=0x7e if *byte != b'\\' => write!(f, "{}", char::from(*byte))?,
                    _ => write!(f, "\\x{byte:02x}")?,
                }
            }
        }
        Ok(())
    }
}

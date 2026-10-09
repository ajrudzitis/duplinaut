//! Byte identities for paths in a backup. Rendered terminal text is not identity.
use vstd::prelude::*;

verus! {
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackupPath { components: Vec<Vec<u8>> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError { Absolute, EmptyComponent, DotComponent, ParentComponent, Nul, ByteLimit, ComponentLimit }

impl BackupPath {
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
        loop {
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
                while index < cursor {
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

    pub fn components(&self) -> &Vec<Vec<u8>> { &self.components }
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

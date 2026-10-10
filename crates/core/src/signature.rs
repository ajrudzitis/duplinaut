//! Read-only librsync signatures. Header framing comes from the Vest DSL;
//! algorithm bounds and record iteration await formal controller refinement.
use crate::generated::signature::SignatureHeaderFmt;
use vest_lib::core::exec::parser::Parser;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeakChecksum {
    Rollsum,
    RabinKarp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrongChecksum {
    Md4,
    Blake2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureHeader {
    pub weak: WeakChecksum,
    pub strong: StrongChecksum,
    pub block_length: u32,
    pub strong_length: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct SignatureLimits {
    pub max_bytes: usize,
    pub max_records: usize,
}

impl Default for SignatureLimits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_records: 10_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureError {
    Limit,
    TruncatedHeader,
    UnsupportedMagic,
    InvalidBlockLength,
    InvalidStrongLength,
    IncompleteRecord,
}

/// Validated signature metadata and borrowed checksum records. No allocation.
#[derive(Clone, Copy, Debug)]
pub struct Signature<'a> {
    header: SignatureHeader,
    records: &'a [u8],
    record_length: usize,
}

impl<'a> Signature<'a> {
    //= docs/parsing-requirements.md#parse-sig-001
    //# The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

    //= docs/parsing-requirements.md#parse-sig-002
    //# The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

    // Keep the wire requirement explicit as a remainder for forthcoming
    // controller refinement; no formal result is claimed by this annotation.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn parse(bytes: &'a [u8], limits: SignatureLimits) -> Result<Self, SignatureError> {
        if bytes.len() > limits.max_bytes {
            return Err(SignatureError::Limit);
        }
        if bytes.len() < 12 {
            return Err(SignatureError::TruncatedHeader);
        }
        let (consumed, header) = SignatureHeaderFmt
            .parse(&bytes)
            .map_err(|_| SignatureError::UnsupportedMagic)?;
        if consumed != 12 {
            return Err(SignatureError::TruncatedHeader);
        }
        let (weak, strong) = match header.magic {
            0x72730136 => (WeakChecksum::Rollsum, StrongChecksum::Md4),
            0x72730137 => (WeakChecksum::Rollsum, StrongChecksum::Blake2),
            0x72730146 => (WeakChecksum::RabinKarp, StrongChecksum::Md4),
            0x72730147 => (WeakChecksum::RabinKarp, StrongChecksum::Blake2),
            _ => return Err(SignatureError::UnsupportedMagic),
        };
        if header.block_length == 0 {
            return Err(SignatureError::InvalidBlockLength);
        }
        let maximum = if strong == StrongChecksum::Md4 {
            16
        } else {
            32
        };
        if header.strong_length == 0 || header.strong_length > maximum {
            return Err(SignatureError::InvalidStrongLength);
        }
        let record_length = 4 + header.strong_length as usize;
        let records = &bytes[consumed..];
        if records.len() % record_length != 0 {
            return Err(SignatureError::IncompleteRecord);
        }
        if records.len() / record_length > limits.max_records {
            return Err(SignatureError::Limit);
        }
        Ok(Self {
            header: SignatureHeader {
                weak,
                strong,
                block_length: header.block_length,
                strong_length: header.strong_length,
            },
            records,
            record_length,
        })
    }

    pub fn header(&self) -> SignatureHeader {
        self.header
    }
    pub fn record_count(&self) -> usize {
        self.records.len() / self.record_length
    }

    //= docs/parsing-requirements.md#parse-sig-003
    //# The reader MUST represent original regular-file size as unknown when only a signature payload provides size information.

    /// The final source block length is absent from this format. Even zero
    /// records are exposed without inferring an exact original file size.
    pub fn original_size(&self) -> Option<u64> {
        None
    }

    pub fn records(&self) -> SignatureRecords<'a> {
        SignatureRecords {
            chunks: self.records.chunks_exact(self.record_length),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureRecord<'a> {
    pub weak: u32,
    pub strong: &'a [u8],
}

/// Records borrow the original input, retaining the exact truncated strong sum.
pub struct SignatureRecords<'a> {
    chunks: std::slice::ChunksExact<'a, u8>,
}

impl<'a> Iterator for SignatureRecords<'a> {
    type Item = SignatureRecord<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(|record| SignatureRecord {
            weak: u32::from_be_bytes([record[0], record[1], record[2], record[3]]),
            strong: &record[4..],
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}
impl ExactSizeIterator for SignatureRecords<'_> {}
impl std::iter::FusedIterator for SignatureRecords<'_> {}

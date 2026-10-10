//! Read-only librsync signatures. Header framing comes from the Vest DSL;
//! exact header, algorithm bounds and cursor exposure have checked refinements.
use crate::generated::signature::SignatureHeaderFmt;
#[cfg(verus_keep_ghost)]
use crate::generated::signature::{
    lemma_signature_header_refinement, signature_wire_header, signature_word,
};
use vest_lib::core::exec::parser::Parser;
#[cfg(verus_keep_ghost)]
use vest_lib::core::{proof::*, spec::*};
use vstd::prelude::*;

verus! {

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

// Published semantics and exact diagnostic precedence, independent of Vest.
// No signature record count is interpreted as an original byte length.
pub open spec fn signature_model(input: Seq<u8>, max_bytes: int, max_records: int)
    -> Result<(SignatureHeader, Seq<u8>, int), SignatureError> {
    if input.len() > max_bytes { Err(SignatureError::Limit) }
    else if input.len() < 12 { Err(SignatureError::TruncatedHeader) }
    else { match signature_wire_header(input) {
        None => Err(SignatureError::UnsupportedMagic),
        Some((magic, block, strong_len)) => {
            let weak = if magic == 0x72730136 || magic == 0x72730137 { WeakChecksum::Rollsum } else { WeakChecksum::RabinKarp };
            let strong = if magic == 0x72730136 || magic == 0x72730146 { StrongChecksum::Md4 } else { StrongChecksum::Blake2 };
            let width = 4 + strong_len as int;
            if block == 0 { Err(SignatureError::InvalidBlockLength) }
            else if strong_len == 0 || strong_len > (if strong == StrongChecksum::Md4 { 16u32 } else { 32u32 }) {
                Err(SignatureError::InvalidStrongLength)
            } else if (input.len() - 12) % width != 0 { Err(SignatureError::IncompleteRecord) }
            else if (input.len() - 12) / width > max_records { Err(SignatureError::Limit) }
            else { Ok((SignatureHeader { weak, strong, block_length: block, strong_length: strong_len }, input.subrange(12, input.len() as int), width)) }
        }
    }}
}

impl<'a> Signature<'a> {
    pub closed spec fn valid(&self) -> bool {
        5 <= self.record_length <= 36 && self.records@.len() as int % self.record_length as int == 0
    }
    pub closed spec fn meaning(&self) -> (SignatureHeader, Seq<u8>, int) {
        (self.header, self.records@, self.record_length as int)
    }

    //= docs/parsing-requirements.md#parse-sig-001
    //# The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

    //= docs/parsing-requirements.md#parse-sig-002
    //# The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

    // The explicit remainder check is shared with the independent wire model.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn parse(bytes: &'a [u8], limits: SignatureLimits) -> (result: Result<Self, SignatureError>)
        ensures match result {
            Ok(value) => value.valid() && signature_model(bytes@, limits.max_bytes as int, limits.max_records as int) == Ok(value.meaning()),
            Err(error) => signature_model(bytes@, limits.max_bytes as int, limits.max_records as int) == Err(error),
        },
    {
        proof { lemma_signature_header_refinement(bytes@); SignatureHeaderFmt.lemma_parse_safe(bytes@); }
        if bytes.len() > limits.max_bytes {
            return Err(SignatureError::Limit);
        }
        if bytes.len() < 12 {
            return Err(SignatureError::TruncatedHeader);
        }
        let (consumed, header) = match SignatureHeaderFmt.parse(&bytes) {
            Ok(value) => value,
            Err(_) => return Err(SignatureError::UnsupportedMagic),
        };
        proof {
            header.lemma_deep_view_fields();
            assert(signature_wire_header(bytes@) == Some((header.magic, header.block_length, header.strong_length)));
        }
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
        let maximum = match strong { StrongChecksum::Md4 => 16u32, StrongChecksum::Blake2 => 32u32 };
        proof {
            assert(maximum == (if strong == StrongChecksum::Md4 { 16u32 } else { 32u32 }));
        }
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

    pub fn header(&self) -> (result: SignatureHeader)
        ensures result == self.meaning().0,
    {
        self.header
    }
    pub fn record_count(&self) -> (result: usize)
        requires self.valid(),
        ensures result == self.meaning().1.len() as int / self.meaning().2,
    {
        self.records.len() / self.record_length
    }

    //= docs/parsing-requirements.md#parse-sig-003
    //# The reader MUST represent original regular-file size as unknown when only a signature payload provides size information.

    /// The final source block length is absent from this format. Even zero
    /// records are exposed without inferring an exact original file size.
    pub fn original_size(&self) -> (result: Option<u64>)
        ensures result is None,
    {
        None
    }

    pub fn records(&self) -> (result: SignatureRecords<'a>)
        requires self.valid(),
        ensures result.valid(), result.data() == self.meaning().1, result.cursor() == 0,
            result.width() == self.meaning().2,
    {
        SignatureRecords {
            records: self.records,
            record_length: self.record_length,
            offset: 0,
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
    records: &'a [u8],
    record_length: usize,
    offset: usize,
}

impl<'a> SignatureRecords<'a> {
    pub closed spec fn data(&self) -> Seq<u8> { self.records@ }
    pub closed spec fn cursor(&self) -> int { self.offset as int }
    pub closed spec fn width(&self) -> int { self.record_length as int }
    pub closed spec fn valid(&self) -> bool {
        5 <= self.record_length <= 36 && self.offset <= self.records.len()
            && self.records@.len() as int % self.record_length as int == 0
            && self.offset as int % self.record_length as int == 0
    }

    pub fn next_record(&mut self) -> (result: Option<SignatureRecord<'a>>)
        requires old(self).valid(),
        ensures final(self).valid(), final(self).data() == old(self).data(), final(self).width() == old(self).width(),
            match result {
                None => old(self).cursor() == old(self).data().len() && final(self).cursor() == old(self).cursor(),
                Some(record) => old(self).cursor() < old(self).data().len()
                    && final(self).cursor() == old(self).cursor() + old(self).width()
                    && record.weak == signature_word(old(self).data(), old(self).cursor())
                    && record.strong@ == old(self).data().subrange(old(self).cursor() + 4, final(self).cursor()),
            },
    {
        if self.offset == self.records.len() { return None; }
        proof {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(self.records.len() as int, self.record_length as int);
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(self.offset as int, self.record_length as int);
            let n = self.records.len() as int;
            let p = self.offset as int;
            let w = self.record_length as int;
            assert(0 <= p < n && w >= 5 && n % w == 0 && p % w == 0);
            assert(p + w <= n) by (nonlinear_arith)
                requires 0 <= p < n, w >= 5, n % w == 0, p % w == 0,
                    n == w * (n / w), p == w * (p / w);
            vstd::arithmetic::div_mod::lemma_mod_adds(p, w, w);
            vstd::arithmetic::div_mod::lemma_mod_self_0(w);
            assert((p + w) % w == 0);
        }
        let start = self.offset;
        self.offset += self.record_length;
        let record = &self.records[start..self.offset];
        let weak = ((record[0] as u32) << 24) | ((record[1] as u32) << 16)
            | ((record[2] as u32) << 8) | record[3] as u32;
        Some(SignatureRecord { weak, strong: &record[4..] })
    }
    pub fn remaining(&self) -> (result: usize)
        requires self.valid(),
        ensures result == (self.data().len() - self.cursor()) / self.width(),
    {
        (self.records.len() - self.offset) / self.record_length
    }
}
}

// Standard-library trait dispatch is a thin native adapter. The inherent
// cursor kernel above is the same-source verified implementation; this adapter
// is independently source-reviewed rather than claimed as a Verus trait proof.
impl<'a> Iterator for SignatureRecords<'a> {
    type Item = SignatureRecord<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_record()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.remaining();
        (count, Some(count))
    }
}
impl ExactSizeIterator for SignatureRecords<'_> {}
impl std::iter::FusedIterator for SignatureRecords<'_> {}

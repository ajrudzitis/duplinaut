//! Stored-object integrity and bounded native plain/gzip decoding.
//!
//! RustCrypto digest primitives, flate2's gzip/DEFLATE implementation and their
//! allocation/runtime behavior are trusted adapters, not formally proved here.
//! SHA1/MD5 provide format compatibility, not authenticity. No external process
//! is invoked. Metadata has no manifest volume digest; volume callers must use
//! `decode_verified_volume`, which validates every supported declaration first.
use crate::manifest::{DeclaredDigest, DigestAlgorithm};
use crate::names::Envelope;
use sha1::Digest;
use std::io::Read;

#[derive(Clone, Copy, Debug)]
pub struct EnvelopeLimits {
    pub stored_bytes: usize,
    pub decoded_bytes: usize,
    pub gzip_members: usize,
}
impl Default for EnvelopeLimits {
    fn default() -> Self {
        Self {
            stored_bytes: 8 * 1024 * 1024 * 1024,
            decoded_bytes: 8 * 1024 * 1024 * 1024,
            gzip_members: 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    StoredLimit,
    DecodedLimit,
    MemberLimit,
    Allocation,
    InvalidGzip,
    InvalidDigest,
    ConflictingDigest,
    DigestMismatch,
    UnsupportedIntegrity,
    UnsupportedEnvelope,
}
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
fn digest_matches(hex: &[u8], digest: &[u8]) -> Result<bool, EnvelopeError> {
    if hex.len() != digest.len() * 2 {
        return Err(EnvelopeError::InvalidDigest);
    }
    let mut equal = true;
    for (pair, byte) in hex.as_chunks::<2>().0.iter().zip(digest) {
        let high = hex_value(pair[0]).ok_or(EnvelopeError::InvalidDigest)?;
        let low = hex_value(pair[1]).ok_or(EnvelopeError::InvalidDigest)?;
        equal &= high * 16 + low == *byte;
    }
    Ok(equal)
}

//= docs/parsing-requirements.md#parse-env-005
//= type=implementation
//# Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.
//= docs/parsing-requirements.md#parse-env-006
//= type=implementation
//# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
pub fn verify_stored_digests(
    stored: &[u8],
    declarations: &[DeclaredDigest],
    stored_limit: usize,
) -> Result<(), EnvelopeError> {
    if stored.len() > stored_limit {
        return Err(EnvelopeError::StoredLimit);
    }
    let mut sha1: Option<&[u8]> = None;
    let mut md5: Option<&[u8]> = None;
    let actual_sha1 = sha1::Sha1::digest(stored);
    let actual_md5 = md5::Md5::digest(stored);
    let mut supported = false;
    for declaration in declarations {
        let slot = match declaration.algorithm {
            DigestAlgorithm::Sha1 => &mut sha1,
            DigestAlgorithm::Md5 => &mut md5,
            DigestAlgorithm::Unsupported => continue,
        };
        let actual: &[u8] = match declaration.algorithm {
            DigestAlgorithm::Sha1 => &actual_sha1,
            DigestAlgorithm::Md5 => &actual_md5,
            DigestAlgorithm::Unsupported => unreachable!(),
        };
        // Validate even manually constructed public declarations, rather than
        // relying on the manifest parser having normalized them beforehand.
        digest_matches(&declaration.hex, actual)?;
        if let Some(previous) = slot {
            if !declaration.hex.eq_ignore_ascii_case(previous) {
                return Err(EnvelopeError::ConflictingDigest);
            }
        } else {
            *slot = Some(declaration.hex.as_slice());
        }
        supported = true;
    }
    if !supported {
        return Err(EnvelopeError::UnsupportedIntegrity);
    }
    for (hex, actual) in [(sha1, actual_sha1.as_slice()), (md5, actual_md5.as_slice())] {
        if let Some(hex) = hex {
            if !digest_matches(hex, actual)? {
                return Err(EnvelopeError::DigestMismatch);
            }
        }
    }
    Ok(())
}

/// Decode a selected volume only after validating its original stored bytes.
pub fn decode_verified_volume(
    stored: &[u8],
    envelope: Envelope,
    declarations: &[DeclaredDigest],
    limits: EnvelopeLimits,
) -> Result<Vec<u8>, EnvelopeError> {
    verify_stored_digests(stored, declarations, limits.stored_bytes)?;
    decode_metadata(stored, envelope, limits)
}

//= docs/parsing-requirements.md#parse-env-001
//= type=implementation
//# The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.
/// Decode manifest/signature metadata, which has no manifest volume digest.
/// For data volumes, use `decode_verified_volume` instead.
pub fn decode_metadata(
    stored: &[u8],
    envelope: Envelope,
    limits: EnvelopeLimits,
) -> Result<Vec<u8>, EnvelopeError> {
    if stored.len() > limits.stored_bytes {
        return Err(EnvelopeError::StoredLimit);
    }
    let mut output = Vec::new();
    match envelope {
        Envelope::Plain => {
            if stored.len() > limits.decoded_bytes {
                return Err(EnvelopeError::DecodedLimit);
            }
            output
                .try_reserve_exact(stored.len())
                .map_err(|_| EnvelopeError::Allocation)?;
            output.extend_from_slice(stored);
        }
        Envelope::Gzip => {
            let mut remaining = stored;
            let mut members = 0usize;
            if remaining.is_empty() {
                return Err(EnvelopeError::InvalidGzip);
            }
            while !remaining.is_empty() {
                if members >= limits.gzip_members {
                    return Err(EnvelopeError::MemberLimit);
                }
                members += 1;
                let previous = remaining.len();
                let mut decoder = flate2::bufread::GzDecoder::new(remaining);
                let mut buffer = [0u8; 32 * 1024];
                loop {
                    // Read through true member EOF, including CRC32/ISIZE,
                    // even when output lands exactly at its configured bound.
                    let available = limits.decoded_bytes - output.len();
                    let request = available.saturating_add(1).min(buffer.len());
                    let count = decoder
                        .read(&mut buffer[..request])
                        .map_err(|_| EnvelopeError::InvalidGzip)?;
                    if count == 0 {
                        break;
                    }
                    if count > available {
                        return Err(EnvelopeError::DecodedLimit);
                    }
                    output
                        .try_reserve_exact(count)
                        .map_err(|_| EnvelopeError::Allocation)?;
                    output.extend_from_slice(&buffer[..count]);
                }
                remaining = decoder.into_inner();
                if remaining.len() >= previous {
                    return Err(EnvelopeError::InvalidGzip);
                }
            }
        }
        Envelope::OpenPgp | Envelope::Pkcs7 => return Err(EnvelopeError::UnsupportedEnvelope),
    }
    Ok(output)
}

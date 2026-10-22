//! Native SEIPDv1 recovery adapter. Crypto, key/packet primitives and compression
//! are trusted dependencies; this bounded controller is native-tested, with
//! formal refinement and full compatibility acceptance still pending.
#[cfg(test)]
pub(crate) mod adversarial_tests;
#[cfg(test)]
mod armor_crc_tests;
#[cfg(test)]
mod fixture_tests;
mod framing;
use framing::{frames, native, Packet as Frame};
use pgp::{
    composed::{
        decrypt_session_key_with_password, Deserializable, PlainSessionKey, SignedPublicKey,
        SignedSecretKey,
    },
    packet::{Packet, Signature, SignatureType, StreamDecryptor},
    types::{DecryptionKey, EskType, KeyDetails, Password, Seipdv1ReadMode},
};
use std::io::{Cursor, Read};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Stored,
    Authenticated,
    Decoded,
    Expanded,
    KeyBytes,
    Certificates,
    Subkeys,
    SessionPackets,
    Signatures,
    Packets,
    Depth,
    S2kWork,
    Derivations,
    PacketBody,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    Limit(Limit),
    Allocation,
    Malformed,
    Unsupported,
    MissingPassword,
    MissingSecretKey,
    WrongCredentials,
    AuthenticationFailed,
    InvalidKey,
    InvalidBinding,
    Unsigned,
    SignerNotAllowed,
    InvalidSignature,
    InconsistentSessionKeys,
}
impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpenPGP recovery failed: {self:?}")
    }
}
impl std::error::Error for CryptoError {}
#[derive(Clone, Copy, Debug)]
pub struct CryptoLimits {
    pub stored_bytes: usize,
    pub authenticated_bytes: usize,
    pub decoded_bytes: usize,
    /// Total bytes emitted by compression layers, including intermediate layers.
    pub expanded_bytes: usize,
    pub key_bytes: usize,
    pub certificates: usize,
    pub subkeys_per_certificate: usize,
    pub session_packets: usize,
    pub signatures: usize,
    pub packets: usize,
    pub depth: usize,
    pub s2k_bytes: usize,
    pub derivations: usize,
    pub packet_body_bytes: usize,
}
impl Default for CryptoLimits {
    fn default() -> Self {
        Self {
            stored_bytes: 256 * 1024 * 1024,
            authenticated_bytes: 256 * 1024 * 1024,
            decoded_bytes: 8 * 1024 * 1024 * 1024,
            expanded_bytes: 8 * 1024 * 1024 * 1024,
            key_bytes: 16 * 1024 * 1024,
            certificates: 64,
            subkeys_per_certificate: 64,
            session_packets: 64,
            signatures: 64,
            packets: 4096,
            depth: 8,
            s2k_bytes: 65_011_712,
            derivations: 64,
            packet_body_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}
pub struct SecretExport<'a> {
    pub bytes: &'a [u8],
    pub password: Option<&'a Password>,
}
#[derive(Default)]
pub struct Credentials<'a> {
    pub message_passwords: &'a [Password],
    pub secret_exports: &'a [SecretExport<'a>],
    pub signer_exports: &'a [&'a [u8]],
    pub permitted_signers: &'a [[u8; 20]],
    /// Require authorization even when the permitted-fingerprint list is empty.
    pub require_signature: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateMetadata {
    pub primary_fingerprint: [u8; 20],
    pub created_seconds: u32,
    pub signatures: Vec<CertificateSignatureMetadata>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateSignatureMetadata {
    pub created_seconds: Option<u32>,
    pub key_expiry_seconds: Option<u32>,
    pub signature_expiry_seconds: Option<u32>,
    pub revocation: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedSigner {
    pub primary_fingerprint: [u8; 20],
    pub component_fingerprint: [u8; 20],
}
/// Authorization is offline fingerprint recovery policy, not current validity,
/// user-ID certification or Web-of-Trust verification. No filename is trusted.
pub(crate) struct DecodedObject {
    pub(crate) bytes: Vec<u8>,
    pub(crate) authorized_signer: Option<AuthorizedSigner>,
    pub(crate) certificates: Vec<CertificateMetadata>,
}
struct Work {
    limits: CryptoLimits,
    packets: usize,
    signatures: usize,
    derivations: usize,
    expanded: usize,
}
impl Work {
    fn new(limits: CryptoLimits) -> Self {
        Self {
            limits,
            packets: 0,
            signatures: 0,
            derivations: 0,
            expanded: 0,
        }
    }
    fn packet(&mut self) -> Result<(), CryptoError> {
        if self.packets >= self.limits.packets {
            return Err(CryptoError::Limit(Limit::Packets));
        }
        self.packets += 1;
        Ok(())
    }
    fn signature(&mut self) -> Result<(), CryptoError> {
        if self.signatures >= self.limits.signatures {
            return Err(CryptoError::Limit(Limit::Signatures));
        }
        self.signatures += 1;
        Ok(())
    }
    fn derivation(&mut self) -> Result<(), CryptoError> {
        if self.derivations >= self.limits.derivations {
            return Err(CryptoError::Limit(Limit::Derivations));
        }
        self.derivations += 1;
        Ok(())
    }
}
fn read_bounded(
    mut reader: impl Read,
    maximum: usize,
    limit: Limit,
) -> Result<Vec<u8>, CryptoError> {
    let mut output = Vec::new();
    let mut buffer = [0; 32768];
    loop {
        let available = maximum - output.len();
        let request = available.saturating_add(1).min(buffer.len());
        let count = reader
            .read(&mut buffer[..request])
            .map_err(|_| CryptoError::AuthenticationFailed)?;
        if count > request {
            return Err(CryptoError::Malformed);
        }
        if count == 0 {
            return Ok(output);
        }
        if count > available {
            return Err(CryptoError::Limit(limit));
        }
        output
            .try_reserve_exact(count)
            .map_err(|_| CryptoError::Allocation)?;
        output.extend_from_slice(&buffer[..count]);
    }
}
/// Buffer a source with a real one-byte overflow/EOF probe before key/crypto work.
pub(crate) fn read_ciphertext(
    reader: impl Read,
    limits: CryptoLimits,
) -> Result<Vec<u8>, CryptoError> {
    read_bounded(reader, limits.stored_bytes, Limit::Stored)
}
fn copy(bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes.len())
        .map_err(|_| CryptoError::Allocation)?;
    output.extend_from_slice(bytes);
    Ok(output)
}
fn push<T>(values: &mut Vec<T>, value: T) -> Result<(), CryptoError> {
    values
        .try_reserve_exact(1)
        .map_err(|_| CryptoError::Allocation)?;
    values.push(value);
    Ok(())
}
fn aes(value: u8) -> Result<(), CryptoError> {
    if matches!(value, 7..=9) {
        Ok(())
    } else {
        Err(CryptoError::Unsupported)
    }
}
fn signature_policy(body: &[u8], document: bool) -> Result<(), CryptoError> {
    if body.len() < 6 {
        return Err(CryptoError::Malformed);
    }
    if body[0] != 4 || document && body[1] != 0 {
        return Err(CryptoError::Unsupported);
    }
    match body[2] {
        1 | 3 if matches!(body[3], 8..=11) => Ok(()),
        22 if matches!(body[3], 8..=10) => Ok(()),
        _ => Err(CryptoError::Unsupported),
    }
}
fn s2k(body: &[u8], maximum: usize) -> Result<usize, CryptoError> {
    let typ = *body.first().ok_or(CryptoError::Malformed)?;
    let hash = *body.get(1).ok_or(CryptoError::Malformed)?;
    if !matches!(hash, 2 | 8..=11) {
        return Err(CryptoError::Unsupported);
    }
    let length = match typ {
        0 => 2,
        1 => 10,
        3 => 11,
        _ => return Err(CryptoError::Unsupported),
    };
    if body.len() < length {
        return Err(CryptoError::Malformed);
    }
    if typ == 3 {
        let encoded = body[10] as usize;
        let count = (16 + (encoded & 15)) << ((encoded >> 4) + 6);
        if count > maximum {
            return Err(CryptoError::Limit(Limit::S2kWork));
        }
    }
    Ok(length)
}
fn mpi_end(body: &[u8], offset: usize) -> Result<usize, CryptoError> {
    let header = body
        .get(offset..offset.checked_add(2).ok_or(CryptoError::Malformed)?)
        .ok_or(CryptoError::Malformed)?;
    let bits = u16::from_be_bytes([header[0], header[1]]) as usize;
    let end = offset
        .checked_add(2 + bits.div_ceil(8))
        .ok_or(CryptoError::Malformed)?;
    if end > body.len() {
        return Err(CryptoError::Malformed);
    }
    Ok(end)
}
fn key_policy(packet: &Frame, work: &Work) -> Result<(), CryptoError> {
    let b = &packet.body;
    if b.len() < 8 {
        return Err(CryptoError::Malformed);
    }
    if b[0] != 4 {
        return Err(CryptoError::Unsupported);
    }
    let public_end = match b[5] {
        1..=3 => {
            let bits = u16::from_be_bytes([b[6], b[7]]);
            if !(2048..=8192).contains(&bits) {
                return Err(CryptoError::Unsupported);
            }
            let modulus_end = mpi_end(b, 6)?;
            // The primitive MPI parser strips leading zeros. The encoded bit
            // count must describe the actual modulus, not padded weak material.
            let actual_bits = (modulus_end - 8) * 8 - b[8].leading_zeros() as usize;
            if b[8] == 0 || actual_bits != bits as usize {
                return Err(CryptoError::Malformed);
            }
            mpi_end(b, modulus_end)?
        }
        18 | 22 => {
            let oid_end = 7usize
                .checked_add(b[6] as usize)
                .ok_or(CryptoError::Malformed)?;
            let oid = b.get(7..oid_end).ok_or(CryptoError::Malformed)?;
            let expected: &[u8] = if b[5] == 18 {
                &[0x2b, 0x06, 0x01, 0x04, 0x01, 0x97, 0x55, 0x01, 0x05, 0x01]
            } else {
                &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xda, 0x47, 0x0f, 0x01]
            };
            if oid != expected {
                return Err(CryptoError::Unsupported);
            }
            let end = mpi_end(b, oid_end)?;
            if b[5] == 18 {
                let kdf = b.get(end..end + 4).ok_or(CryptoError::Malformed)?;
                if kdf[0] != 3 || kdf[1] != 1 || !matches!(kdf[2], 8..=10) {
                    return Err(CryptoError::Unsupported);
                }
                aes(kdf[3])?;
                end + 4
            } else {
                end
            }
        }
        _ => return Err(CryptoError::Unsupported),
    };
    if matches!(packet.tag, 5 | 7) {
        let secret = b.get(public_end..).ok_or(CryptoError::Malformed)?;
        match secret.first() {
            Some(0) => {}
            Some(254) => {
                aes(*secret.get(1).ok_or(CryptoError::Malformed)?)?;
                s2k(
                    secret.get(2..).ok_or(CryptoError::Malformed)?,
                    work.limits.s2k_bytes,
                )?;
            }
            Some(_) => return Err(CryptoError::Unsupported),
            None => return Err(CryptoError::Malformed),
        }
    } else if public_end != b.len() {
        return Err(CryptoError::Malformed);
    }
    Ok(())
}
fn key_bytes(export: &[u8], private: bool, maximum: usize) -> Result<Vec<u8>, CryptoError> {
    if export.len() > maximum {
        return Err(CryptoError::Limit(Limit::KeyBytes));
    }
    if !export.starts_with(b"-----BEGIN") {
        return copy(export);
    }
    let expected = if private {
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----".as_slice()
    } else {
        b"-----BEGIN PGP PUBLIC KEY BLOCK-----".as_slice()
    };
    if !export.starts_with(expected) {
        return Err(CryptoError::InvalidKey);
    }
    let options = pgp::armor::DearmorOptions::new().set_limit(maximum);
    let mut armor = pgp::armor::Dearmor::with_options(Cursor::new(export), options);
    let decoded =
        read_bounded(&mut armor, maximum, Limit::KeyBytes).map_err(|error| match error {
            CryptoError::AuthenticationFailed => CryptoError::InvalidKey,
            other => other,
        })?;
    let (typ, _, _, mut tail) = armor.into_parts();
    let expected = if private {
        pgp::armor::BlockType::PrivateKey
    } else {
        pgp::armor::BlockType::PublicKey
    };
    if typ != Some(expected) {
        return Err(CryptoError::InvalidKey);
    }
    let mut buffer = [0; 1024];
    loop {
        let n = tail
            .read(&mut buffer)
            .map_err(|_| CryptoError::InvalidKey)?;
        if n == 0 {
            break;
        }
        if !buffer[..n].iter().all(u8::is_ascii_whitespace) {
            return Err(CryptoError::InvalidKey);
        }
    }
    Ok(decoded)
}
fn key_packets(
    export: &[u8],
    private: bool,
    work: &mut Work,
) -> Result<(Vec<pgp::errors::Result<Packet>>, usize), CryptoError> {
    let binary = key_bytes(export, private, work.limits.key_bytes)?;
    let framed = frames(&binary, work)?;
    let primary_tag = if private { 5 } else { 6 };
    if framed.first().map(|p| p.tag) != Some(primary_tag) {
        return Err(CryptoError::InvalidKey);
    }
    let mut primary = 0usize;
    let mut subkeys = 0usize;
    let mut result = Vec::new();
    for packet in &framed {
        match packet.tag {
            5..=7 | 14 => {
                if matches!(packet.tag, 5 | 6) {
                    primary += 1;
                    if primary > 1 {
                        return Err(CryptoError::InvalidKey);
                    }
                } else {
                    subkeys += 1;
                    if subkeys > work.limits.subkeys_per_certificate {
                        return Err(CryptoError::Limit(Limit::Subkeys));
                    }
                }
                if !private && matches!(packet.tag, 5 | 7) {
                    return Err(CryptoError::InvalidKey);
                }
                key_policy(packet, work)?;
            }
            2 => {
                signature_preflight(&packet.body, false, 0, work)?;
            }
            13 | 17 => {}
            _ => return Err(CryptoError::Unsupported),
        }
        result
            .try_reserve_exact(1)
            .map_err(|_| CryptoError::Allocation)?;
        result.push(Ok(native(packet)?));
    }
    Ok((result, framed.len()))
}
// Upstream composition drops unsigned components and unexpected signatures.
// Reject that normalization: every preflighted packet must remain represented.
fn represented_packets(
    details: &pgp::composed::SignedKeyDetails,
    subkeys: impl Iterator<Item = usize>,
) -> Result<usize, CryptoError> {
    let components = details
        .users
        .iter()
        .map(|user| user.signatures.len())
        .chain(
            details
                .user_attributes
                .iter()
                .map(|attribute| attribute.signatures.len()),
        )
        .chain(subkeys);
    let mut count = 1usize
        .checked_add(details.direct_signatures.len())
        .and_then(|n| n.checked_add(details.revocation_signatures.len()))
        .ok_or(CryptoError::InvalidKey)?;
    for signatures in components {
        count = count
            .checked_add(1)
            .and_then(|n| n.checked_add(signatures))
            .ok_or(CryptoError::InvalidKey)?;
    }
    Ok(count)
}
fn fingerprint(key: &impl KeyDetails) -> Result<[u8; 20], CryptoError> {
    key.fingerprint()
        .as_bytes()
        .try_into()
        .map_err(|_| CryptoError::Unsupported)
}
fn self_certified(details: &pgp::composed::SignedKeyDetails) -> bool {
    !details.users.is_empty()
        || !details.user_attributes.is_empty()
        || !details.direct_signatures.is_empty()
}
fn metadata(key: &SignedPublicKey) -> Result<CertificateMetadata, CryptoError> {
    let mut signatures = Vec::new();
    let all = key
        .details
        .direct_signatures
        .iter()
        .chain(key.details.revocation_signatures.iter())
        .chain(
            key.details
                .users
                .iter()
                .flat_map(|user| user.signatures.iter()),
        )
        .chain(
            key.details
                .user_attributes
                .iter()
                .flat_map(|attribute| attribute.signatures.iter()),
        )
        .chain(
            key.public_subkeys
                .iter()
                .flat_map(|subkey| subkey.signatures.iter()),
        );
    for signature in all {
        signatures
            .try_reserve_exact(1)
            .map_err(|_| CryptoError::Allocation)?;
        signatures.push(CertificateSignatureMetadata {
            created_seconds: signature.created().map(|v| v.as_secs()),
            key_expiry_seconds: signature.key_expiration_time().map(|v| v.as_secs()),
            signature_expiry_seconds: signature.signature_expiration_time().map(|v| v.as_secs()),
            revocation: matches!(
                signature.typ(),
                Some(
                    SignatureType::KeyRevocation
                        | SignatureType::SubkeyRevocation
                        | SignatureType::CertRevocation
                )
            ),
        });
    }
    Ok(CertificateMetadata {
        primary_fingerprint: fingerprint(&key.primary_key)?,
        created_seconds: key.primary_key.created_at().as_secs(),
        signatures,
    })
}
fn validate_secret_params(
    params: &pgp::types::SecretParams,
    work: &mut Work,
) -> Result<(), CryptoError> {
    // Count every candidate unwrap, including unprotected public-key material;
    // expensive RSA/ECDH work must not evade the password-attempt budget.
    work.derivation()?;
    if let pgp::types::SecretParams::Encrypted(encrypted) = params {
        if !matches!(
            encrypted.string_to_key_params(),
            pgp::types::S2kParams::Cfb { .. }
        ) {
            return Err(CryptoError::Unsupported);
        }
    }
    Ok(())
}
fn session_key(
    candidate: PlainSessionKey,
    selected: &mut Option<PlainSessionKey>,
) -> Result<(), CryptoError> {
    let algorithm = candidate.sym_algorithm().ok_or(CryptoError::Unsupported)?;
    aes(u8::from(algorithm))?;
    if let Some(previous) = selected {
        if previous != &candidate {
            return Err(CryptoError::InconsistentSessionKeys);
        }
    } else {
        *selected = Some(candidate);
    }
    Ok(())
}

fn signature_preflight(
    body: &[u8],
    document: bool,
    depth: usize,
    work: &mut Work,
) -> Result<(), CryptoError> {
    signature_policy(body, document)?;
    if depth > work.limits.depth {
        return Err(CryptoError::Limit(Limit::Depth));
    }
    let hashed = u16::from_be_bytes([body[4], body[5]]) as usize;
    let end = 6usize.checked_add(hashed).ok_or(CryptoError::Malformed)?;
    subpackets(body.get(6..end).ok_or(CryptoError::Malformed)?, depth, work)?;
    let pair = body.get(end..end + 2).ok_or(CryptoError::Malformed)?;
    let unhashed = u16::from_be_bytes([pair[0], pair[1]]) as usize;
    subpackets(
        body.get(end + 2..end + 2 + unhashed)
            .ok_or(CryptoError::Malformed)?,
        depth,
        work,
    )?;
    // Require exact signature-material consumption even for embedded signatures.
    // Establish this independently of rPGP's nested parser, which does not
    // expose an inner consumption position (its enclosing length guard also
    // rejects the tested appended-byte case).
    let mut material_end = mpi_end(body, end + 2 + unhashed + 2)?;
    if body[2] == 22 {
        material_end = mpi_end(body, material_end)?;
    }
    if material_end != body.len() {
        return Err(CryptoError::Malformed);
    }
    Ok(())
}
fn subpackets(mut bytes: &[u8], depth: usize, work: &mut Work) -> Result<(), CryptoError> {
    while !bytes.is_empty() {
        work.packet()?;
        let first = bytes[0];
        bytes = &bytes[1..];
        let (count, advance) = match first {
            0..=191 => (first as usize, 0),
            192..=254 => {
                let next = *bytes.first().ok_or(CryptoError::Malformed)?;
                (((first as usize - 192) << 8) + next as usize + 192, 1)
            }
            255 => {
                let raw = bytes.get(..4).ok_or(CryptoError::Malformed)?;
                (
                    u32::from_be_bytes(raw.try_into().map_err(|_| CryptoError::Malformed)?)
                        as usize,
                    4,
                )
            }
        };
        bytes = bytes.get(advance..).ok_or(CryptoError::Malformed)?;
        let (body, tail) = bytes
            .split_at_checked(count)
            .ok_or(CryptoError::Malformed)?;
        bytes = tail;
        let typ = *body.first().ok_or(CryptoError::Malformed)?;
        let (kind, critical) = pgp::packet::SubpacketType::from_u8(typ);
        if critical && matches!(kind, pgp::packet::SubpacketType::Other(_)) {
            return Err(CryptoError::Unsupported);
        }
        if typ & 127 == 32 {
            work.signature()?;
            signature_preflight(
                &body[1..],
                false,
                depth
                    .checked_add(1)
                    .ok_or(CryptoError::Limit(Limit::Depth))?,
                work,
            )?;
        }
    }
    Ok(())
}
fn decompress(frame: &Frame, work: &mut Work) -> Result<Vec<u8>, CryptoError> {
    let (&kind, bytes) = frame.body.split_first().ok_or(CryptoError::Malformed)?;
    let available = work
        .limits
        .expanded_bytes
        .checked_sub(work.expanded)
        .ok_or(CryptoError::Limit(Limit::Expanded))?;
    let output = match kind {
        0 => {
            if bytes.len() > available {
                return Err(CryptoError::Limit(Limit::Expanded));
            }
            copy(bytes)?
        }
        1 => inflate(bytes, false, available)?,
        2 => inflate(bytes, true, available)?,
        3 => {
            let mut reader = bzip2::bufread::BzDecoder::new(bytes);
            let decoded = read_bounded(&mut reader, available, Limit::Expanded)?;
            if !reader.into_inner().is_empty() {
                return Err(CryptoError::Malformed);
            }
            decoded
        }
        _ => return Err(CryptoError::Unsupported),
    };
    work.expanded = work
        .expanded
        .checked_add(output.len())
        .ok_or(CryptoError::Limit(Limit::Expanded))?;
    Ok(output)
}
fn inflate(bytes: &[u8], zlib: bool, maximum: usize) -> Result<Vec<u8>, CryptoError> {
    let mut decoder = flate2::Decompress::new(zlib);
    let mut output = Vec::new();
    let mut buffer = [0; 32768];
    loop {
        let consumed = usize::try_from(decoder.total_in()).map_err(|_| CryptoError::Malformed)?;
        let before_out = decoder.total_out();
        let available = maximum - output.len();
        let request = available.saturating_add(1).min(buffer.len());
        let status = decoder
            .decompress(
                bytes.get(consumed..).ok_or(CryptoError::Malformed)?,
                &mut buffer[..request],
                flate2::FlushDecompress::None,
            )
            .map_err(|_| CryptoError::Malformed)?;
        let count = usize::try_from(
            decoder
                .total_out()
                .checked_sub(before_out)
                .ok_or(CryptoError::Malformed)?,
        )
        .map_err(|_| CryptoError::Malformed)?;
        if count > request {
            return Err(CryptoError::Malformed);
        }
        if count > available {
            return Err(CryptoError::Limit(Limit::Expanded));
        }
        output
            .try_reserve_exact(count)
            .map_err(|_| CryptoError::Allocation)?;
        output.extend_from_slice(&buffer[..count]);
        if status == flate2::Status::StreamEnd {
            if decoder.total_in() != bytes.len() as u64 {
                return Err(CryptoError::Malformed);
            }
            return Ok(output);
        }
        if decoder.total_in() == consumed as u64 && count == 0 {
            return Err(CryptoError::Malformed);
        }
    }
}
struct Content {
    bytes: Vec<u8>,
    signatures: Vec<Signature>,
}
fn content(bytes: &[u8], depth: usize, work: &mut Work) -> Result<Content, CryptoError> {
    if depth > work.limits.depth {
        return Err(CryptoError::Limit(Limit::Depth));
    }
    let packets = frames(bytes, work)?;
    let mut begin = 0usize;
    let mut prefix = Vec::new();
    let mut operations: Vec<&Frame> = Vec::new();
    while let Some(packet) = packets.get(begin) {
        match packet.tag {
            2 if operations.is_empty() => {
                signature_preflight(
                    &packet.body,
                    true,
                    depth
                        .checked_add(1)
                        .ok_or(CryptoError::Limit(Limit::Depth))?,
                    work,
                )?;
                let Packet::Signature(signature) = native(packet)? else {
                    return Err(CryptoError::Malformed);
                };
                push(&mut prefix, signature)?;
            }
            4 if prefix.is_empty() => {
                if operations.len() >= work.limits.signatures {
                    return Err(CryptoError::Limit(Limit::Signatures));
                }
                let b = &packet.body;
                if b.len() != 13 {
                    return Err(CryptoError::Malformed);
                }
                if b[0] != 3
                    || b[1] != 0
                    || !matches!(b[2], 8..=11)
                    || !matches!(b[3], 1 | 3 | 22)
                    || b[3] == 22 && b[2] == 11
                {
                    return Err(CryptoError::Unsupported);
                }
                push(&mut operations, packet)?;
            }
            _ => break,
        }
        begin += 1;
    }
    for (index, operation) in operations.iter().enumerate() {
        if operation.body[12] != u8::from(index + 1 == operations.len()) {
            return Err(CryptoError::Malformed);
        }
    }
    let depth = if !prefix.is_empty() || !operations.is_empty() {
        if depth >= work.limits.depth {
            return Err(CryptoError::Limit(Limit::Depth));
        }
        depth + 1
    } else {
        depth
    };
    let inner = packets.get(begin).ok_or(CryptoError::Malformed)?;
    let mut result = match inner.tag {
        8 => {
            if depth >= work.limits.depth {
                return Err(CryptoError::Limit(Limit::Depth));
            }
            let decoded = decompress(inner, work)?;
            content(&decoded, depth + 1, work)?
        }
        11 => {
            let b = &inner.body;
            if b.len() < 6 {
                return Err(CryptoError::Malformed);
            }
            if !matches!(b[0], b'b' | b't' | b'u') {
                return Err(CryptoError::Unsupported);
            }
            let offset = 6 + b[1] as usize;
            let literal = b.get(offset..).ok_or(CryptoError::Malformed)?;
            if literal.len() > work.limits.decoded_bytes {
                return Err(CryptoError::Limit(Limit::Decoded));
            }
            Content {
                bytes: copy(literal)?,
                signatures: Vec::new(),
            }
        }
        1 | 3 | 9 | 18 | 20 => return Err(CryptoError::Unsupported),
        _ => return Err(CryptoError::Malformed),
    };
    let trailing = &packets[begin + 1..];
    if trailing.len() != operations.len() {
        return Err(CryptoError::Malformed);
    }
    for (operation, signature) in operations.iter().rev().zip(trailing) {
        if signature.tag != 2 {
            return Err(CryptoError::Malformed);
        }
        signature_preflight(&signature.body, true, depth, work)?;
        if operation.body[1] != signature.body[1]
            || operation.body[2] != signature.body[3]
            || operation.body[3] != signature.body[2]
        {
            return Err(CryptoError::Malformed);
        }
        let Packet::Signature(signature) = native(signature)? else {
            return Err(CryptoError::Malformed);
        };
        push(&mut prefix, signature)?;
    }
    result
        .signatures
        .try_reserve_exact(prefix.len())
        .map_err(|_| CryptoError::Allocation)?;
    result.signatures.extend(prefix);
    Ok(result)
}

/// Decrypt an original stored object. No plaintext is returned until complete
/// MDC, framing, decompression and configured signer checks have passed.
//= docs/parsing-requirements.md#parse-env-010
//# The adapter MUST enforce its configured ciphertext bound before materializing an encrypted object and authenticate the complete SEIPDv1 payload before releasing plaintext to message or duplicity parsers.
pub(crate) fn decode(
    stored: &[u8],
    credentials: Credentials<'_>,
    limits: CryptoLimits,
) -> Result<DecodedObject, CryptoError> {
    if stored.len() > limits.stored_bytes {
        return Err(CryptoError::Limit(Limit::Stored));
    }
    if credentials
        .secret_exports
        .len()
        .checked_add(credentials.signer_exports.len())
        .ok_or(CryptoError::Limit(Limit::Certificates))?
        > limits.certificates
    {
        return Err(CryptoError::Limit(Limit::Certificates));
    }
    if credentials.permitted_signers.len() > limits.certificates {
        return Err(CryptoError::Limit(Limit::Certificates));
    }
    if credentials.message_passwords.len() > limits.derivations {
        return Err(CryptoError::Limit(Limit::Derivations));
    }
    let total_keys = credentials
        .secret_exports
        .iter()
        .map(|key| key.bytes.len())
        .chain(credentials.signer_exports.iter().map(|key| key.len()))
        .try_fold(0usize, |sum, n| sum.checked_add(n))
        .ok_or(CryptoError::Limit(Limit::KeyBytes))?;
    if total_keys > limits.key_bytes {
        return Err(CryptoError::Limit(Limit::KeyBytes));
    }
    let mut work = Work::new(limits);
    let outer = frames(stored, &mut work)?;
    let (edata, wrappers) = outer.split_last().ok_or(CryptoError::Malformed)?;
    if edata.tag != 18 || edata.body.first() != Some(&1) {
        return Err(CryptoError::Unsupported);
    }
    if wrappers.is_empty() {
        return Err(CryptoError::Malformed);
    }
    if wrappers.len() > limits.session_packets {
        return Err(CryptoError::Limit(Limit::SessionPackets));
    }
    if edata.body.len() - 1 > limits.authenticated_bytes {
        return Err(CryptoError::Limit(Limit::Authenticated));
    }
    for wrapper in wrappers {
        match wrapper.tag {
            1 => {
                if wrapper.body.len() < 10 {
                    return Err(CryptoError::Malformed);
                }
                if wrapper.body[0] != 3 || !matches!(wrapper.body[9], 1 | 2 | 18) {
                    return Err(CryptoError::Unsupported);
                }
            }
            3 => {
                if wrapper.body.len() < 4 {
                    return Err(CryptoError::Malformed);
                }
                if wrapper.body[0] != 4 {
                    return Err(CryptoError::Unsupported);
                }
                aes(wrapper.body[1])?;
                s2k(&wrapper.body[2..], limits.s2k_bytes)?;
            }
            _ => return Err(CryptoError::Unsupported),
        }
    }
    let mut secret_keys = Vec::new();
    for export in credentials.secret_exports {
        let (packets, expected_packets) = key_packets(export.bytes, true, &mut work)?;
        let mut parsed = SignedSecretKey::from_packets(packets.into_iter().peekable());
        let key = parsed
            .next()
            .ok_or(CryptoError::InvalidKey)?
            .map_err(|_| CryptoError::InvalidKey)?;
        if parsed.next().is_some()
            || !self_certified(&key.details)
            || represented_packets(
                &key.details,
                key.public_subkeys
                    .iter()
                    .map(|subkey| subkey.signatures.len())
                    .chain(
                        key.secret_subkeys
                            .iter()
                            .map(|subkey| subkey.signatures.len()),
                    ),
            )? != expected_packets
        {
            return Err(CryptoError::InvalidKey);
        }
        key.verify_bindings()
            .map_err(|_| CryptoError::InvalidBinding)?;
        push(&mut secret_keys, (key, export.password))?;
    }
    let mut public_keys = Vec::new();
    let mut certificates = Vec::new();
    for export in credentials.signer_exports {
        let (packets, expected_packets) = key_packets(export, false, &mut work)?;
        let mut parsed = SignedPublicKey::from_packets(packets.into_iter().peekable());
        let key = parsed
            .next()
            .ok_or(CryptoError::InvalidKey)?
            .map_err(|_| CryptoError::InvalidKey)?;
        if parsed.next().is_some()
            || !self_certified(&key.details)
            || represented_packets(
                &key.details,
                key.public_subkeys
                    .iter()
                    .map(|subkey| subkey.signatures.len()),
            )? != expected_packets
        {
            return Err(CryptoError::InvalidKey);
        }
        key.verify_bindings()
            .map_err(|_| CryptoError::InvalidBinding)?;
        push(&mut certificates, metadata(&key)?)?;
        push(&mut public_keys, key)?;
    }
    let mut selected = None;
    let mut matching = false;
    let mut missing_password = false;
    for wrapper in wrappers {
        match native(wrapper)? {
            Packet::SymKeyEncryptedSessionKey(packet) => {
                if credentials.message_passwords.is_empty() {
                    missing_password = true;
                }
                for password in credentials.message_passwords {
                    matching = true;
                    work.derivation()?;
                    if let Ok(candidate) = decrypt_session_key_with_password(&packet, password) {
                        session_key(candidate, &mut selected)?;
                    }
                }
            }
            Packet::PublicKeyEncryptedSessionKey(packet) => {
                for (key, password) in &secret_keys {
                    if packet.match_identity(&key.primary_key)
                        && matches!(
                            key.primary_key.algorithm(),
                            pgp::crypto::public_key::PublicKeyAlgorithm::RSA
                                | pgp::crypto::public_key::PublicKeyAlgorithm::RSAEncrypt
                                | pgp::crypto::public_key::PublicKeyAlgorithm::ECDH
                        )
                    {
                        matching = true;
                        if password.is_none()
                            && matches!(
                                key.primary_key.secret_params(),
                                pgp::types::SecretParams::Encrypted(_)
                            )
                        {
                            missing_password = true;
                            continue;
                        }
                        validate_secret_params(key.primary_key.secret_params(), &mut work)?;
                        let empty = Password::empty();
                        let password = password.unwrap_or(&empty);
                        if let Ok(Ok(candidate)) = key.primary_key.decrypt(
                            password,
                            packet.values().map_err(|_| CryptoError::Malformed)?,
                            EskType::V3_4,
                        ) {
                            session_key(candidate, &mut selected)?;
                        }
                    }
                    for subkey in &key.secret_subkeys {
                        if !packet.match_identity(&subkey.key)
                            || !matches!(
                                subkey.key.algorithm(),
                                pgp::crypto::public_key::PublicKeyAlgorithm::RSA
                                    | pgp::crypto::public_key::PublicKeyAlgorithm::RSAEncrypt
                                    | pgp::crypto::public_key::PublicKeyAlgorithm::ECDH
                            )
                        {
                            continue;
                        }
                        matching = true;
                        if password.is_none()
                            && matches!(
                                subkey.key.secret_params(),
                                pgp::types::SecretParams::Encrypted(_)
                            )
                        {
                            missing_password = true;
                            continue;
                        }
                        validate_secret_params(subkey.key.secret_params(), &mut work)?;
                        let empty = Password::empty();
                        let password = password.unwrap_or(&empty);
                        if let Ok(Ok(candidate)) = subkey.key.decrypt(
                            password,
                            packet.values().map_err(|_| CryptoError::Malformed)?,
                            EskType::V3_4,
                        ) {
                            session_key(candidate, &mut selected)?;
                        }
                    }
                }
            }
            _ => return Err(CryptoError::Unsupported),
        }
    }
    let selected = selected.ok_or(if missing_password {
        CryptoError::MissingPassword
    } else if matching {
        CryptoError::WrongCredentials
    } else {
        CryptoError::MissingSecretKey
    })?;
    let PlainSessionKey::V3_4 { sym_alg, ref key } = selected else {
        return Err(CryptoError::Unsupported);
    };
    if key.len() != sym_alg.key_size() {
        return Err(CryptoError::Malformed);
    }
    let mut decryptor = StreamDecryptor::v1(
        sym_alg,
        Seipdv1ReadMode::CheckFirst {
            max_message_size: limits.authenticated_bytes,
        },
        key.as_ref(),
        &edata.body[1..],
    )
    .map_err(|_| CryptoError::AuthenticationFailed)?;
    let authenticated = read_bounded(
        &mut decryptor,
        limits.authenticated_bytes,
        Limit::Authenticated,
    )?;
    let result = content(&authenticated, 0, &mut work)?;
    let mut authorized_signer = None;
    if credentials.require_signature || !credentials.permitted_signers.is_empty() {
        if result.signatures.is_empty() {
            return Err(CryptoError::Unsigned);
        }
        for signature in &result.signatures {
            let mut verified = None;
            let mut permitted_certificate = false;
            for certificate in &public_keys {
                let primary = fingerprint(&certificate.primary_key)?;
                if !credentials.permitted_signers.contains(&primary) {
                    continue;
                }
                permitted_certificate = true;
                if signature
                    .verify(&certificate.primary_key, &result.bytes[..])
                    .is_ok()
                {
                    verified = Some(AuthorizedSigner {
                        primary_fingerprint: primary,
                        component_fingerprint: primary,
                    });
                    break;
                }
                for subkey in &certificate.public_subkeys {
                    if !subkey.signatures.iter().any(|binding| {
                        binding.key_flags().sign() && binding.embedded_signature().is_some()
                    }) {
                        continue;
                    }
                    if signature.verify(&subkey.key, &result.bytes[..]).is_ok() {
                        verified = Some(AuthorizedSigner {
                            primary_fingerprint: primary,
                            component_fingerprint: fingerprint(&subkey.key)?,
                        });
                        break;
                    }
                }
                if verified.is_some() {
                    break;
                }
            }
            let verified = verified.ok_or(if permitted_certificate {
                CryptoError::InvalidSignature
            } else {
                CryptoError::SignerNotAllowed
            })?;
            if authorized_signer.is_none() {
                authorized_signer = Some(verified);
            }
        }
    }
    Ok(DecodedObject {
        bytes: result.bytes,
        authorized_signer,
        certificates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_class_limits_precede_claimed_packet_body_reads() {
        for (tag, limits, expected) in [
            (
                3,
                CryptoLimits {
                    session_packets: 0,
                    ..CryptoLimits::default()
                },
                Limit::SessionPackets,
            ),
            (
                2,
                CryptoLimits {
                    signatures: 0,
                    ..CryptoLimits::default()
                },
                Limit::Signatures,
            ),
            (
                4,
                CryptoLimits {
                    signatures: 0,
                    ..CryptoLimits::default()
                },
                Limit::Signatures,
            ),
            (
                14,
                CryptoLimits {
                    subkeys_per_certificate: 0,
                    ..CryptoLimits::default()
                },
                Limit::Subkeys,
            ),
        ] {
            // An incomplete length header would otherwise produce Malformed.
            // The configured class limit must win before any body work.
            assert!(
                matches!(frames(&[0xc0 | tag, 255], &mut Work::new(limits)), Err(CryptoError::Limit(found)) if found == expected)
            );
        }
    }
    #[test]
    fn rsa_one_mpi_and_legacy_ed_two_mpi_tails_are_exact_recursively() {
        let rsa = [4, 0, 1, 8, 0, 0, 0, 0, 0, 0, 0, 1, 1];
        let ed = [4, 0, 22, 10, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1];
        for valid in [&rsa[..], &ed[..]] {
            signature_preflight(valid, true, 0, &mut Work::new(CryptoLimits::default())).unwrap();
            for extra in [&[0x55][..], &[0, 1, 1][..]] {
                let malformed = [valid, extra].concat();
                assert!(matches!(
                    signature_preflight(
                        &malformed,
                        true,
                        0,
                        &mut Work::new(CryptoLimits::default())
                    ),
                    Err(CryptoError::Malformed)
                ));
                let mut containing = vec![
                    4,
                    0,
                    1,
                    8,
                    0,
                    0,
                    0,
                    (malformed.len() + 2) as u8,
                    (malformed.len() + 1) as u8,
                    32,
                ];
                containing.extend(&malformed);
                containing.extend([0, 0, 0, 1, 1]);
                assert!(matches!(
                    signature_preflight(
                        &containing,
                        true,
                        0,
                        &mut Work::new(CryptoLimits::default())
                    ),
                    Err(CryptoError::Malformed)
                ));
            }
            assert!(matches!(
                signature_preflight(
                    &valid[..valid.len() - 1],
                    true,
                    0,
                    &mut Work::new(CryptoLimits::default())
                ),
                Err(CryptoError::Malformed)
            ));
        }
    }
    #[test]
    fn rsa_modulus_bound_cannot_be_spoofed_by_declared_bits_or_padding() {
        let mut body = vec![4, 0, 0, 0, 0, 1, 8, 0]; // RSA, declared 2048 bits.
        body.extend([0; 256]);
        body[8] = 0x80;
        body.extend([0, 17, 1, 0, 1]); // Public exponent 65537.
        let mut frame = Frame { tag: 6, body };
        key_policy(&frame, &Work::new(CryptoLimits::default())).unwrap();
        frame.body[8] = 0x40; // Actual 2047 bits, declared 2048.
        assert!(matches!(
            key_policy(&frame, &Work::new(CryptoLimits::default())),
            Err(CryptoError::Malformed)
        ));
        frame.body[8] = 0;
        frame.body[200] = 0x80; // Heavily padded weak modulus.
        assert!(matches!(
            key_policy(&frame, &Work::new(CryptoLimits::default())),
            Err(CryptoError::Malformed)
        ));
    }
    fn binary(packets: &[Frame]) -> Vec<u8> {
        let mut output = Vec::new();
        for frame in packets {
            output.extend([0xc0 | frame.tag, 255]);
            output.extend((frame.body.len() as u32).to_be_bytes());
            output.extend(&frame.body);
        }
        output
    }
    #[test]
    fn corrupt_binding_secret_integrity_and_missing_key_password_fail() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7");
        let public = std::fs::read(root.join("test-keys/public.asc")).unwrap();
        let secret = std::fs::read(root.join("test-keys/secret.asc")).unwrap();
        let stored =
            std::fs::read(root.join("public-key/duplicity-full.20231114T221320Z.manifest.gpg"))
                .unwrap();
        let signed =
            std::fs::read(root.join("signed/duplicity-full.20231114T221320Z.manifest.gpg"))
                .unwrap();
        let password = Password::from("duplicity-reader-public-test-fixture");
        let missing = [SecretExport {
            bytes: &secret,
            password: None,
        }];
        assert!(matches!(
            decode(
                &stored,
                Credentials {
                    secret_exports: &missing,
                    ..Credentials::default()
                },
                CryptoLimits::default()
            ),
            Err(CryptoError::MissingPassword)
        ));
        let exports = [SecretExport {
            bytes: &secret,
            password: Some(&password),
        }];
        let mut work = Work::new(CryptoLimits::default());
        let mut packets = frames(
            &key_bytes(&public, false, work.limits.key_bytes).unwrap(),
            &mut work,
        )
        .unwrap();
        *packets
            .iter_mut()
            .rev()
            .find(|frame| frame.tag == 2)
            .unwrap()
            .body
            .last_mut()
            .unwrap() ^= 1;
        let corrupted = binary(&packets);
        let signers = [corrupted.as_slice()];
        assert!(matches!(
            decode(
                &signed,
                Credentials {
                    secret_exports: &exports,
                    signer_exports: &signers,
                    require_signature: true,
                    ..Credentials::default()
                },
                CryptoLimits::default()
            ),
            Err(CryptoError::InvalidBinding)
        ));
        let mut work = Work::new(CryptoLimits::default());
        let mut unsigned = frames(
            &key_bytes(&public, false, work.limits.key_bytes).unwrap(),
            &mut work,
        )
        .unwrap();
        assert_eq!(unsigned.pop().unwrap().tag, 2); // Strip recipient binding.
        let unsigned = binary(&unsigned);
        let signers = [unsigned.as_slice()];
        assert!(matches!(
            decode(
                &signed,
                Credentials {
                    secret_exports: &exports,
                    signer_exports: &signers,
                    require_signature: true,
                    ..Credentials::default()
                },
                CryptoLimits::default()
            ),
            Err(CryptoError::InvalidKey)
        ));
        let mut work = Work::new(CryptoLimits::default());
        let mut unsigned_user = frames(
            &key_bytes(&public, false, work.limits.key_bytes).unwrap(),
            &mut work,
        )
        .unwrap();
        unsigned_user.push(Frame {
            tag: 13,
            body: b"unsigned identity".to_vec(),
        });
        let unsigned_user = binary(&unsigned_user);
        let signers = [unsigned_user.as_slice()];
        assert!(matches!(
            decode(
                &signed,
                Credentials {
                    secret_exports: &exports,
                    signer_exports: &signers,
                    require_signature: true,
                    ..Credentials::default()
                },
                CryptoLimits::default()
            ),
            Err(CryptoError::InvalidKey)
        ));
        let mut work = Work::new(CryptoLimits::default());
        let mut packets = frames(
            &key_bytes(&secret, true, work.limits.key_bytes).unwrap(),
            &mut work,
        )
        .unwrap();
        *packets
            .iter_mut()
            .find(|frame| frame.tag == 7)
            .unwrap()
            .body
            .last_mut()
            .unwrap() ^= 1;
        let corrupted = binary(&packets);
        let corrupted_exports = [SecretExport {
            bytes: &corrupted,
            password: Some(&password),
        }];
        assert!(matches!(
            decode(
                &stored,
                Credentials {
                    secret_exports: &corrupted_exports,
                    ..Credentials::default()
                },
                CryptoLimits::default()
            ),
            Err(CryptoError::WrongCredentials)
        ));
        for (limits, expected) in [
            (
                CryptoLimits {
                    key_bytes: secret.len() - 1,
                    ..CryptoLimits::default()
                },
                Limit::KeyBytes,
            ),
            (
                CryptoLimits {
                    certificates: 0,
                    ..CryptoLimits::default()
                },
                Limit::Certificates,
            ),
            (
                CryptoLimits {
                    subkeys_per_certificate: 0,
                    ..CryptoLimits::default()
                },
                Limit::Subkeys,
            ),
            (
                CryptoLimits {
                    signatures: 0,
                    ..CryptoLimits::default()
                },
                Limit::Signatures,
            ),
            (
                CryptoLimits {
                    s2k_bytes: 58_720_255,
                    ..CryptoLimits::default()
                },
                Limit::S2kWork,
            ),
        ] {
            assert!(
                matches!(decode(&stored, Credentials { secret_exports: &exports, ..Credentials::default() }, limits), Err(CryptoError::Limit(found)) if found == expected)
            );
        }
    }
    #[test]
    fn official_key_exports_pass_strict_preflight() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/test-keys");
        for (name, private) in [("secret.asc", true), ("public.asc", false)] {
            let bytes = std::fs::read(path.join(name)).unwrap();
            key_bytes(&bytes, private, CryptoLimits::default().key_bytes).expect("armor framing");
            let mut work = Work::new(CryptoLimits::default());
            let (packets, _) = key_packets(&bytes, private, &mut work).expect("packet preflight");
            if private {
                let mut parsed = SignedSecretKey::from_packets(packets.into_iter().peekable());
                parsed
                    .next()
                    .expect("secret composition present")
                    .expect("secret composition valid");
                assert!(parsed.next().is_none());
            } else {
                let mut parsed = SignedPublicKey::from_packets(packets.into_iter().peekable());
                parsed
                    .next()
                    .expect("public composition present")
                    .expect("public composition valid");
                assert!(parsed.next().is_none());
            }
        }
    }
}

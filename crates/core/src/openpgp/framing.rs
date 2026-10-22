//! Strict framing controller. Native-tested; formal refinement is pending.
use super::{CryptoError, Limit, Work};

pub(super) struct Packet {
    pub tag: u8,
    pub body: Vec<u8>,
}
fn take<'a>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [u8], CryptoError> {
    let (head, tail) = bytes
        .split_at_checked(count)
        .ok_or(CryptoError::Malformed)?;
    *bytes = tail;
    Ok(head)
}
fn length(bytes: &mut &[u8]) -> Result<(usize, bool), CryptoError> {
    let first = take(bytes, 1)?[0];
    match first {
        0..=191 => Ok((first as usize, false)),
        192..=223 => Ok((
            ((first as usize - 192) << 8) + take(bytes, 1)?[0] as usize + 192,
            false,
        )),
        224..=254 => Ok((
            1usize
                .checked_shl((first & 31) as u32)
                .ok_or(CryptoError::Malformed)?,
            true,
        )),
        255 => Ok((
            u32::from_be_bytes(
                take(bytes, 4)?
                    .try_into()
                    .map_err(|_| CryptoError::Malformed)?,
            ) as usize,
            false,
        )),
    }
}
pub(super) fn frames(mut bytes: &[u8], work: &mut Work) -> Result<Vec<Packet>, CryptoError> {
    let mut packets = Vec::new();
    let mut sessions = 0usize;
    let mut operations = 0usize;
    let mut subkeys = 0usize;
    let signature_room = work
        .limits
        .signatures
        .checked_sub(work.signatures)
        .ok_or(CryptoError::Limit(Limit::Signatures))?;
    while !bytes.is_empty() {
        work.packet()?;
        let header = take(&mut bytes, 1)?[0];
        if header & 128 == 0 {
            return Err(CryptoError::Malformed);
        }
        let tag = if header & 64 != 0 {
            header & 63
        } else {
            (header >> 2) & 15
        };
        // Charge semantic resource classes before reading/copying body bytes.
        // OPS packets announce later signatures; they use the initial available
        // budget here, while actual signature packets charge the global count.
        if matches!(tag, 1 | 3) {
            if sessions >= work.limits.session_packets {
                return Err(CryptoError::Limit(Limit::SessionPackets));
            }
            sessions += 1;
        }
        if tag == 2 {
            work.signature()?;
        }
        if tag == 4 {
            if operations >= signature_room {
                return Err(CryptoError::Limit(Limit::Signatures));
            }
            operations += 1;
        }
        if matches!(tag, 7 | 14) {
            if subkeys >= work.limits.subkeys_per_certificate {
                return Err(CryptoError::Limit(Limit::Subkeys));
            }
            subkeys += 1;
        }
        let (mut count, mut partial) = if header & 64 != 0 {
            length(&mut bytes)?
        } else {
            let count = match header & 3 {
                0 => take(&mut bytes, 1)?[0] as usize,
                1 => u16::from_be_bytes(
                    take(&mut bytes, 2)?
                        .try_into()
                        .map_err(|_| CryptoError::Malformed)?,
                ) as usize,
                2 => u32::from_be_bytes(
                    take(&mut bytes, 4)?
                        .try_into()
                        .map_err(|_| CryptoError::Malformed)?,
                ) as usize,
                3 => bytes.len(), // Authenticated enclosing EOF bounds old indeterminate bodies.
                _ => return Err(CryptoError::Malformed),
            };
            (count, false)
        };
        // RFC9580 4.2.1.4 restricts partial bodies to data packets and
        // requires the first segment to contain at least 512 octets.
        if partial && (!matches!(tag, 8 | 11 | 18) || count < 512) {
            return Err(CryptoError::Malformed);
        }
        let mut body = Vec::new();
        loop {
            let segment = take(&mut bytes, count)?;
            let total = body
                .len()
                .checked_add(count)
                .ok_or(CryptoError::Limit(Limit::PacketBody))?;
            if total > work.limits.packet_body_bytes {
                return Err(CryptoError::Limit(Limit::PacketBody));
            }
            body.try_reserve_exact(count)
                .map_err(|_| CryptoError::Allocation)?;
            body.extend_from_slice(segment);
            if !partial {
                break;
            }
            (count, partial) = length(&mut bytes)?; // Required definite terminal segment.
        }
        packets
            .try_reserve_exact(1)
            .map_err(|_| CryptoError::Allocation)?;
        packets.push(Packet { tag, body });
    }
    Ok(packets)
}
pub(super) fn native(packet: &Packet) -> Result<pgp::packet::Packet, CryptoError> {
    let tag = pgp::types::Tag::from(packet.tag);
    let length =
        u32::try_from(packet.body.len()).map_err(|_| CryptoError::Limit(Limit::PacketBody))?;
    let header = pgp::packet::PacketHeader::new_fixed(tag, length);
    let mut input = std::io::Cursor::new(&packet.body);
    let result =
        pgp::packet::Packet::from_reader(header, &mut input).map_err(|_| CryptoError::Malformed)?;
    if input.position() != packet.body.len() as u64 {
        return Err(CryptoError::Malformed);
    }
    Ok(result)
}

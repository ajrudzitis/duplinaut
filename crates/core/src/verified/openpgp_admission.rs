//! Verified state machine used by the private OpenPGP volume admission path.
//!
//! Verus proves normalized cursor/finality and stage-token state transitions.
//! A source-reviewed native gate calls the stored-byte digest checker before
//! it can invoke the private Verus digest transition. Verus does not prove the
//! gate's call order, the SHA1/MD5 implementations, or crypto/tar behavior.
#![allow(clippy::len_zero)]
use crate::envelope::{verify_stored_digests, EnvelopeError};
use crate::manifest::DeclaredDigest;
use vstd::prelude::*;

verus! {
pub struct Position {
    pub ordinal: usize,
    pub final_volume: bool,
}

/// Non-copy stage token created by the wrapper after its stored-byte digest check.
pub struct DigestPermit<'a> {
    ordinal: usize,
    final_volume: bool,
    /// The exact stored object whose declarations were checked.
    stored: &'a [u8],
}
impl DigestPermit<'_> {
    pub closed spec fn ordinal_value(&self) -> usize { self.ordinal }
    pub closed spec fn final_value(&self) -> bool { self.final_volume }
}

/// Non-copy stage token returned by the wrapper after native decode succeeds.
pub struct AuthenticatedPermit {
    ordinal: usize,
    final_volume: bool,
}
impl AuthenticatedPermit {
    pub closed spec fn ordinal_value(&self) -> usize { self.ordinal }
    pub closed spec fn final_value(&self) -> bool { self.final_volume }
}

/// Non-copy stage token returned by the wrapper after assembler checks succeed.
pub struct TarValidatedPermit {
    ordinal: usize,
    final_volume: bool,
}
impl TarValidatedPermit {
    pub closed spec fn ordinal_value(&self) -> usize { self.ordinal }
    pub closed spec fn final_value(&self) -> bool { self.final_volume }
}

/// Cursor over the already-normalized manifest volume slice.
/// Phases: 0 ready, 1 digest checked, 2 authenticated, 3 tar checked, 4 rejected.
pub struct OpenPgpAdmission {
    next: usize,
    count: usize,
    phase: u8,
}

impl OpenPgpAdmission {
    pub closed spec fn next_value(&self) -> usize { self.next }
    pub closed spec fn count_value(&self) -> usize { self.count }
    pub closed spec fn phase_value(&self) -> u8 { self.phase }

    pub fn new(count: usize) -> (result: Self)
        requires count > 0,
        ensures result.next_value() == 0, result.count_value() == count,
            result.phase_value() == 0,
    {
        OpenPgpAdmission { next: 0, count, phase: 0 }
    }

    pub fn position(&self) -> (result: Option<Position>)
        ensures match result {
            Some(p) => self.phase_value() == 0 && self.next_value() < self.count_value()
                && p.ordinal == self.next_value()
                && p.final_volume == (self.next_value() + 1 == self.count_value()),
            None => self.phase_value() != 0 || self.next_value() >= self.count_value(),
        },
    {
        if self.phase != 0 || self.next >= self.count { return None; }
        Some(Position {
            ordinal: self.next,
            final_volume: self.next + 1 == self.count,
        })
    }

    fn digest_verified<'a>(
        &mut self,
        position: Position,
        stored: &'a [u8],
    ) -> (result: DigestPermit<'a>)
        requires old(self).phase_value() == 0,
            old(self).next_value() < old(self).count_value(),
            position.ordinal == old(self).next_value(),
            position.final_volume == (old(self).next_value() + 1 == old(self).count_value()),
        ensures final(self).phase_value() == 1,
            final(self).next_value() == old(self).next_value(),
            final(self).count_value() == old(self).count_value(),
            result.ordinal_value() == position.ordinal,
            result.final_value() == position.final_volume,
    {
        self.phase = 1;
        DigestPermit { ordinal: position.ordinal, final_volume: position.final_volume, stored }
    }

    pub fn authenticated(&mut self, permit: DigestPermit<'_>) -> (result: AuthenticatedPermit)
        requires old(self).phase_value() == 1,
            old(self).next_value() < old(self).count_value(),
            permit.ordinal_value() == old(self).next_value(),
            permit.final_value() == (old(self).next_value() + 1 == old(self).count_value()),
        ensures final(self).phase_value() == 2,
            final(self).next_value() == old(self).next_value(),
            final(self).count_value() == old(self).count_value(),
            result.ordinal_value() == permit.ordinal_value(),
            result.final_value() == permit.final_value(),
    {
        self.phase = 2;
        AuthenticatedPermit { ordinal: permit.ordinal, final_volume: permit.final_volume }
    }

    pub fn tar_validated(&mut self, permit: AuthenticatedPermit) -> (result: TarValidatedPermit)
        requires old(self).phase_value() == 2,
            old(self).next_value() < old(self).count_value(),
            permit.ordinal_value() == old(self).next_value(),
            permit.final_value() == (old(self).next_value() + 1 == old(self).count_value()),
        ensures final(self).phase_value() == 3,
            final(self).next_value() == old(self).next_value(),
            final(self).count_value() == old(self).count_value(),
            result.ordinal_value() == permit.ordinal_value(),
            result.final_value() == permit.final_value(),
    {
        self.phase = 3;
        TarValidatedPermit { ordinal: permit.ordinal, final_volume: permit.final_volume }
    }

    /// Advance only with a tar-stage capability for this normalized ordinal.
    pub fn commit_openpgp(&mut self, permit: TarValidatedPermit) -> (result: bool)
        requires old(self).phase_value() == 3,
            old(self).next_value() < old(self).count_value(),
            permit.ordinal_value() == old(self).next_value(),
            permit.final_value() == (old(self).next_value() + 1 == old(self).count_value()),
        ensures final(self).phase_value() == 0,
            final(self).next_value() == old(self).next_value() + 1,
            final(self).count_value() == old(self).count_value(),
    {
        if self.phase != 3 || self.next >= self.count
            || permit.ordinal != self.next
            || permit.final_volume != (self.next + 1 == self.count) {
            return false;
        }
        self.next += 1;
        self.phase = 0;
        true
    }

    /// Plain/gzip has its own reviewed decoder path, but shares normalized order.
    pub fn commit_plain(&mut self)
        requires old(self).phase_value() == 0,
            old(self).next_value() < old(self).count_value(),
        ensures final(self).phase_value() == 0,
            final(self).next_value() == old(self).next_value() + 1,
            final(self).count_value() == old(self).count_value(),
    {
        self.next += 1;
    }

    /// An observed error or cancellation latches rejection without cursor movement.
    pub fn reject(&mut self)
        requires old(self).phase_value() != 4,
        ensures final(self).phase_value() == 4,
            final(self).next_value() == old(self).next_value(),
            final(self).count_value() == old(self).count_value(),
    {
        self.phase = 4;
    }

    pub fn is_complete(&self) -> (result: bool)
        ensures result == (self.next_value() == self.count_value()
            && self.phase_value() == 0),
    {
        self.next == self.count && self.phase == 0
    }

    pub fn ordinal(&self) -> (result: usize)
        ensures result == self.next_value(),
    {
        self.next
    }
}
}

/// Native, source-reviewed bridge to the formal cursor transition.
///
/// This is the only Rust-visible producer of `DigestPermit`. The call to
/// `verify_stored_digests` and the following transition are deliberately in
/// one private module boundary. The digest implementation and the factual
/// call-order relation are trusted native code, not Verus-proved properties.
impl OpenPgpAdmission {
    pub(crate) fn verify_stored_digest<'a>(
        &mut self,
        position: Position,
        stored: &'a [u8],
        declarations: &[DeclaredDigest],
        stored_limit: usize,
    ) -> Result<DigestPermit<'a>, EnvelopeError> {
        verify_stored_digests(stored, declarations, stored_limit)?;
        Ok(self.digest_verified(position, stored))
    }
}

/// Consume the digest capability and decode only its borrowed, checked bytes.
/// Keeping this at the token's private-field boundary prevents pairing a check
/// of object A with a decode of object B inside another reader call.
pub(crate) fn decode_after_digest(
    admission: &mut OpenPgpAdmission,
    credentials: crate::openpgp::Credentials<'_>,
    limits: crate::openpgp::CryptoLimits,
    digest: DigestPermit<'_>,
) -> Result<(crate::openpgp::DecodedObject, AuthenticatedPermit), crate::openpgp::CryptoError> {
    let decoded = crate::openpgp::decode(digest.stored, credentials, limits)?;
    let authenticated = admission.authenticated(digest);
    Ok((decoded, authenticated))
}

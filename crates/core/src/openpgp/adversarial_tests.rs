use crate::openpgp::{Credentials, CryptoError, CryptoLimits, Limit, decode};
use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    packet::{SymEncryptedProtectedData, SymKeyEncryptedSessionKey},
    ser::Serialize,
    types::{Password, StringToKey},
};
use rand::{SeedableRng, rngs::StdRng};
use std::io::Write;

fn packet(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xc0 | tag, 255];
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn literal(bytes: &[u8]) -> Vec<u8> {
    let mut body = vec![b'b', 0, 0, 0, 0, 0];
    body.extend_from_slice(bytes);
    packet(11, &body)
}
fn seal(bytes: &[u8]) -> Vec<u8> {
    let key = [0x42; 32];
    let wrapper = SymKeyEncryptedSessionKey::encrypt_v4(
        &Password::from("test only"),
        &key.to_vec().into(),
        StringToKey::Salted {
            hash_alg: HashAlgorithm::Sha256,
            salt: [0x31; 8],
        },
        SymmetricKeyAlgorithm::AES256,
    )
    .unwrap();
    let encrypted = SymEncryptedProtectedData::encrypt_seipdv1(
        StdRng::seed_from_u64(41),
        SymmetricKeyAlgorithm::AES256,
        &key,
        bytes,
    )
    .unwrap();
    [
        packet(3, &wrapper.to_bytes().unwrap()),
        packet(18, &encrypted.to_bytes().unwrap()),
    ]
    .concat()
}
fn recover(
    bytes: &[u8],
    limits: CryptoLimits,
) -> Result<crate::openpgp::DecodedObject, CryptoError> {
    let password = [Password::from("test only")];
    decode(
        bytes,
        Credentials {
            message_passwords: &password,
            ..Credentials::default()
        },
        limits,
    )
}
#[test]
fn authenticated_payload_is_exactly_one_literal_and_has_no_trailing_bytes() {
    let good = literal(b"complete");
    assert_eq!(
        recover(&seal(&good), CryptoLimits::default())
            .unwrap()
            .bytes,
        b"complete"
    );
    for bad in [
        Vec::new(),
        [good.as_slice(), literal(b"second").as_slice()].concat(),
        [good.as_slice(), &[0]].concat(),
        packet(18, &[1, 0]),
        packet(9, &[0]),
        packet(20, &[1]),
    ] {
        assert!(recover(&seal(&bad), CryptoLimits::default()).is_err());
    }
    let bounds = CryptoLimits {
        decoded_bytes: 7,
        ..CryptoLimits::default()
    };
    assert!(matches!(
        recover(&seal(&good), bounds),
        Err(CryptoError::Limit(Limit::Decoded))
    ));
}
#[test]
fn truncation_mutations_unsupported_envelopes_and_partial_framing_are_rejected() {
    let stored = seal(&literal(b"mutation target"));
    for length in 0..stored.len() {
        assert!(
            recover(&stored[..length], CryptoLimits::default()).is_err(),
            "ciphertext truncation {length}"
        );
    }
    for index in [0, 1, 8, stored.len() / 2, stored.len() - 1] {
        let mut damaged = stored.clone();
        damaged[index] ^= 1;
        assert!(
            recover(&damaged, CryptoLimits::default()).is_err(),
            "ciphertext mutation {index}"
        );
    }
    for ciphertext in [
        packet(9, &[0]),
        packet(18, &[2]),
        packet(20, &[1]),
        [stored.as_slice(), &[0]].concat(),
    ] {
        assert!(recover(&ciphertext, CryptoLimits::default()).is_err());
    }
    let payload = vec![b'x'; 700];
    let definite = literal(&payload);
    let body = &definite[6..];
    let mut partial = vec![0xcb, 233]; // Legal first partial length = 512.
    partial.extend_from_slice(&body[..512]);
    let rest = body.len() - 512;
    partial.extend([((rest - 192) >> 8) as u8 + 192, ((rest - 192) & 255) as u8]);
    partial.extend_from_slice(&body[512..]);
    assert_eq!(
        recover(&seal(&partial), CryptoLimits::default())
            .unwrap()
            .bytes,
        payload
    );
    let mut unterminated = vec![0xcb, 233];
    unterminated.extend([0; 512]);
    assert!(recover(&seal(&unterminated), CryptoLimits::default()).is_err());
    for bad in [
        vec![0xcb, 224, 0, 0],
        vec![0xc2, 233],
        vec![0],
        vec![0xcb, 255, 0xff, 0xff, 0xff, 0xff],
    ] {
        assert!(recover(&seal(&bad), CryptoLimits::default()).is_err());
    }
}
#[test]
fn every_compression_profile_is_bounded_and_consumes_exactly_its_stream() {
    let input = literal(b"compressible literal content");
    for algorithm in 0..=3 {
        let compressed = match algorithm {
            0 => input.clone(),
            1 => {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&input).unwrap();
                encoder.finish().unwrap()
            }
            2 => {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&input).unwrap();
                encoder.finish().unwrap()
            }
            3 => {
                let mut encoder =
                    bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
                encoder.write_all(&input).unwrap();
                encoder.finish().unwrap()
            }
            _ => unreachable!(),
        };
        let mut body = vec![algorithm];
        body.extend(&compressed);
        assert_eq!(
            recover(&seal(&packet(8, &body)), CryptoLimits::default())
                .unwrap()
                .bytes,
            b"compressible literal content"
        );
        let bound = CryptoLimits {
            expanded_bytes: input.len() - 1,
            ..CryptoLimits::default()
        };
        assert!(matches!(
            recover(&seal(&packet(8, &body)), bound),
            Err(CryptoError::Limit(Limit::Expanded))
        ));
        if algorithm != 0 {
            let truncated = &body[..body.len() - 1];
            assert!(
                recover(&seal(&packet(8, truncated)), CryptoLimits::default()).is_err(),
                "truncated compression {algorithm}"
            );
            body.push(0);
            assert!(recover(&seal(&packet(8, &body)), CryptoLimits::default()).is_err());
        }
    }
}
#[test]
fn depth_packet_session_derivation_and_authentication_budgets_fail_closed() {
    let mut nested = literal(b"x");
    for _ in 0..8 {
        nested = packet(8, &[&[0][..], &nested].concat());
    }
    assert!(recover(&seal(&nested), CryptoLimits::default()).is_ok());
    nested = packet(8, &[&[0][..], &nested].concat());
    assert!(matches!(
        recover(&seal(&nested), CryptoLimits::default()),
        Err(CryptoError::Limit(Limit::Depth))
    ));
    let stored = seal(&literal(b"bound"));
    for (limits, kind) in [
        (
            CryptoLimits {
                packets: 1,
                ..CryptoLimits::default()
            },
            Limit::Packets,
        ),
        (
            CryptoLimits {
                session_packets: 0,
                ..CryptoLimits::default()
            },
            Limit::SessionPackets,
        ),
        (
            CryptoLimits {
                derivations: 0,
                ..CryptoLimits::default()
            },
            Limit::Derivations,
        ),
        (
            CryptoLimits {
                authenticated_bytes: 1,
                ..CryptoLimits::default()
            },
            Limit::Authenticated,
        ),
        (
            CryptoLimits {
                packet_body_bytes: 1,
                ..CryptoLimits::default()
            },
            Limit::PacketBody,
        ),
    ] {
        assert!(
            matches!(recover(&stored, limits), Err(CryptoError::Limit(found)) if found == kind)
        );
    }
    let password = [Password::from("test only")];
    assert!(matches!(
        decode(
            &stored,
            Credentials {
                message_passwords: &password,
                require_signature: true,
                ..Credentials::default()
            },
            CryptoLimits::default()
        ),
        Err(CryptoError::Unsigned)
    ));
}
#[test]
fn full_fingerprint_authorization_rejects_unsigned_wrong_signer_and_bad_signatures() {
    use pgp::composed::{Deserializable, MessageBuilder, SignedSecretKey};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/test-keys");
    let secret = std::fs::read(root.join("secret.asc")).unwrap();
    let public = std::fs::read(root.join("public.asc")).unwrap();
    let (key, _) = SignedSecretKey::from_string(std::str::from_utf8(&secret).unwrap()).unwrap();
    let mut builder = MessageBuilder::from_bytes("", b"signed literal".to_vec());
    builder.sign(
        &key.primary_key,
        Password::from("duplicity-reader-public-test-fixture"),
        HashAlgorithm::Sha512,
    );
    let signed = builder.to_vec(StdRng::seed_from_u64(73)).unwrap();
    let fingerprint: [u8; 20] = [
        0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23, 0x17,
        0x3b, 0xad, 0x8a, 0x76, 0x56,
    ];
    let passwords = [Password::from("test only")];
    let signers = [public.as_slice()];
    let allowed = [fingerprint];
    let recover = |payload: &[u8], fingerprints: &[[u8; 20]], limits: CryptoLimits| {
        decode(
            &seal(payload),
            Credentials {
                message_passwords: &passwords,
                signer_exports: &signers,
                permitted_signers: fingerprints,
                require_signature: true,
                ..Credentials::default()
            },
            limits,
        )
    };
    let output = recover(&signed, &allowed, CryptoLimits::default()).unwrap();
    assert_eq!(output.bytes, b"signed literal");
    assert_eq!(
        output.authorized_signer.unwrap().primary_fingerprint,
        fingerprint
    );
    assert!(matches!(
        recover(&literal(b"unsigned"), &allowed, CryptoLimits::default()),
        Err(CryptoError::Unsigned)
    ));
    assert!(matches!(
        recover(&signed, &[], CryptoLimits::default()),
        Err(CryptoError::SignerNotAllowed)
    ));
    let mut wrong = fingerprint;
    wrong[0] ^= 1; // Same short key ID, different whole fingerprint.
    assert!(matches!(
        recover(&signed, &[wrong], CryptoLimits::default()),
        Err(CryptoError::SignerNotAllowed)
    ));
    let mut corrupted = signed.clone();
    *corrupted.last_mut().unwrap() ^= 1;
    assert!(matches!(
        recover(&corrupted, &allowed, CryptoLimits::default()),
        Err(CryptoError::InvalidSignature)
    ));
    assert!(matches!(
        recover(
            &signed,
            &allowed,
            CryptoLimits {
                depth: 0,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Depth))
    ));
}
#[test]
fn authenticated_valid_embedded_backsignature_with_trailing_bytes_is_rejected() {
    use pgp::{
        composed::{
            EncryptionCaps, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
        },
        crypto::public_key::PublicKeyAlgorithm,
        packet::{SignatureConfig, SignatureType},
        types::KeyDetails,
    };
    let subkey = SubkeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_sign(true)
        .can_encrypt(EncryptionCaps::None)
        .build()
        .unwrap();
    let key = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_certify(true)
        .can_sign(true)
        .can_encrypt(EncryptionCaps::None)
        .primary_user_id("public regression identity".into())
        .subkeys(vec![subkey])
        .build()
        .unwrap()
        .generate(StdRng::seed_from_u64(119))
        .unwrap();
    let public = SignedPublicKey::from(key.clone());
    public.verify_bindings().unwrap();
    let subkey = &public.public_subkeys[0];
    let back = subkey.signatures[0].embedded_signature().unwrap();
    back.verify_primary_key_binding(&subkey.key, &public.primary_key)
        .unwrap();
    let signature = SignatureConfig::v4(
        SignatureType::Binary,
        PublicKeyAlgorithm::EdDSALegacy,
        HashAlgorithm::Sha512,
    )
    .sign(
        &key.primary_key,
        &Password::empty(),
        &b"authenticated literal"[..],
    )
    .unwrap();
    let signature_body = signature.to_bytes().unwrap();
    let hashed = u16::from_be_bytes([signature_body[4], signature_body[5]]) as usize;
    let pos = 6 + hashed;
    let unhashed = u16::from_be_bytes([signature_body[pos], signature_body[pos + 1]]) as usize;
    let end = pos + 2 + unhashed;
    let back_bytes = back.to_bytes().unwrap();
    let public_export = public.to_bytes().unwrap();
    let signers = [public_export.as_slice()];
    let passwords = [Password::from("test only")];
    let fingerprint: [u8; 20] = public.fingerprint().as_bytes().try_into().unwrap();
    let allowed = [fingerprint];
    for trailing in [false, true] {
        let mut embedded = back_bytes.clone();
        if trailing {
            embedded.push(0x55);
        }
        let mut subpacket = vec![255];
        subpacket.extend(((embedded.len() + 1) as u32).to_be_bytes());
        subpacket.push(32);
        subpacket.extend(embedded);
        let mut outer = signature_body[..pos].to_vec();
        outer.extend(((unhashed + subpacket.len()) as u16).to_be_bytes());
        outer.extend(&signature_body[pos + 2..end]);
        outer.extend(subpacket);
        outer.extend(&signature_body[end..]);
        let mut native_reader = &outer[..];
        let header =
            pgp::packet::PacketHeader::new_fixed(pgp::types::Tag::Signature, outer.len() as u32);
        let native = pgp::packet::Signature::try_from_reader(header, &mut native_reader);
        if trailing {
            // rPGP's generic subpacket reserialization-length guard also rejects
            // this example; project framing must establish exact consumption
            // independently rather than relying on primitive normalization.
            assert!(native.is_err());
        } else {
            let normalized = native.unwrap();
            normalized
                .verify(&public.primary_key, &b"authenticated literal"[..])
                .unwrap();
            normalized
                .embedded_signature()
                .unwrap()
                .verify_primary_key_binding(&subkey.key, &public.primary_key)
                .unwrap();
        }
        let payload = [packet(2, &outer), literal(b"authenticated literal")].concat();
        let decoded = decode(
            &seal(&payload),
            Credentials {
                message_passwords: &passwords,
                signer_exports: &signers,
                permitted_signers: &allowed,
                require_signature: true,
                ..Credentials::default()
            },
            CryptoLimits::default(),
        );
        if trailing {
            assert!(matches!(decoded, Err(CryptoError::Malformed)));
        } else {
            assert_eq!(decoded.unwrap().bytes, b"authenticated literal");
        }
    }
    let mut outer_extra = signature_body;
    outer_extra.push(0x55);
    assert!(matches!(
        decode(
            &seal(&[packet(2, &outer_extra), literal(b"authenticated literal")].concat()),
            Credentials {
                message_passwords: &passwords,
                signer_exports: &signers,
                permitted_signers: &allowed,
                require_signature: true,
                ..Credentials::default()
            },
            CryptoLimits::default()
        ),
        Err(CryptoError::Malformed)
    ));
}

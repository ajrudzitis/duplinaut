use super::{
    content, decode, framing::frames, key_bytes, key_packets, key_policy, s2k, signature_preflight,
    Credentials, CryptoError, CryptoLimits, Frame, Limit, SecretExport, Work,
};
use pgp::{
    composed::{
        EncryptionCaps, KeyType, MessageBuilder, SecretKeyParamsBuilder, SubkeyParamsBuilder,
    },
    crypto::{ecc_curve::ECCCurve, hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    packet::{SymEncryptedProtectedData, SymKeyEncryptedSessionKey},
    ser::Serialize,
    types::{Password, StringToKey},
};
use rand::{rngs::StdRng, SeedableRng};

fn packet(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut result = vec![0xc0 | tag, 255];
    result.extend_from_slice(&(body.len() as u32).to_be_bytes());
    result.extend_from_slice(body);
    result
}

fn literal(bytes: &[u8]) -> Vec<u8> {
    let mut body = vec![b'b', 0, 0, 0, 0, 0];
    body.extend_from_slice(bytes);
    packet(11, &body)
}

fn signature_with_embedded(inner: Option<&[u8]>) -> Vec<u8> {
    let mut body = vec![4, 0, 1, 8, 0, 0]; // v4 RSA/SHA256 with no hashed subpackets.
    if let Some(inner) = inner {
        let subpacket_len = inner.len() + 1; // Type octet plus embedded signature.
        body.extend(((subpacket_len + 1) as u16).to_be_bytes()); // Include its length octet.
        body.push(subpacket_len as u8);
        body.push(32); // Embedded signature subpacket.
        body.extend_from_slice(inner);
    } else {
        body.extend([0, 0]); // Empty unhashed subpacket area.
    }
    body.extend([0, 0, 0, 1, 1]); // Hash prefix plus one-bit RSA MPI.
    body
}

fn encrypted_data(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    let encrypted = SymEncryptedProtectedData::encrypt_seipdv1(
        StdRng::seed_from_u64(302),
        SymmetricKeyAlgorithm::AES256,
        key,
        &literal(plaintext),
    )
    .unwrap();
    packet(18, &encrypted.to_bytes().unwrap())
}

fn skesk_object(password: Option<&Password>, s2k: StringToKey) -> Vec<u8> {
    let key = [0x61; 32];
    let wrapper = SymKeyEncryptedSessionKey::encrypt_v4(
        password.unwrap_or(&Password::empty()),
        &key.to_vec().into(),
        s2k,
        SymmetricKeyAlgorithm::AES256,
    )
    .unwrap();
    [
        packet(3, &wrapper.to_bytes().unwrap()),
        encrypted_data(&key, b"admitted"),
    ]
    .concat()
}

fn fixture_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/test-keys")
}

fn public_key_export() -> Vec<u8> {
    std::fs::read(fixture_root().join("public.asc")).unwrap()
}

fn public_end(body: &[u8]) -> usize {
    match body[5] {
        1..=3 => {
            let first = super::mpi_end(body, 6).unwrap();
            super::mpi_end(body, first).unwrap()
        }
        18 | 22 => {
            let oid_end = 7 + body[6] as usize;
            let mpi_end = super::mpi_end(body, oid_end).unwrap();
            if body[5] == 18 {
                mpi_end + 4
            } else {
                mpi_end
            }
        }
        _ => panic!("unexpected fixture key algorithm"),
    }
}

fn protected_fixture_key_frame() -> Frame {
    let secret = std::fs::read(fixture_root().join("secret.asc")).unwrap();
    let binary = key_bytes(&secret, true, CryptoLimits::default().key_bytes).unwrap();
    let mut work = Work::new(CryptoLimits::default());
    let mut packets = frames(&binary, &mut work).unwrap();
    let packet_index = packets
        .iter()
        .position(|packet| {
            matches!(packet.tag, 5 | 7) && packet.body[public_end(&packet.body)] == 254
        })
        .expect("fixture has a protected secret-key packet");
    let mut packet = packets.swap_remove(packet_index);
    let count_offset = public_end(&packet.body) + 2 + 10;
    packet.body[count_offset] = 0;
    packet
}

#[test]
//= docs/parsing-requirements.md#parse-env-016
//= type=test
//# The adapter MUST enforce configured key, session-key packet, signature, nesting, and S2K-work limits before the corresponding unbounded work or allocation.
fn aggregate_export_and_certificate_allowlist_counts_admit_exact_bounds() {
    let key_a = b"ab";
    let key_b = b"cde";
    let secret_exports = [SecretExport {
        bytes: key_a,
        password: None,
    }];
    let signer_exports = [key_b.as_slice()];
    let exact_bytes = decode(
        &[],
        Credentials {
            secret_exports: &secret_exports,
            signer_exports: &signer_exports,
            ..Credentials::default()
        },
        CryptoLimits {
            certificates: 2,
            key_bytes: key_a.len() + key_b.len(),
            ..CryptoLimits::default()
        },
    );
    assert!(matches!(exact_bytes, Err(CryptoError::Malformed)));
    assert!(matches!(
        decode(
            &[],
            Credentials {
                secret_exports: &secret_exports,
                signer_exports: &signer_exports,
                ..Credentials::default()
            },
            CryptoLimits {
                certificates: 2,
                key_bytes: key_a.len() + key_b.len() - 1,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::KeyBytes))
    ));

    let third_signer = b"f";
    let signer_exports_over = [key_b.as_slice(), third_signer.as_slice()];
    assert!(matches!(
        decode(
            &[],
            Credentials {
                secret_exports: &secret_exports,
                signer_exports: &signer_exports_over,
                ..Credentials::default()
            },
            CryptoLimits {
                certificates: 2,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Certificates))
    ));
    assert!(matches!(
        decode(
            &[],
            Credentials {
                secret_exports: &secret_exports,
                signer_exports: &signer_exports,
                ..Credentials::default()
            },
            CryptoLimits {
                certificates: 1,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Certificates))
    ));

    let permitted = [[0x11; 20]];
    assert!(matches!(
        decode(
            &[],
            Credentials {
                permitted_signers: &permitted,
                ..Credentials::default()
            },
            CryptoLimits {
                certificates: 1,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Malformed)
    ));
    let permitted_over = [[0x11; 20], [0x22; 20]];
    assert!(matches!(
        decode(
            &[],
            Credentials {
                permitted_signers: &permitted_over,
                ..Credentials::default()
            },
            CryptoLimits {
                certificates: 1,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Certificates))
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-env-016
//= type=test
//# The adapter MUST enforce configured key, session-key packet, signature, nesting, and S2K-work limits before the corresponding unbounded work or allocation.
fn session_signature_and_depth_limits_accept_exact_and_precede_malformed_tails() {
    for (tag, limit, expected) in [
        (
            3,
            CryptoLimits {
                session_packets: 1,
                ..CryptoLimits::default()
            },
            Limit::SessionPackets,
        ),
        (
            2,
            CryptoLimits {
                signatures: 1,
                ..CryptoLimits::default()
            },
            Limit::Signatures,
        ),
        (
            4,
            CryptoLimits {
                signatures: 1,
                ..CryptoLimits::default()
            },
            Limit::Signatures,
        ),
    ] {
        let one = packet(tag, &[]);
        let mut work = Work::new(limit);
        assert_eq!(frames(&one, &mut work).unwrap().len(), 1, "exact tag {tag}");
        let over = [one.as_slice(), &[0xc0 | tag][..]].concat();
        assert!(matches!(
            frames(&over, &mut Work::new(limit)),
            Err(CryptoError::Limit(found)) if found == expected
        ));
    }

    let mut exact = literal(b"depth two");
    for _ in 0..2 {
        exact = packet(8, &[&[0][..], &exact].concat());
    }
    assert_eq!(
        content(
            &exact,
            0,
            &mut Work::new(CryptoLimits {
                depth: 2,
                ..CryptoLimits::default()
            })
        )
        .unwrap()
        .bytes,
        b"depth two"
    );
    let over = packet(8, &[&[0][..], &exact].concat());
    assert!(matches!(
        content(
            &over,
            0,
            &mut Work::new(CryptoLimits {
                depth: 2,
                ..CryptoLimits::default()
            })
        ),
        Err(CryptoError::Limit(Limit::Depth))
    ));

    let leaf = signature_with_embedded(None);
    let one = signature_with_embedded(Some(&leaf));
    let two = signature_with_embedded(Some(&one));
    signature_preflight(&leaf, true, 0, &mut Work::new(CryptoLimits::default()))
        .expect("embedded signature leaf is valid");
    signature_preflight(&one, true, 0, &mut Work::new(CryptoLimits::default()))
        .expect("one embedded signature is valid");
    signature_preflight(
        &two,
        true,
        0,
        &mut Work::new(CryptoLimits {
            depth: 2,
            ..CryptoLimits::default()
        }),
    )
    .expect("exact embedded-signature depth is admitted");
    let three = signature_with_embedded(Some(&two));
    assert!(matches!(
        signature_preflight(
            &three,
            true,
            0,
            &mut Work::new(CryptoLimits {
                depth: 2,
                ..CryptoLimits::default()
            })
        ),
        Err(CryptoError::Limit(Limit::Depth))
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-env-016
//= type=test
//# The adapter MUST enforce configured key, session-key packet, signature, nesting, and S2K-work limits before the corresponding unbounded work or allocation.
fn exported_subkey_and_protected_secret_s2k_limits_use_small_bounds() {
    let public = public_key_export();
    let mut work = Work::new(CryptoLimits::default());
    let binary = key_bytes(&public, false, work.limits.key_bytes).unwrap();
    let parsed = frames(&binary, &mut work).unwrap();
    let subkey_count = parsed
        .iter()
        .filter(|packet| matches!(packet.tag, 7 | 14))
        .count();
    assert!(subkey_count > 0);
    key_packets(
        &public,
        false,
        &mut Work::new(CryptoLimits {
            subkeys_per_certificate: subkey_count,
            ..CryptoLimits::default()
        }),
    )
    .expect("exact subkey bound admits exported certificate");
    assert!(matches!(
        key_packets(
            &public,
            false,
            &mut Work::new(CryptoLimits {
                subkeys_per_certificate: subkey_count - 1,
                ..CryptoLimits::default()
            })
        ),
        Err(CryptoError::Limit(Limit::Subkeys))
    ));

    let mut secret_frame = protected_fixture_key_frame();
    let secret_start = public_end(&secret_frame.body) + 2;
    assert_eq!(secret_frame.body[secret_start], 3);
    let encoded_count = secret_frame.body[secret_start + 10];
    let decoded_count =
        (16 + (usize::from(encoded_count) & 15)) << ((usize::from(encoded_count) >> 4) + 6);
    assert!(decoded_count >= 1024);
    secret_frame.body[secret_start + 10] = 0; // Valid discrete encoded count => 1024.
    assert_eq!(s2k(&secret_frame.body[secret_start..], 1024).unwrap(), 11);
    assert!(key_policy(
        &secret_frame,
        &Work::new(CryptoLimits {
            s2k_bytes: 1024,
            ..CryptoLimits::default()
        })
    )
    .is_ok());
    assert!(matches!(
        key_policy(
            &secret_frame,
            &Work::new(CryptoLimits {
                s2k_bytes: 1023,
                ..CryptoLimits::default()
            })
        ),
        Err(CryptoError::Limit(Limit::S2kWork))
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-env-016
//= type=test
//# The adapter MUST enforce configured key, session-key packet, signature, nesting, and S2K-work limits before the corresponding unbounded work or allocation.
fn s2k_and_shared_derivation_edges_are_checked_in_both_credential_paths() {
    let count_zero = StringToKey::IteratedAndSalted {
        hash_alg: HashAlgorithm::Sha256,
        salt: [0x22; 8],
        count: 0, // Discrete wire count 0 represents 1024 bytes of S2K work.
    };
    let no_password = skesk_object(None, count_zero.clone());
    assert!(matches!(
        decode(
            &no_password,
            Credentials::default(),
            CryptoLimits {
                s2k_bytes: 1024,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::MissingPassword)
    ));
    assert!(matches!(
        decode(
            &no_password,
            Credentials::default(),
            CryptoLimits {
                s2k_bytes: 1023,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::S2kWork))
    ));

    let password = Password::from("cheap local derivation");
    let symmetric = skesk_object(Some(&password), count_zero);
    let passwords = [password];
    assert_eq!(
        decode(
            &symmetric,
            Credentials {
                message_passwords: &passwords,
                ..Credentials::default()
            },
            CryptoLimits {
                derivations: 1,
                ..CryptoLimits::default()
            }
        )
        .unwrap()
        .bytes,
        b"admitted"
    );
    assert!(matches!(
        decode(
            &symmetric,
            Credentials {
                message_passwords: &passwords,
                ..Credentials::default()
            },
            CryptoLimits {
                derivations: 0,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Derivations))
    ));

    let ecdh_subkey = SubkeyParamsBuilder::default()
        .key_type(KeyType::ECDH(ECCCurve::Curve25519Legacy))
        .can_encrypt(EncryptionCaps::All)
        .build()
        .unwrap();
    let secret = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_certify(true)
        .can_sign(true)
        .primary_user_id("resource edge fixture".into())
        .subkeys(vec![ecdh_subkey])
        .build()
        .unwrap()
        .generate(StdRng::seed_from_u64(303))
        .unwrap();
    let secret_export = secret.to_bytes().unwrap();
    let mut rng = StdRng::seed_from_u64(304);
    let mut builder = MessageBuilder::from_bytes("", b"pkesk edge".to_vec())
        .seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
    builder
        .encrypt_to_key(&mut rng, &secret.secret_subkeys[0].public_key())
        .unwrap();
    let public_key_object = builder.to_vec(&mut rng).unwrap();
    let exports = [SecretExport {
        bytes: &secret_export,
        password: None,
    }];
    assert_eq!(
        decode(
            &public_key_object,
            Credentials {
                secret_exports: &exports,
                ..Credentials::default()
            },
            CryptoLimits {
                derivations: 1,
                ..CryptoLimits::default()
            }
        )
        .unwrap()
        .bytes,
        b"pkesk edge"
    );
    assert!(matches!(
        decode(
            &public_key_object,
            Credentials {
                secret_exports: &exports,
                ..Credentials::default()
            },
            CryptoLimits {
                derivations: 0,
                ..CryptoLimits::default()
            }
        ),
        Err(CryptoError::Limit(Limit::Derivations))
    ));

    let mut counter = Work::new(CryptoLimits {
        derivations: 1,
        ..CryptoLimits::default()
    });
    counter.derivation().unwrap();
    assert!(matches!(
        counter.derivation(),
        Err(CryptoError::Limit(Limit::Derivations))
    ));
}

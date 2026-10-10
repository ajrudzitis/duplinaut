use duplicity_core::envelope::*;
use duplicity_core::manifest::{DeclaredDigest, DigestAlgorithm, Manifest, ManifestLimits};
use duplicity_core::names::Envelope;
use std::io::{Read, Write};
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}
fn declaration(algorithm: DigestAlgorithm, hex: &[u8]) -> DeclaredDigest {
    DeclaredDigest {
        algorithm,
        name: Vec::new(),
        hex: hex.to_vec(),
    }
}

//= docs/parsing-requirements.md#parse-env-005
//= type=test
//# Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.
//= docs/parsing-requirements.md#parse-env-006
//= type=test
//# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
#[test]
fn every_supported_digest_required_and_public_declarations_revalidated() {
    let sha = declaration(
        DigestAlgorithm::Sha1,
        b"A9993E364706816ABA3E25717850C26C9CD0D89D",
    );
    let md5 = declaration(DigestAlgorithm::Md5, b"900150983cd24fb0d6963f7d28e17f72");
    let unknown = declaration(DigestAlgorithm::Unsupported, b"anything");
    assert_eq!(
        verify_stored_digests(b"abc", &[sha.clone(), md5.clone(), unknown.clone()], 3),
        Ok(())
    );
    assert_eq!(
        verify_stored_digests(b"abc", &[unknown], 3),
        Err(EnvelopeError::UnsupportedIntegrity)
    );
    assert_eq!(
        verify_stored_digests(b"abc", &[], 3),
        Err(EnvelopeError::UnsupportedIntegrity)
    );
    assert_eq!(
        verify_stored_digests(b"abc", std::slice::from_ref(&sha), 2),
        Err(EnvelopeError::StoredLimit)
    );
    let wrong = declaration(DigestAlgorithm::Md5, b"00000000000000000000000000000000");
    assert_eq!(
        verify_stored_digests(b"abc", &[sha.clone(), wrong.clone()], 3),
        Err(EnvelopeError::DigestMismatch)
    );
    assert_eq!(
        verify_stored_digests(b"abc", &[md5, wrong], 3),
        Err(EnvelopeError::ConflictingDigest)
    );
    for hex in [b"123" as &[u8], b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"] {
        assert_eq!(
            verify_stored_digests(b"abc", &[declaration(DigestAlgorithm::Sha1, hex)], 3),
            Err(EnvelopeError::InvalidDigest)
        );
    }
    assert_eq!(
        decode_verified_volume(
            b"broken gzip",
            Envelope::Gzip,
            &[sha],
            EnvelopeLimits::default()
        ),
        Err(EnvelopeError::DigestMismatch)
    );
}

//= docs/parsing-requirements.md#parse-env-001
//= type=test
//# The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.
#[test]
fn gzip_footer_consecutive_members_trailing_bytes_and_limits() {
    let first = gzip(b"abc");
    let second = gzip(b"def");
    let limits = EnvelopeLimits {
        stored_bytes: 1000,
        decoded_bytes: 6,
        gzip_members: 2,
    };
    let joined = [first.clone(), second].concat();
    assert_eq!(
        decode_metadata(&joined, Envelope::Gzip, limits).unwrap(),
        b"abcdef"
    );
    assert_eq!(
        decode_metadata(
            &joined,
            Envelope::Gzip,
            EnvelopeLimits {
                gzip_members: 1,
                ..limits
            }
        ),
        Err(EnvelopeError::MemberLimit)
    );
    assert_eq!(
        decode_metadata(
            &joined,
            Envelope::Gzip,
            EnvelopeLimits {
                decoded_bytes: 5,
                ..limits
            }
        ),
        Err(EnvelopeError::DecodedLimit)
    );
    assert_eq!(
        decode_metadata(
            &first,
            Envelope::Gzip,
            EnvelopeLimits {
                decoded_bytes: 3,
                ..limits
            }
        )
        .unwrap(),
        b"abc"
    );
    for suffix in [b"\0" as &[u8], b"junk", b"\x1f\x8b"] {
        assert_eq!(
            decode_metadata(
                &[first.clone(), suffix.to_vec()].concat(),
                Envelope::Gzip,
                limits
            ),
            Err(EnvelopeError::InvalidGzip)
        );
    }
    for index in [first.len() - 8, first.len() - 4] {
        let mut corrupt = first.clone();
        corrupt[index] ^= 1;
        assert_eq!(
            decode_metadata(&corrupt, Envelope::Gzip, limits),
            Err(EnvelopeError::InvalidGzip)
        );
    }
    for length in 0..first.len() {
        assert!(decode_metadata(&first[..length], Envelope::Gzip, limits).is_err());
    }
    assert_eq!(
        decode_metadata(b"abc", Envelope::Plain, limits).unwrap(),
        b"abc"
    );
    assert_eq!(
        decode_metadata(
            b"abc",
            Envelope::Plain,
            EnvelopeLimits {
                decoded_bytes: 2,
                ..limits
            }
        ),
        Err(EnvelopeError::DecodedLimit)
    );
    assert_eq!(
        decode_metadata(
            &gzip(b""),
            Envelope::Gzip,
            EnvelopeLimits {
                decoded_bytes: 0,
                ..limits
            }
        )
        .unwrap(),
        b""
    );
    for envelope in [Envelope::OpenPgp, Envelope::Pkcs7] {
        assert_eq!(
            decode_metadata(b"", envelope, limits),
            Err(EnvelopeError::UnsupportedEnvelope)
        );
    }
}

//= docs/parsing-requirements.md#parse-env-001
//= type=test
//# The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.
//= docs/parsing-requirements.md#parse-env-005
//= type=test
//# Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.
#[test]
fn official_volumes_match_recorded_manifest_digests_before_decode() {
    fn walk(root: &std::path::Path, checked: &mut usize) {
        for item in std::fs::read_dir(root).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                walk(&path, checked);
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy();
            if !name.ends_with(".manifest") && !name.ends_with(".m") {
                continue;
            }
            let manifest =
                Manifest::parse(&std::fs::read(&path).unwrap(), ManifestLimits::default()).unwrap();
            for volume in manifest.volumes() {
                let prefix = name
                    .strip_suffix(".manifest")
                    .or_else(|| name.strip_suffix(".m"))
                    .unwrap();
                let long = format!("{prefix}.vol{}.difftar.gz", volume.number);
                let short = format!("{prefix}.{}.dt.z", volume.number);
                let object = [path.with_file_name(long), path.with_file_name(short)]
                    .into_iter()
                    .find(|p| p.exists())
                    .unwrap();
                let stored = std::fs::read(&object).unwrap();
                let decoded = decode_verified_volume(
                    &stored,
                    Envelope::Gzip,
                    &volume.hashes,
                    EnvelopeLimits::default(),
                )
                .unwrap();
                let mut oracle = Vec::new();
                flate2::read::GzDecoder::new(stored.as_slice())
                    .read_to_end(&mut oracle)
                    .unwrap();
                assert_eq!(decoded, oracle);
                let mut corrupted = stored.clone();
                corrupted[0] ^= 1;
                assert_eq!(
                    decode_verified_volume(
                        &corrupted,
                        Envelope::Gzip,
                        &volume.hashes,
                        EnvelopeLimits::default()
                    ),
                    Err(EnvelopeError::DigestMismatch)
                );
                *checked += 1;
            }
        }
    }
    let mut checked = 0;
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
        &mut checked,
    );
    assert!(checked >= 20, "only {checked} official volumes checked");
}

//= docs/parsing-requirements.md#parse-env-001
//= type=test
//# The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.
#[test]
fn gzip_optional_header_fields_are_bounded_and_validated() {
    let plain = gzip(b"abc");
    let limits = EnvelopeLimits {
        stored_bytes: 200_000,
        decoded_bytes: 3,
        gzip_members: 1,
    };
    for flag in [0x20, 0x40, 0x80] {
        let mut malformed = plain.clone();
        malformed[3] |= flag;
        assert_eq!(
            decode_metadata(&malformed, Envelope::Gzip, limits),
            Err(EnvelopeError::InvalidGzip)
        );
    }
    for flag in [0x08, 0x10] {
        let mut header = plain[..10].to_vec();
        header[3] = flag;
        header.extend(std::iter::repeat_n(b'a', 65_535));
        let mut valid = header.clone();
        valid.push(0);
        valid.extend_from_slice(&plain[10..]);
        assert_eq!(
            decode_metadata(&valid, Envelope::Gzip, limits).unwrap(),
            b"abc"
        );
        header.push(b'a');
        header.push(0);
        header.extend_from_slice(&plain[10..]);
        assert_eq!(
            decode_metadata(&header, Envelope::Gzip, limits),
            Err(EnvelopeError::InvalidGzip)
        );
    }
    let mut extra = plain[..10].to_vec();
    extra[3] = 0x04;
    extra.extend_from_slice(&u16::MAX.to_le_bytes());
    extra.extend(std::iter::repeat_n(0x55, usize::from(u16::MAX)));
    extra.extend_from_slice(&plain[10..]);
    assert_eq!(
        decode_metadata(&extra, Envelope::Gzip, limits).unwrap(),
        b"abc"
    );
    assert_eq!(
        decode_metadata(&extra[..12], Envelope::Gzip, limits),
        Err(EnvelopeError::InvalidGzip)
    );
    // A present FHCRC cannot be ignored. Zero is incorrect for this header.
    let mut bad_crc = plain[..10].to_vec();
    bad_crc[3] = 0x02;
    bad_crc.extend_from_slice(&[0, 0]);
    bad_crc.extend_from_slice(&plain[10..]);
    assert_eq!(
        decode_metadata(&bad_crc, Envelope::Gzip, limits),
        Err(EnvelopeError::InvalidGzip)
    );
}

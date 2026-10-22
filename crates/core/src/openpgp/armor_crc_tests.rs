use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::Path;

fn base64(text: &str) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut accumulator = 0u32;
    let mut bits = 0;
    let mut decoded = Vec::new();
    for byte in text
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
    {
        let value = alphabet.iter().position(|v| *v == byte).unwrap() as u32;
        accumulator = (accumulator << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            decoded.push((accumulator >> bits) as u8);
        }
    }
    decoded
}
fn crc24(bytes: &[u8]) -> u32 {
    let mut crc = 0xb704ce;
    for byte in bytes {
        crc ^= u32::from(*byte) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x1000000 != 0 {
                crc ^= 0x1864cfb;
            }
        }
    }
    crc & 0xffffff
}
#[test]
fn original_armor_crc_is_valid_but_rpgp_optional_checker_discards_hash_updates() {
    assert_eq!(crc24(b"123456789"), 0x21cf02);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7");
    let provenance: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("provenance.json")).unwrap()).unwrap();
    for name in ["secret.asc", "public.asc"] {
        let path = format!("test-keys/{name}");
        let original = std::fs::read(root.join(&path)).unwrap();
        let identity = provenance["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == path)
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&original)),
            identity["sha256"].as_str().unwrap()
        );
        let text = std::str::from_utf8(&original).unwrap();
        let body: String = text
            .lines()
            .skip_while(|line| !line.is_empty())
            .skip(1)
            .take_while(|line| !line.starts_with('=') && !line.starts_with("-----"))
            .collect();
        let decoded = base64(&body);
        let checksum = base64(
            text.lines()
                .find(|line| line.starts_with('='))
                .unwrap()
                .trim_start_matches('='),
        );
        let expected =
            (u32::from(checksum[0]) << 16) | (u32::from(checksum[1]) << 8) | u32::from(checksum[2]);
        assert_eq!(crc24(&decoded), expected, "{name} independent checksum");
        let mut normal = pgp::armor::Dearmor::new(Cursor::new(&original));
        let mut native = Vec::new();
        normal.read_to_end(&mut native).unwrap();
        assert_eq!(native, decoded);
        let mut checked = pgp::armor::Dearmor::with_options(
            Cursor::new(&original),
            pgp::armor::DearmorOptions::new().enable_crc24_check(),
        );
        assert!(checked.read_to_end(&mut Vec::new()).is_err());
        assert!(
            matches!(checked.crc24_status(), pgp::armor::ArmorCrc24Status::CheckedInvalid { footer_crc, calculated_crc: 0xb704ce } if footer_crc == expected)
        );
    }
}

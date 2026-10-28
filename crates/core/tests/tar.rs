use duplicity_core::tar::{
    ArchiveKind, EntryType, Operation, TarError, TarLimits, TarTime, TarVolume, Termination,
};
use std::io::Read;

fn checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: u64 = header.iter().map(|b| u64::from(*b)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}
fn signed_checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: i64 = header
        .iter()
        .map(|byte| {
            if *byte >= 128 {
                i64::from(*byte) - 256
            } else {
                i64::from(*byte)
            }
        })
        .sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}
fn member(name: &[u8], kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 512];
    bytes[..name.len()].copy_from_slice(name);
    bytes[100..108].copy_from_slice(b"0000644\0");
    bytes[108..116].copy_from_slice(b"0000000\0");
    bytes[116..124].copy_from_slice(b"0000000\0");
    bytes[124..136].copy_from_slice(format!("{:011o}\0", payload.len()).as_bytes());
    bytes[136..148].copy_from_slice(b"00000000001\0");
    bytes[156] = kind;
    bytes[257..265].copy_from_slice(b"ustar\x0000");
    checksum(&mut bytes);
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(512), 0);
    bytes
}
fn symlink_with_linkname(link: &[u8]) -> Vec<u8> {
    let mut bytes = member(b"snapshot/link", b'2', b"");
    bytes[157..157 + link.len()].copy_from_slice(link);
    checksum(&mut bytes[..512]);
    bytes
}
fn record(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut length = key.len() + value.len() + 4;
    loop {
        let text = format!("{length} ");
        let actual = text.len() + key.len() + value.len() + 2;
        if actual == length {
            return [text.as_bytes(), key, b"=", value, b"\n"].concat();
        }
        length = actual;
    }
}
fn parse(bytes: &[u8]) -> Result<TarVolume<'_>, TarError> {
    TarVolume::parse(bytes, Termination::Data, TarLimits::default())
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

//= docs/parsing-requirements.md#parse-tar-002
//= type=test
//# The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

#[test]
fn complete_entry_extents_padding_and_checksum() {
    let mut bytes = member(b"snapshot/file", b'0', b"hello");
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].payload, b"hello");
    assert_eq!(parsed.entry_stream_bytes, 1024);
    assert_eq!(
        parsed.entries[0]
            .classify(ArchiveKind::Data, 1024, 16)
            .unwrap()
            .operation,
        Operation::Snapshot
    );
    bytes[1023] = 0xa5; // Padding bytes are not content and need not be zero.
    assert_eq!(parse(&bytes).unwrap().entries[0].payload, b"hello");
    for size in 0..1024 {
        if size == 0 {
            continue;
        }
        assert!(parse(&bytes[..size]).is_err(), "accepted truncation {size}");
    }
    bytes[1] ^= 1;
    assert_eq!(parse(&bytes).unwrap_err(), TarError::Checksum);
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

#[test]
fn tar_checksum_admission_accepts_unsigned_signed_and_rejects_mismatch() {
    let unsigned_bytes = member(b"snapshot/file", b'0', b"hello");
    assert!(parse(&unsigned_bytes).is_ok());

    let mut signed_bytes = member(b"snapshot/file", b'0', b"hello");
    signed_bytes[0] = 0xff;
    signed_checksum(&mut signed_bytes[..512]);
    assert!(parse(&signed_bytes).is_ok());

    signed_bytes[0] ^= 1;
    assert_eq!(parse(&signed_bytes).unwrap_err(), TarError::Checksum);
}

//= docs/parsing-requirements.md#parse-tar-004
//= type=test
//# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

#[test]
fn ordinary_tar_entry_type_admission_matrix_is_explicit() {
    let supported = [
        (0, EntryType::Regular),
        (b'0', EntryType::Regular),
        (b'1', EntryType::Hardlink),
        (b'2', EntryType::Symlink),
        (b'3', EntryType::CharacterDevice),
        (b'4', EntryType::BlockDevice),
        (b'5', EntryType::Directory),
        (b'6', EntryType::Fifo),
    ];
    for (flag, expected) in supported {
        let bytes = member(b"snapshot/file", flag, b"");
        assert_eq!(parse(&bytes).unwrap().entries[0].entry_type, expected);
    }

    for flag in *b"123456" {
        let bytes = member(b"snapshot/file", flag, b"x");
        assert_eq!(parse(&bytes).unwrap_err(), TarError::UnsupportedEntry);
    }

    for flag in *b"7S" {
        let bytes = member(b"snapshot/file", flag, b"");
        assert_eq!(parse(&bytes).unwrap_err(), TarError::UnsupportedEntry);
    }
}

//= docs/parsing-requirements.md#parse-tar-002
//= type=test
//# The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

#[test]
fn tar_payload_extent_boundaries_preserve_payload_padding_and_truncation_errors() {
    for size in [0usize, 1, 511, 512, 513, 1023, 1024] {
        let payload = vec![0xa5; size];
        let bytes = member(b"snapshot/file", b'0', &payload);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.entries[0].payload, payload);
        let padded_size = size.div_ceil(512) * 512;
        assert_eq!(parsed.entry_stream_bytes, 512 + padded_size);
    }

    let mut truncated = member(b"snapshot/file", b'0', &vec![0x5a; 513]);
    truncated.truncate(512 + 512);
    assert_eq!(parse(&truncated).unwrap_err(), TarError::Truncated);
}

//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

#[test]
fn footer_profiles_are_explicit() {
    let entry = member(b"snapshot/file", b'0', b"hello");
    assert!(TarVolume::parse(&entry, Termination::Signatures, TarLimits::default()).is_err());
    assert!(TarVolume::parse(&[], Termination::Signatures, TarLimits::default()).is_ok());
    for blocks in [1, 2, 20] {
        let bytes = [entry.clone(), vec![0; blocks * 512]].concat();
        assert!(parse(&bytes).is_ok());
        assert!(TarVolume::parse(&bytes, Termination::Signatures, TarLimits::default()).is_ok());
    }
    let mut bad = [entry.clone(), vec![0; 512]].concat();
    bad.push(0);
    assert!(parse(&bad).is_err());
    bad[1024] = 1;
    assert!(parse(&bad).is_err());
    let encrypted = [entry.clone(), vec![0; 10240 - entry.len() % 10240]].concat();
    let stored = b"ciphertext bytes";
    assert!(TarVolume::parse(
        &encrypted,
        Termination::Encrypted {
            stored_ciphertext: stored,
            nonfinal: false
        },
        TarLimits::default()
    )
    .is_ok());
    let topped = [encrypted.clone(), stored[..6].to_vec()].concat();
    assert!(TarVolume::parse(
        &topped,
        Termination::Encrypted {
            stored_ciphertext: stored,
            nonfinal: true
        },
        TarLimits::default()
    )
    .is_ok());
    assert!(TarVolume::parse(
        &topped,
        Termination::Encrypted {
            stored_ciphertext: stored,
            nonfinal: false
        },
        TarLimits::default()
    )
    .is_err());
    let wrong = [encrypted, b"cipherX".to_vec()].concat();
    assert!(TarVolume::parse(
        &wrong,
        Termination::Encrypted {
            stored_ciphertext: stored,
            nonfinal: true
        },
        TarLimits::default()
    )
    .is_err());
    let short_footer = [entry, vec![0; 512]].concat();
    assert!(TarVolume::parse(
        &short_footer,
        Termination::Encrypted {
            stored_ciphertext: stored,
            nonfinal: true
        },
        TarLimits::default()
    )
    .is_err());
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn gnu_base256_signed_time_and_signed_checksum() {
    let mut bytes = member(b"snapshot/file", b'0', b"hello");
    bytes[124..136].fill(0);
    bytes[124] = 0x80;
    bytes[135] = 5;
    bytes[136..148].fill(0xff); // GNU negative binary: -1 second.
    bytes[0] = 0xff;
    checksum(&mut bytes[..512]);
    let parsed = parse(&bytes).unwrap();
    assert_eq!(
        parsed.entries[0].mtime,
        TarTime {
            seconds: -1,
            nanoseconds: 0
        }
    );
    let sum: i64 = bytes[..512]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if (148..156).contains(&i) {
                32
            } else {
                i64::from(*b as i8)
            }
        })
        .sum();
    bytes[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    assert!(parse(&bytes).is_ok());
    bytes[124..136].fill(0xff);
    checksum(&mut bytes[..512]);
    assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

//= docs/parsing-requirements.md#parse-tar-004
//= type=test
//# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

#[test]
fn posix_prefix_and_gnu_header_are_distinct() {
    let mut bytes = member(b"file", b'0', b"");
    bytes[345..354].copy_from_slice(b"snapshot/");
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap().entries[0].name, b"snapshot//file");
    bytes[257..265].copy_from_slice(b"ustar  \0");
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap().entries[0].name, b"file");
    bytes[257..265].fill(0);
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap().entries[0].name, b"file");
    bytes[257] = 1;
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap_err(), TarError::UnsupportedHeader);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn gnu_long_name_and_link_require_real_entries() {
    let name = b"snapshot/long-name\0";
    let link = b"target\0";
    let bytes = [
        member(b"././@LongLink", b'L', name),
        member(b"././@LongLink", b'K', link),
        member(b"short", b'2', b""),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].name, b"snapshot/long-name");
    assert_eq!(parsed.entries[0].linkname, b"target");
    assert_eq!(parsed.entries[0].entry_type, EntryType::Symlink);
    assert_eq!(
        parse(&member(b"././@LongLink", b'L', name)).unwrap_err(),
        TarError::DanglingOverride
    );
    assert!(parse(
        &[
            member(b"././@LongLink", b'L', b"bad\0x"),
            member(b"snapshot/f", b'0', b"")
        ]
        .concat()
    )
    .is_err());
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_path_precedes_pending_gnu_long_name_in_either_extension_order() {
    let gnu = member(b"././@LongLink", b'L', b"snapshot/gnu-name\0");
    let pax = member(b"local", b'x', &record(b"path", b"snapshot/pax-name"));
    let ordinary = member(b"raw", b'0', b"");

    for bytes in [
        [gnu.clone(), pax.clone(), ordinary.clone()].concat(),
        [pax.clone(), gnu.clone(), ordinary.clone()].concat(),
    ] {
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.entries[0].name, b"snapshot/pax-name");
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn empty_local_pax_path_allows_pending_gnu_long_name() {
    let gnu = member(b"././@LongLink", b'L', b"snapshot/gnu-name\0");
    let empty_pax_path = member(b"local", b'x', &record(b"path", b""));
    let ordinary = member(b"raw", b'0', b"");

    for bytes in [
        [gnu.clone(), empty_pax_path.clone(), ordinary.clone()].concat(),
        [empty_pax_path.clone(), gnu.clone(), ordinary.clone()].concat(),
    ] {
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.entries[0].name, b"snapshot/gnu-name");
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_linkpath_precedes_pending_gnu_long_link_in_either_extension_order() {
    let gnu = member(b"././@LongLink", b'K', b"target/gnu\0");
    let pax = member(b"local", b'x', &record(b"linkpath", b"target/pax"));
    let symlink = symlink_with_linkname(b"target/raw");

    for bytes in [
        [gnu.clone(), pax.clone(), symlink.clone()].concat(),
        [pax.clone(), gnu.clone(), symlink.clone()].concat(),
    ] {
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.entries[0].linkname, b"target/pax");
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn empty_local_pax_linkpath_allows_gnu_or_header_link_fallback() {
    let gnu = member(b"././@LongLink", b'K', b"target/gnu\0");
    let empty_pax_link = member(b"local", b'x', &record(b"linkpath", b""));
    let symlink = symlink_with_linkname(b"target/raw");

    for bytes in [
        [gnu.clone(), empty_pax_link.clone(), symlink.clone()].concat(),
        [empty_pax_link.clone(), gnu.clone(), symlink.clone()].concat(),
    ] {
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.entries[0].linkname, b"target/gnu");
    }

    let bytes = [empty_pax_link, symlink].concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].linkname, b"target/raw");
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn pax_global_local_deletion_binary_and_numeric_overrides() {
    let global = [record(b"uid", b"99"), record(b"path", b"snapshot/global")].concat();
    let local = [
        record(b"path", b"snapshot/raw-\xff"),
        record(b"hdrcharset", b"BINARY"),
        record(b"mtime", b"-0.25"),
        record(b"atime", b"1.123456789000"),
        record(b"ctime", b"+2"),
        record(b"gid", b"123"),
        record(b"uname", b"user"),
        record(b"gname", b"group"),
    ]
    .concat();
    let deletion = [record(b"uid", b""), record(b"path", b"")].concat();
    let bytes = [
        member(b"global", b'g', &global),
        member(b"local", b'x', &local),
        member(b"raw", b'0', b""),
        member(b"local", b'x', &deletion),
        member(b"snapshot/raw", b'0', b""),
        member(b"raw", b'0', b""),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].name, b"snapshot/raw-\xff");
    assert_eq!(
        parsed.entries[0].mtime,
        TarTime {
            seconds: -1,
            nanoseconds: 750_000_000
        }
    );
    assert_eq!(
        parsed.entries[0].atime,
        Some(TarTime {
            seconds: 1,
            nanoseconds: 123_456_789
        })
    );
    assert_eq!(
        parsed.entries[0].ctime,
        Some(TarTime {
            seconds: 2,
            nanoseconds: 0
        })
    );
    assert_eq!((parsed.entries[0].uid, parsed.entries[0].gid), (99, 123));
    assert_eq!(parsed.entries[1].name, b"snapshot/raw");
    assert_eq!(parsed.entries[1].uid, 0);
    assert_eq!(parsed.entries[2].name, b"snapshot/global");
    assert_eq!(parsed.entries[2].uid, 99);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_uid_gid_global_local_deletion_and_header_fallback_are_exact() {
    let global = [
        record(b"uid", b"18446744073709551615"),
        record(b"gid", b"0"),
    ]
    .concat();
    let local = [
        record(b"uid", b"0"),
        record(b"gid", b"18446744073709551615"),
    ]
    .concat();
    let deletion = [record(b"uid", b""), record(b"gid", b"")].concat();
    let clear_global = [record(b"uid", b""), record(b"gid", b"")].concat();

    let mut raw = member(b"snapshot/raw", b'0', b"");
    raw[108..116].copy_from_slice(b"0000012\0"); // octal 10
    raw[116..124].copy_from_slice(b"0000023\0"); // octal 19
    checksum(&mut raw);
    let bytes = [
        member(b"global", b'g', &global),
        member(b"local", b'x', &local),
        raw.clone(),
        member(b"local", b'x', &deletion),
        raw.clone(),
        raw.clone(),
        member(b"global", b'g', &clear_global),
        raw,
    ]
    .concat();

    let parsed = parse(&bytes).unwrap();
    assert_eq!(
        (parsed.entries[0].uid, parsed.entries[0].gid),
        (0, u64::MAX)
    );
    assert_eq!((parsed.entries[1].uid, parsed.entries[1].gid), (10, 19));
    assert_eq!(
        (parsed.entries[2].uid, parsed.entries[2].gid),
        (u64::MAX, 0)
    );
    assert_eq!((parsed.entries[3].uid, parsed.entries[3].gid), (10, 19));
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn invalid_pax_uid_gid_values_are_numeric_errors() {
    for key in [b"uid" as &[u8], b"gid"] {
        for value in [b"+1" as &[u8], b"18446744073709551616"] {
            let bytes = [
                member(b"local", b'x', &record(key, value)),
                member(b"snapshot/raw", b'0', b""),
            ]
            .concat();
            assert_eq!(
                parse(&bytes).unwrap_err(),
                TarError::Numeric,
                "{key:?}={value:?}"
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn pax_size_override_controls_payload_and_empty_local_deletion_falls_back() {
    let global_size = member(b"global", b'g', &record(b"size", b"1"));
    let local_override = member(b"local", b'x', &record(b"size", b"2"));
    let local_delete = member(b"local", b'x', &record(b"size", b""));
    let bytes = [
        global_size,
        member(b"snapshot/global", b'0', b"abc"),
        local_override,
        member(b"snapshot/local", b'0', b"def"),
        local_delete,
        member(b"snapshot/header", b'0', b"ghi"),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].payload, b"a");
    assert_eq!(parsed.entries[1].payload, b"de");
    assert_eq!(parsed.entries[2].payload, b"ghi");
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn pax_mtime_global_local_and_empty_local_deletion_resolve_exactly() {
    let bytes = [
        member(b"global", b'g', &record(b"mtime", b"2.25")),
        member(b"snapshot/global", b'0', b""),
        member(b"local", b'x', &record(b"mtime", b"-0.000000001")),
        member(b"snapshot/local", b'0', b""),
        member(b"local", b'x', &record(b"mtime", b"")),
        member(b"snapshot/header", b'0', b""),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(
        parsed.entries[0].mtime,
        TarTime {
            seconds: 2,
            nanoseconds: 250_000_000
        }
    );
    assert_eq!(
        parsed.entries[1].mtime,
        TarTime {
            seconds: -1,
            nanoseconds: 999_999_999
        }
    );
    assert_eq!(
        parsed.entries[2].mtime,
        TarTime {
            seconds: 1,
            nanoseconds: 0
        }
    );
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn pax_atime_ctime_global_local_and_empty_local_deletion_resolve_exactly() {
    let bytes = [
        member(
            b"global",
            b'g',
            &[record(b"atime", b"1.125"), record(b"ctime", b"-2.5")].concat(),
        ),
        member(b"snapshot/global", b'0', b""),
        member(
            b"local",
            b'x',
            &[record(b"atime", b"+0.000000001"), record(b"ctime", b"4.5")].concat(),
        ),
        member(b"snapshot/local", b'0', b""),
        member(
            b"local",
            b'x',
            &[record(b"atime", b""), record(b"ctime", b"")].concat(),
        ),
        member(b"snapshot/deleted", b'0', b""),
        member(b"snapshot/global-again", b'0', b""),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(
        parsed.entries[0].atime,
        Some(TarTime {
            seconds: 1,
            nanoseconds: 125_000_000
        })
    );
    assert_eq!(
        parsed.entries[0].ctime,
        Some(TarTime {
            seconds: -3,
            nanoseconds: 500_000_000
        })
    );
    assert_eq!(
        parsed.entries[1].atime,
        Some(TarTime {
            seconds: 0,
            nanoseconds: 1
        })
    );
    assert_eq!(
        parsed.entries[1].ctime,
        Some(TarTime {
            seconds: 4,
            nanoseconds: 500_000_000
        })
    );
    assert_eq!(parsed.entries[2].atime, None);
    assert_eq!(parsed.entries[2].ctime, None);
    assert_eq!(parsed.entries[3].atime, parsed.entries[0].atime);
    assert_eq!(parsed.entries[3].ctime, parsed.entries[0].ctime);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn invalid_pax_atime_and_ctime_values_are_numeric_errors() {
    for (key, value) in [
        (b"atime" as &[u8], b"1.1234567891" as &[u8]),
        (b"ctime", b"-9223372036854775808.000000001"),
    ] {
        let bytes = [
            member(b"pax", b'x', &record(key, value)),
            member(b"snapshot/file", b'0', b""),
        ]
        .concat();
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn official_duplicity_307_pax_path_override_is_applied() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.vol1.difftar.gz");
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut decoded)
        .unwrap();
    let parsed = TarVolume::parse(&decoded, Termination::Data, TarLimits::default()).unwrap();
    assert!(parsed
        .entries
        .iter()
        .any(|entry| entry.name == b"multivol_snapshot/large.bin/1"));
    let overridden = parsed
        .entries
        .iter()
        .find(|entry| entry.name == b"multivol_snapshot/large.bin/1")
        .unwrap();
    assert_eq!(overridden.atime, None);
    assert_eq!(overridden.ctime, None);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

#[test]
fn official_duplicity_307_pax_path_preserves_header_mtime() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.vol1.difftar.gz");
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap())
        .read_to_end(&mut decoded)
        .unwrap();
    let parsed = TarVolume::parse(&decoded, Termination::Data, TarLimits::default()).unwrap();
    let mut archive = tar::Archive::new(decoded.as_slice());
    let expected: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.path_bytes().into_owned(),
                entry.header().mtime().unwrap(),
            )
        })
        .collect();
    assert_eq!(parsed.entries.len(), expected.len());
    for (native, (name, seconds)) in parsed.entries.iter().zip(expected) {
        assert_eq!(native.name, name);
        assert_eq!(
            native.mtime,
            TarTime {
                seconds: seconds as i64,
                nanoseconds: 0
            }
        );
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

//= docs/parsing-requirements.md#parse-tar-004
//= type=test
//# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

#[test]
fn malformed_and_unsupported_pax_are_not_skipped() {
    for payload in [
        b"0 path=x\n" as &[u8],
        b"99 path=x\n",
        b"12 path=x!",
        b"x path=x\n",
        b"8 a=b\n",
        b"9 path=\nX",
        b"10path=x\n",
    ] {
        assert!(
            parse(&member(b"pax", b'x', payload)).is_err(),
            "{payload:?}"
        );
    }
    for (key, value) in [
        (b"size" as &[u8], b"-1" as &[u8]),
        (b"uid", b"18446744073709551616"),
        (b"mtime", b"1.0000000001"),
        (b"mtime", b"1."),
        (b"mtime", b"1e3"),
        (b"hdrcharset", b"unknown"),
        (b"GNU.sparse.size", b"10"),
    ] {
        assert!(parse(&member(b"pax", b'x', &record(key, value))).is_err());
    }
    let duplicate = [record(b"size", b"1"), record(b"size", b"2")].concat();
    assert_eq!(
        parse(&member(b"pax", b'x', &duplicate)).unwrap_err(),
        TarError::Pax
    );
    let invalid_utf8 = [
        member(b"pax", b'x', &record(b"path", b"snapshot/\xff")),
        member(b"raw", b'0', b""),
    ]
    .concat();
    assert_eq!(parse(&invalid_utf8).unwrap_err(), TarError::TextEncoding);
    let size = [
        member(b"pax", b'x', &record(b"size", b"1")),
        member(b"snapshot/file", b'0', b"abc"),
    ]
    .concat();
    assert_eq!(parse(&size).unwrap().entries[0].payload, b"a");
}

//= docs/parsing-requirements.md#parse-entry-001
//= type=test
//# The reader MUST classify duplicity data entries using the five data-entry prefixes defined above.

//= docs/parsing-requirements.md#parse-entry-002
//= type=test
//# The reader MUST classify signature entries using the three signature-entry prefixes defined above.

//= docs/parsing-requirements.md#parse-entry-003
//= type=test
//# The reader MUST reject an unrecognized duplicity operation prefix in an otherwise recognized data or signature archive.

#[test]
fn entry_classification_and_limits() {
    for (name, archive, operation, part) in [
        (
            b"snapshot/." as &[u8],
            ArchiveKind::Data,
            Operation::Snapshot,
            None,
        ),
        (b"diff/file", ArchiveKind::Data, Operation::Diff, None),
        (b"deleted/file", ArchiveKind::Data, Operation::Deleted, None),
        (
            b"multivol_snapshot/file/1",
            ArchiveKind::Data,
            Operation::MultipartSnapshot,
            Some(1),
        ),
        (
            b"multivol_diff/file/2",
            ArchiveKind::Data,
            Operation::MultipartDiff,
            Some(2),
        ),
        (
            b"signature/file",
            ArchiveKind::Signatures,
            Operation::Signature,
            None,
        ),
    ] {
        let bytes = member(name, b'0', b"");
        let parsed = parse(&bytes).unwrap();
        let classified = parsed.entries[0].classify(archive, 1024, 16).unwrap();
        assert_eq!((classified.operation, classified.part), (operation, part));
    }
    for name in [
        b"other/file" as &[u8],
        b"snapshot/../file",
        b"multivol_diff/file/0",
        b"multivol_diff/file/18446744073709551616",
        b"multivol_diff/file/",
        b"snapshot//file",
    ] {
        let bytes = member(name, b'0', b"");
        assert!(parse(&bytes).unwrap().entries[0]
            .classify(ArchiveKind::Data, 1024, 16)
            .is_err());
    }
    let bytes = member(b"snapshot/file", b'0', b"hello");
    for limits in [
        TarLimits {
            max_bytes: bytes.len() - 1,
            ..TarLimits::default()
        },
        TarLimits {
            max_entries: 0,
            ..TarLimits::default()
        },
        TarLimits {
            max_metadata_bytes: 0,
            ..TarLimits::default()
        },
        TarLimits {
            max_path_bytes: 1,
            ..TarLimits::default()
        },
        TarLimits {
            max_entry_bytes: 4,
            ..TarLimits::default()
        },
    ] {
        assert_eq!(
            TarVolume::parse(&bytes, Termination::Data, limits).unwrap_err(),
            TarError::Limit
        );
    }
    let extension = [
        member(b"pax", b'x', &record(b"path", b"snapshot/file")),
        bytes,
    ]
    .concat();
    assert_eq!(
        TarVolume::parse(
            &extension,
            Termination::Data,
            TarLimits {
                max_extended_headers: 0,
                ..TarLimits::default()
            }
        )
        .unwrap_err(),
        TarError::Limit
    );
    assert_eq!(
        parse(&member(b"snapshot/f", b'S', b"")).unwrap_err(),
        TarError::UnsupportedEntry
    );
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

//= docs/parsing-requirements.md#parse-tar-002
//= type=test
//# The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

#[test]
fn all_official_plain_gzip_archives_scan_and_classify() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut pending = vec![root];
    let mut archives = 0;
    let mut entries = 0;
    while let Some(directory) = pending.pop() {
        for item in std::fs::read_dir(directory).unwrap() {
            let item = item.unwrap();
            if item.file_type().unwrap().is_dir() {
                pending.push(item.path());
                continue;
            }
            let name = item.file_name();
            let bytes = name.as_encoded_bytes();
            let signature = bytes.ends_with(b".sigtar.gz") || bytes.ends_with(b".st.z");
            if !signature && !bytes.ends_with(b".difftar.gz") && !bytes.ends_with(b".dt.z") {
                continue;
            }
            let mut decoded = Vec::new();
            flate2::read::GzDecoder::new(std::fs::File::open(item.path()).unwrap())
                .read_to_end(&mut decoded)
                .unwrap();
            let parsed = TarVolume::parse(
                &decoded,
                if signature {
                    Termination::Signatures
                } else {
                    Termination::Data
                },
                TarLimits::default(),
            )
            .unwrap_or_else(|error| panic!("{name:?}: {error:?}"));
            let mut oracle = tar::Archive::new(decoded.as_slice());
            let expected: Vec<_> = oracle
                .entries()
                .unwrap()
                .map(|entry| {
                    let mut entry = entry.unwrap();
                    let path = entry.path_bytes().into_owned();
                    let mut payload = Vec::new();
                    entry.read_to_end(&mut payload).unwrap();
                    (path, payload)
                })
                .collect();
            assert_eq!(parsed.entries.len(), expected.len(), "{name:?}");
            for (entry, (path, payload)) in parsed.entries.iter().zip(expected) {
                assert_eq!(entry.name, path);
                assert_eq!(entry.payload, payload);
                entry
                    .classify(
                        if signature {
                            ArchiveKind::Signatures
                        } else {
                            ArchiveKind::Data
                        },
                        1024 * 1024,
                        1024,
                    )
                    .unwrap();
                entries += 1;
            }
            archives += 1;
        }
    }
    assert_eq!(archives, 48);
    assert!(entries > 600);
}

//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

#[test]
fn encrypted_record_footer_handles_one_block_and_full_record() {
    for (payload_length, footer_length) in [(9216, 512), (9728, 10240)] {
        let entry = member(b"snapshot/file", b'0', &vec![b'x'; payload_length]);
        let bytes = [entry.clone(), vec![0; footer_length]].concat();
        let parsed = TarVolume::parse(
            &bytes,
            Termination::Encrypted {
                stored_ciphertext: b"stored",
                nonfinal: false,
            },
            TarLimits::default(),
        )
        .unwrap();
        assert_eq!(parsed.entry_stream_bytes, entry.len());
        assert_eq!(parsed.entries[0].payload.len(), payload_length);
        assert!(TarVolume::parse(
            &bytes[..bytes.len() - 512],
            Termination::Encrypted {
                stored_ciphertext: b"stored",
                nonfinal: false
            },
            TarLimits::default()
        )
        .is_err());
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn numeric_conversion_boundaries_and_interior_padding() {
    fn binary(field: &mut [u8], value: i128) {
        let encoded = value.to_be_bytes();
        let length = field.len();
        field.copy_from_slice(&encoded[16 - length..]);
        field[0] = if value < 0 { 0xff } else { 0x80 };
    }
    let mut bytes = member(b"snapshot/file", b'0', b"");
    binary(&mut bytes[100..108], i128::from(u32::MAX));
    binary(&mut bytes[136..148], i128::from(i64::MAX));
    checksum(&mut bytes);
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].mode, u32::MAX);
    assert_eq!(parsed.entries[0].mtime.seconds, i64::MAX);
    binary(&mut bytes[136..148], i128::from(i64::MIN));
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap().entries[0].mtime.seconds, i64::MIN);
    for value in [i128::from(i64::MIN) - 1, i128::from(i64::MAX) + 1] {
        binary(&mut bytes[136..148], value);
        checksum(&mut bytes);
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    }
    bytes = member(b"snapshot/file", b'0', b"");
    binary(&mut bytes[100..108], i128::from(u32::MAX) + 1);
    checksum(&mut bytes);
    assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    for range in [108..116, 116..124, 124..136] {
        let mut bytes = member(b"snapshot/file", b'0', b"");
        binary(&mut bytes[range], -1);
        checksum(&mut bytes);
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    }
    for value in [
        u64::MAX.to_string(),
        "18446744073709551616".into(),
        "-1".into(),
    ] {
        let bytes = [
            member(b"pax", b'x', &record(b"uid", value.as_bytes())),
            member(b"snapshot/f", b'0', b""),
        ]
        .concat();
        if value == u64::MAX.to_string() {
            assert_eq!(parse(&bytes).unwrap().entries[0].uid, u64::MAX);
        } else {
            assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
        }
    }
    for field in [b"001\x000001" as &[u8], b"001 0001"] {
        let mut bytes = member(b"snapshot/file", b'0', b"");
        bytes[108..116].copy_from_slice(field);
        checksum(&mut bytes);
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn trailing_global_state_is_valid_but_local_overrides_are_dangling() {
    let global = member(b"global", b'g', &record(b"uid", b"1"));
    assert!(parse(&global).unwrap().entries.is_empty());
    for bytes in [
        member(b"local", b'x', &record(b"uid", b"1")),
        member(b"long", b'L', b"snapshot/name\0"),
        member(b"link", b'K', b"target\0"),
    ] {
        assert_eq!(parse(&bytes).unwrap_err(), TarError::DanglingOverride);
        let with_footer = [bytes, vec![0; 512]].concat();
        assert_eq!(parse(&with_footer).unwrap_err(), TarError::DanglingOverride);
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_timestamp_precision_and_signed_extremes() {
    for (text, seconds, nanoseconds) in [
        (
            b"9223372036854775807.999999999" as &[u8],
            i64::MAX,
            999_999_999,
        ),
        (b"-9223372036854775808.000000000000", i64::MIN, 0),
        (b"-9223372036854775807.999999999", i64::MIN, 1),
        (b"-0.000000001", -1, 999_999_999),
        (b"+0.100000000000", 0, 100_000_000),
    ] {
        let bytes = [
            member(b"pax", b'x', &record(b"mtime", text)),
            member(b"snapshot/f", b'0', b""),
        ]
        .concat();
        assert_eq!(
            parse(&bytes).unwrap().entries[0].mtime,
            TarTime {
                seconds,
                nanoseconds
            }
        );
    }
    for text in [
        b"9223372036854775808" as &[u8],
        b"-9223372036854775808.000000001",
        b"18446744073709551616",
        b"1.1234567891",
        b"1.",
        b".1",
        b"1.2.3",
        b"--1",
        b"+",
        b"1.000000000x",
    ] {
        let bytes = [
            member(b"pax", b'x', &record(b"mtime", text)),
            member(b"snapshot/f", b'0', b""),
        ]
        .concat();
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric, "{text:?}");
    }
}

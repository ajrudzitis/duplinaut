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
fn member_with_owner_names(uname: &[u8], gname: &[u8]) -> Vec<u8> {
    assert!(uname.len() <= 32 && gname.len() <= 32);
    let mut bytes = member(b"snapshot/file", b'0', b"");
    bytes[265..265 + uname.len()].copy_from_slice(uname);
    bytes[297..297 + gname.len()].copy_from_slice(gname);
    checksum(&mut bytes[..512]);
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

//= docs/parsing-requirements.md#parse-tar-018
//= type=test
//# For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.
#[test]
fn entry_metadata_charge_exact_limit_and_rejection_are_atomic() {
    let one = member(b"snapshot/file", b'0', b"");
    let per_entry =
        std::mem::size_of::<duplicity_core::tar::TarEntry<'_>>() * 4 + b"snapshot/file".len();
    let exact = TarLimits {
        max_metadata_bytes: per_entry,
        ..TarLimits::default()
    };
    assert_eq!(
        TarVolume::parse(&one, Termination::Data, exact)
            .unwrap()
            .entries
            .len(),
        1
    );

    let one_short = TarLimits {
        max_metadata_bytes: per_entry - 1,
        ..TarLimits::default()
    };
    assert_eq!(
        TarVolume::parse(&one, Termination::Data, one_short).unwrap_err(),
        TarError::Limit
    );

    let two = [one.as_slice(), one.as_slice()].concat();
    let second_entry_over_limit = TarLimits {
        max_metadata_bytes: per_entry * 2 - 1,
        ..TarLimits::default()
    };
    // Parsing is atomic: the API returns no TarVolume containing the first
    // entry when admitting the second entry exceeds the aggregate limit.
    assert_eq!(
        TarVolume::parse(&two, Termination::Data, second_entry_over_limit).unwrap_err(),
        TarError::Limit
    );
}

//= docs/parsing-requirements.md#parse-tar-019
//= type=test
//# GNU `L`/`K` and PAX `x`/`g` extension headers MUST charge their raw-header payload size against the cumulative metadata limit before interpreting or retaining extension metadata. Arithmetic overflow or an exceeded limit MUST reject the volume without returning partial entries.
#[test]
fn extension_metadata_limits_are_charged_before_interpreting_each_extension_kind() {
    let raw_entry = member(b"snapshot/file", b'0', b"");
    let pax_path = record(b"path", b"snapshot/file");
    let cases: [(u8, &[u8], usize, usize); 4] = [
        (b'L', b"snapshot/file\0", b"snapshot/file".len(), 0),
        (b'K', b"target\0", b"snapshot/file".len(), b"target".len()),
        (b'x', &pax_path, b"snapshot/file".len(), 0),
        (b'g', &pax_path, b"snapshot/file".len(), 0),
    ];

    for (kind, payload, name_length, link_length) in cases {
        let extension = member(b"extension", kind, payload);
        let archive = [extension.as_slice(), raw_entry.as_slice()].concat();
        let entry_charge = std::mem::size_of::<duplicity_core::tar::TarEntry<'_>>() * 4
            + name_length
            + link_length;
        let exact = TarLimits {
            max_metadata_bytes: payload.len() + entry_charge,
            ..TarLimits::default()
        };
        assert!(
            TarVolume::parse(&archive, Termination::Data, exact).is_ok(),
            "extension kind {} must fit its exact raw-size charge",
            char::from(kind)
        );
        let one_short = TarLimits {
            max_metadata_bytes: payload.len() + entry_charge - 1,
            ..TarLimits::default()
        };
        assert_eq!(
            TarVolume::parse(&archive, Termination::Data, one_short).unwrap_err(),
            TarError::Limit,
            "extension kind {} must fail one byte below its cumulative charge",
            char::from(kind)
        );

        // Invalid payload syntax distinguishes the charge check from PAX or
        // GNU content interpretation: the exhausted budget wins first.
        let malformed_payload = b"!";
        let malformed = member(b"extension", kind, malformed_payload);
        let no_extension_budget = TarLimits {
            max_metadata_bytes: malformed_payload.len() - 1,
            ..TarLimits::default()
        };
        assert_eq!(
            TarVolume::parse(&malformed, Termination::Data, no_extension_budget).unwrap_err(),
            TarError::Limit,
            "extension kind {} must be charged before content interpretation",
            char::from(kind)
        );
    }
}

//= docs/parsing-requirements.md#parse-tar-019
//= type=test
//# GNU `L`/`K` and PAX `x`/`g` extension headers MUST charge their raw-header payload size against the cumulative metadata limit before interpreting or retaining extension metadata. Arithmetic overflow or an exceeded limit MUST reject the volume without returning partial entries.
#[test]
fn extension_metadata_charges_accumulate_across_headers() {
    let first_record = record(b"path", b"snapshot/file");
    let second_record = record(b"path", b"snapshot/other");
    let first = member(b"x1", b'x', &first_record);
    let second = member(b"x2", b'x', &second_record);
    let entry = member(b"snapshot/file", b'0', b"");
    let archive = [first.as_slice(), second.as_slice(), entry.as_slice()].concat();
    let entry_charge =
        std::mem::size_of::<duplicity_core::tar::TarEntry<'_>>() * 4 + b"snapshot/other".len();
    let exact_charge = first_record.len() + second_record.len() + entry_charge;

    assert!(TarVolume::parse(
        &archive,
        Termination::Data,
        TarLimits {
            max_metadata_bytes: exact_charge,
            ..TarLimits::default()
        },
    )
    .is_ok());
    assert_eq!(
        TarVolume::parse(
            &archive,
            Termination::Data,
            TarLimits {
                max_metadata_bytes: exact_charge - 1,
                ..TarLimits::default()
            },
        )
        .unwrap_err(),
        TarError::Limit
    );
}

//= docs/parsing-requirements.md#parse-tar-020
//= type=test
//# The reader MUST checked-increment the GNU/PAX extension-header count and reject the header if the increment overflows or exceeds `max_extended_headers`, before charging or interpreting extension contents.
#[test]
fn extension_header_count_is_bounded_before_content_interpretation() {
    let payload = record(b"path", b"snapshot/file");
    let entry = member(b"snapshot/file", b'0', b"");
    let exactly_one = [member(b"pax", b'x', &payload).as_slice(), entry.as_slice()].concat();
    assert!(TarVolume::parse(
        &exactly_one,
        Termination::Data,
        TarLimits {
            max_extended_headers: 1,
            ..TarLimits::default()
        },
    )
    .is_ok());

    for kind in *b"LKxg" {
        let malformed = member(b"extension", kind, b"!");
        assert_eq!(
            TarVolume::parse(
                &malformed,
                Termination::Data,
                TarLimits {
                    max_extended_headers: 0,
                    ..TarLimits::default()
                },
            )
            .unwrap_err(),
            TarError::Limit,
            "extension kind {} must be rejected by count before malformed contents are parsed",
            char::from(kind),
        );
    }

    let first = member(b"long-name", b'L', b"snapshot/file\0");
    let malformed_second = member(b"pax", b'x', b"!");
    let cumulative = [first.as_slice(), malformed_second.as_slice()].concat();
    assert_eq!(
        TarVolume::parse(
            &cumulative,
            Termination::Data,
            TarLimits {
                max_extended_headers: 1,
                ..TarLimits::default()
            },
        )
        .unwrap_err(),
        TarError::Limit,
        "the cumulative extension count must reject the second malformed header before parsing it",
    );
}

//= docs/parsing-requirements.md#parse-tar-021
//= type=test
//# Before allocating or retaining output fields for a real tar entry, the parser MUST require the accepted entry count to be below `max_entries`. If the count equals or exceeds the limit, it MUST return Limit without returning a partial entry list.
#[test]
fn real_entry_count_is_bounded_before_output_field_interpretation() {
    let first = member(b"snapshot/first", b'0', b"");
    let malformed_second = member(b"", b'0', b"");
    let archive = [first, malformed_second].concat();
    let one_entry = TarLimits {
        max_entries: 1,
        ..TarLimits::default()
    };
    assert_eq!(
        TarVolume::parse(&archive, Termination::Data, one_entry).unwrap_err(),
        TarError::Limit
    );

    let zero_entries = TarLimits {
        max_entries: 0,
        ..TarLimits::default()
    };
    assert_eq!(
        TarVolume::parse(
            &member(b"snapshot/file", b'0', b""),
            Termination::Data,
            zero_entries
        )
        .unwrap_err(),
        TarError::Limit
    );
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

//= docs/parsing-requirements.md#parse-tar-002
//= type=test
//# The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.
//= docs/parsing-requirements.md#parse-tar-008
//= type=test
//# After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary.
//= docs/parsing-requirements.md#parse-tar-008
//= type=test
//# It MUST reject configured entry-size excess and truncated extents with explicit errors.

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

//= docs/parsing-requirements.md#parse-tar-024
//= type=test
//# After decoding the checksum field, the parser MUST reject a header whose checksum does not match before interpreting magic/version or any other numeric header field; that mismatch MUST produce Checksum.
#[test]
fn checksum_mismatch_precedes_unsupported_magic_and_malformed_numeric_fields() {
    let mut bad_magic = member(b"snapshot/file", b'0', b"");
    bad_magic[257..265].copy_from_slice(b"badmagic");
    assert_eq!(parse(&bad_magic).unwrap_err(), TarError::Checksum);

    let mut bad_mode = member(b"snapshot/file", b'0', b"");
    bad_mode[100..108].copy_from_slice(b"0000X000");
    assert_eq!(parse(&bad_mode).unwrap_err(), TarError::Checksum);
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

//= docs/parsing-requirements.md#parse-tar-022
//= type=test
//# At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.
#[test]
fn eof_finalization_reports_dangling_override_before_footer_and_publishes_only_valid_volumes() {
    let local = member(b"PaxHeader", b'x', &record(b"path", b"snapshot/file"));
    let dangling_with_bad_footer = [local, vec![0; 512], b"x".to_vec()].concat();
    assert_eq!(
        TarVolume::parse(
            &dangling_with_bad_footer,
            Termination::Data,
            TarLimits::default()
        )
        .unwrap_err(),
        TarError::DanglingOverride
    );

    let entry = member(b"snapshot/file", b'0', b"");
    let invalid_footer = [entry.clone(), vec![0; 512], b"x".to_vec()].concat();
    assert_eq!(
        TarVolume::parse(&invalid_footer, Termination::Data, TarLimits::default()).unwrap_err(),
        TarError::Termination
    );
    assert!(TarVolume::parse(&entry, Termination::Data, TarLimits::default()).is_ok());

    let global = member(b"PaxHeader", b'g', &record(b"path", b"ignored"));
    let global_only = [global, vec![0; 512]].concat();
    assert!(TarVolume::parse(&global_only, Termination::Data, TarLimits::default()).is_ok());
}

//= docs/parsing-requirements.md#parse-tar-001
//= type=test
//# The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
//= docs/parsing-requirements.md#parse-tar-008
//= type=test
//# After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary.

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

//= docs/parsing-requirements.md#parse-tar-009
//= type=test
//# When PAX path and GNU long-name overrides are absent, a POSIX ustar header MUST use its raw name field if the prefix is empty and MUST join a nonempty prefix to the raw name with one slash byte.
//= docs/parsing-requirements.md#parse-tar-025
//= type=test
//# After checksum admission, the parser MUST accept a header only when its eight magic/version bytes are exactly POSIX ustar (`ustar\0` followed by `00`), old GNU (`ustar ` followed by ` \0`), or eight zero bytes for V7. Any other magic/version byte sequence MUST produce UnsupportedHeader. Only POSIX ustar headers may interpret bytes 345 through 499 as a path prefix; old GNU and V7 headers MUST NOT use those bytes as a prefix.

#[test]
fn posix_prefix_and_gnu_header_are_distinct() {
    let mut bytes = member(b"file", b'0', b"");
    assert_eq!(parse(&bytes).unwrap().entries[0].name, b"file");
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

//= docs/parsing-requirements.md#parse-tar-010
//= type=test
//# Raw tar text fields MUST end at the first NUL byte, or use the entire field when no NUL is present.
#[test]
fn raw_tar_name_and_link_fields_end_at_first_nul() {
    let bytes = [
        member(b"snapshot\0ignored", b'0', b""),
        symlink_with_linkname(b"target\0ignored"),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].name, b"snapshot");
    assert_eq!(parsed.entries[1].linkname, b"target");
}

//= docs/parsing-requirements.md#parse-tar-012
//= type=test
//# The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.
#[test]
fn effective_name_limit_uses_pax_gnu_or_raw_ustar_source_length() {
    let mut raw = member(b"file", b'0', b"");
    raw[345..349].copy_from_slice(b"dir/");
    checksum(&mut raw[..512]);
    let raw_archive = raw;

    let pax_archive = [
        member(b"local", b'x', &record(b"path", b"pax-name")),
        member(b"raw", b'0', b""),
    ]
    .concat();
    let gnu_archive = [
        member(b"././@LongLink", b'L', b"gnu-name\0"),
        member(b"raw", b'0', b""),
    ]
    .concat();

    for (archive, expected_name) in [
        (raw_archive.as_slice(), b"dir//file" as &[u8]),
        (pax_archive.as_slice(), b"pax-name"),
        (gnu_archive.as_slice(), b"gnu-name"),
    ] {
        let exact_limit = expected_name.len();
        assert_eq!(
            TarVolume::parse(
                archive,
                Termination::Data,
                TarLimits {
                    max_path_bytes: exact_limit,
                    ..TarLimits::default()
                },
            )
            .unwrap()
            .entries[0]
                .name,
            expected_name
        );
        assert_eq!(
            TarVolume::parse(
                archive,
                Termination::Data,
                TarLimits {
                    max_path_bytes: exact_limit - 1,
                    ..TarLimits::default()
                },
            )
            .unwrap_err(),
            TarError::Limit
        );
    }
}

//= docs/parsing-requirements.md#parse-tar-013
//= type=test
//# The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.
#[test]
fn effective_link_limit_uses_pax_gnu_or_raw_header_source_length() {
    let symlink = |target: &[u8]| {
        let mut header = member(b"file", b'2', b"");
        header[157..157 + target.len()].copy_from_slice(target);
        checksum(&mut header[..512]);
        header
    };
    let pax_archive = [
        member(b"local", b'x', &record(b"linkpath", b"pax-target")),
        symlink(b"raw"),
    ]
    .concat();
    let gnu_archive = [
        member(b"././@LongLink", b'K', b"gnu-target\0"),
        symlink(b"raw"),
    ]
    .concat();
    let raw_archive = symlink(b"raw-target");

    for (archive, expected_target) in [
        (pax_archive.as_slice(), b"pax-target" as &[u8]),
        (gnu_archive.as_slice(), b"gnu-target"),
        (raw_archive.as_slice(), b"raw-target"),
    ] {
        let exact_limit = expected_target.len();
        assert_eq!(
            TarVolume::parse(
                archive,
                Termination::Data,
                TarLimits {
                    max_path_bytes: exact_limit,
                    ..TarLimits::default()
                },
            )
            .unwrap()
            .entries[0]
                .linkname,
            expected_target
        );
        assert_eq!(
            TarVolume::parse(
                archive,
                Termination::Data,
                TarLimits {
                    max_path_bytes: exact_limit - 1,
                    ..TarLimits::default()
                },
            )
            .unwrap_err(),
            TarError::Limit
        );
    }
}

//= docs/parsing-requirements.md#parse-tar-014
//= type=test
//# The parser MUST compute the exact byte lengths of the selected PAX `uname` and `gname` values, or their raw header fallbacks, and reject values above `max_path_bytes` before allocating the output owner names.
#[test]
fn owner_name_limit_uses_pax_override_or_raw_header_fallback() {
    let cases = [
        (
            Some((b"uname" as &[u8], b"pax-user" as &[u8])),
            b"raw" as &[u8],
            b"grp" as &[u8],
            b"pax-user" as &[u8],
            b"grp" as &[u8],
        ),
        (
            Some((b"gname" as &[u8], b"pax-grp!" as &[u8])),
            b"raw" as &[u8],
            b"grp" as &[u8],
            b"raw" as &[u8],
            b"pax-grp!" as &[u8],
        ),
        (
            None,
            b"raw-user" as &[u8],
            b"grp" as &[u8],
            b"raw-user" as &[u8],
            b"grp" as &[u8],
        ),
        (
            None,
            b"raw" as &[u8],
            b"raw-grp!" as &[u8],
            b"raw" as &[u8],
            b"raw-grp!" as &[u8],
        ),
    ];

    for (override_field, uname, gname, expected_uname, expected_gname) in cases {
        let limit = expected_uname.len().max(expected_gname.len());
        let mut header = member_with_owner_names(uname, gname);
        header[..100].fill(0);
        header[0] = b'f';
        checksum(&mut header[..512]);
        let archive = match override_field {
            Some((key, value)) => [member(b"l", b'x', &record(key, value)), header].concat(),
            None => header,
        };
        let parsed = TarVolume::parse(
            &archive,
            Termination::Data,
            TarLimits {
                max_path_bytes: limit,
                ..TarLimits::default()
            },
        )
        .unwrap();
        assert_eq!(parsed.entries[0].uname, expected_uname);
        assert_eq!(parsed.entries[0].gname, expected_gname);
        assert_eq!(
            TarVolume::parse(
                &archive,
                Termination::Data,
                TarLimits {
                    max_path_bytes: limit - 1,
                    ..TarLimits::default()
                },
            )
            .unwrap_err(),
            TarError::Limit
        );
    }
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

//= docs/parsing-requirements.md#parse-tar-023
//= type=test
//# GNU `L` and `K` payloads MUST contain a NUL terminator, all bytes from the first NUL onward MUST be zero, and the selected prefix length MUST NOT exceed `max_path_bytes`.
#[test]
fn gnu_long_records_enforce_terminator_padding_and_path_bound() {
    let exact_name = [
        member(b"././@LongLink", b'L', b"snapshot/f\0\0\0"),
        member(b"short", b'0', b""),
    ]
    .concat();
    let exact = TarVolume::parse(
        &exact_name,
        Termination::Data,
        TarLimits {
            max_path_bytes: 10,
            ..TarLimits::default()
        },
    )
    .unwrap();
    assert_eq!(exact.entries[0].name, b"snapshot/f");

    let over_limit_name = [
        member(b"././@LongLink", b'L', b"snapshot/long\0\0"),
        member(b"short", b'0', b""),
    ]
    .concat();
    assert_eq!(
        TarVolume::parse(
            &over_limit_name,
            Termination::Data,
            TarLimits {
                max_path_bytes: 12,
                ..TarLimits::default()
            },
        )
        .unwrap_err(),
        TarError::Limit
    );

    let missing_terminator = [
        member(b"././@LongLink", b'L', b"snapshot/file"),
        member(b"short", b'0', b""),
    ]
    .concat();
    assert_eq!(
        parse(&missing_terminator).unwrap_err(),
        TarError::TextEncoding
    );

    let nonzero_tail = [
        member(b"././@LongLink", b'K', b"target\0x"),
        member(b"snapshot/link", b'2', b""),
    ]
    .concat();
    assert_eq!(parse(&nonzero_tail).unwrap_err(), TarError::TextEncoding);

    let zero_padded_link = [
        member(b"././@LongLink", b'K', b"target\0\0\0"),
        member(b"snapshot/link", b'2', b""),
    ]
    .concat();
    let linked = parse(&zero_padded_link).unwrap();
    assert_eq!(linked.entries[0].linkname, b"target");
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

//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# Local fields MUST override global fields for the next real entry.
//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# That local state MUST be consumed only after the real entry is accepted.
//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# The tar parser MUST apply PAX extension state in header order.
//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# Each present field in a global `g` header replaces the prior global value, an empty global value deletes it, and omitted global fields persist.
//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# Each present field in a local `x` header replaces the pending local value, including empty values, while omitted local fields persist across later extension headers.
//= docs/parsing-requirements.md#parse-tar-006
//= type=test
//# GNU `L`/`K` and PAX `g` headers MUST NOT consume pending local fields.
#[test]
fn pax_global_and_local_extension_timing_survives_interleaved_metadata() {
    let global_before = [
        record(b"path", b"snapshot/global-before"),
        record(b"uid", b"99"),
    ]
    .concat();
    let local = [record(b"path", b"snapshot/local"), record(b"uid", b"")].concat();
    let later_local = record(b"gname", b"local-group");
    let gnu_name = member(b"././@LongLink", b'L', b"snapshot/gnu-name\0");
    let gnu_link = member(b"././@LongLink", b'K', b"target/gnu-link\0");
    let global_after = [
        record(b"path", b"snapshot/global-after"),
        record(b"uid", b""),
    ]
    .concat();
    let first = symlink_with_linkname(b"target/header-link");
    let second = member(b"raw-second", b'0', b"");
    let archive = [
        member(b"global-before", b'g', &global_before),
        member(b"local", b'x', &local),
        member(b"local-later", b'x', &later_local),
        gnu_name,
        gnu_link,
        member(b"global-after", b'g', &global_after),
        first,
        second,
    ]
    .concat();

    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries.len(), 2);
    assert_eq!(parsed.entries[0].name, b"snapshot/local");
    assert_eq!(parsed.entries[0].linkname, b"target/gnu-link");
    assert_eq!(parsed.entries[0].gname, b"local-group");
    assert_eq!(
        parsed.entries[0].uid, 0,
        "empty local value suppresses inherited global uid"
    );
    assert_eq!(parsed.entries[1].name, b"snapshot/global-after");
    assert_eq!(
        parsed.entries[1].uid, 0,
        "empty global update removes the old uid"
    );
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

//= docs/parsing-requirements.md#parse-tar-027
//= type=test
//# After checksum and header-profile admission, the parser MUST admit raw tar `size`, `uid`, `gid`, `device_major`, and `device_minor` fields as nonnegative `u64` values. A negative decoded value or a value greater than `u64::MAX` MUST produce Numeric before payload-extent selection or output-metadata retention. This raw-field admission MUST still occur when an effective PAX `size` supplies a real entry's payload extent; GNU/PAX extension payload extents continue to use their raw header size as specified by PARSE-TAR-007.
#[test]
fn negative_raw_owner_and_device_fields_are_numeric() {
    for range in [108..116, 116..124, 329..337, 337..345] {
        let mut bytes = member(b"snapshot/raw", b'0', b"");
        bytes[range].fill(0xff); // GNU negative base-256 value -1.
        checksum(&mut bytes[..512]);
        assert_eq!(parse(&bytes).unwrap_err(), TarError::Numeric);
    }
}

//= docs/parsing-requirements.md#parse-tar-027
//= type=test
//# After checksum and header-profile admission, the parser MUST admit raw tar `size`, `uid`, `gid`, `device_major`, and `device_minor` fields as nonnegative `u64` values. A negative decoded value or a value greater than `u64::MAX` MUST produce Numeric before payload-extent selection or output-metadata retention. This raw-field admission MUST still occur when an effective PAX `size` supplies a real entry's payload extent; GNU/PAX extension payload extents continue to use their raw header size as specified by PARSE-TAR-007.
#[test]
fn raw_size_is_admitted_even_when_pax_supplies_the_extent() {
    let pax_size = member(b"local", b'x', &record(b"size", b"1"));
    let mut negative_size = member(b"snapshot/raw", b'0', b"x");
    negative_size[124..136].fill(0xff);
    checksum(&mut negative_size[..512]);
    let archive = [pax_size.as_slice(), negative_size.as_slice()].concat();
    assert_eq!(parse(&archive).unwrap_err(), TarError::Numeric);

    let mut oversized_size = member(b"snapshot/raw", b'0', b"x");
    oversized_size[124..136].fill(0xff);
    oversized_size[124] = 0x80; // Positive base-256, larger than u64::MAX.
    checksum(&mut oversized_size[..512]);
    let archive = [pax_size.as_slice(), oversized_size.as_slice()].concat();
    assert_eq!(parse(&archive).unwrap_err(), TarError::Numeric);
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

//= docs/parsing-requirements.md#parse-tar-007
//= type=test
//# GNU and PAX extension headers MUST use the raw size in their own tar header to determine their payload extent.
//= docs/parsing-requirements.md#parse-tar-007
//= type=test
//# A real entry MUST use the effective PAX `size` value when present, or its raw tar header size when absent.
//= docs/parsing-requirements.md#parse-tar-008
//= type=test
//# After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary.
#[test]
fn global_pax_size_does_not_change_extension_payload_extent() {
    let global_size = member(b"global", b'g', &record(b"size", b"1"));
    let local_path = member(b"local", b'x', &record(b"path", b"snapshot/file"));
    let raw_entry = member(b"raw-name", b'0', b"abc");
    let archive = [global_size, local_path, raw_entry].concat();

    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].name, b"snapshot/file");
    assert_eq!(parsed.entries[0].payload, b"a");

    let global_size = member(b"global", b'g', &record(b"size", b"1"));
    let gnu_name = member(b"././@LongLink", b'L', b"snapshot/gnu-long-name\0");
    let raw_entry = member(b"raw-name", b'0', b"abc");
    let archive = [global_size, gnu_name, raw_entry].concat();
    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].name, b"snapshot/gnu-long-name");
    assert_eq!(parsed.entries[0].payload, b"a");
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

//= docs/parsing-requirements.md#parse-tar-015
//= type=test
//# For each accepted tar entry, the output name MUST be the selected PAX path, GNU long-name, or raw ustar prefix/name, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
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
    for value in [b"binary" as &[u8], b"UTF8", b"UTF-8 ", b"ASCII", b"unknown"] {
        assert_eq!(
            parse(&member(b"pax", b'x', &record(b"hdrcharset", value))).unwrap_err(),
            TarError::UnsupportedPax,
            "{value:?}"
        );
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

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

//= docs/parsing-requirements.md#parse-tar-004
//= type=test
//# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.
#[test]
fn hdrcharset_exact_tokens_and_empty_local_deletion_are_enforced() {
    for charset in [b"BINARY" as &[u8], b"UTF-8", b"ISO-IR 10646 2000 UTF-8"] {
        let archive = [
            member(b"local", b'x', &record(b"hdrcharset", charset)),
            member(b"snapshot/file", b'0', b""),
        ]
        .concat();
        assert_eq!(parse(&archive).unwrap().entries.len(), 1, "{charset:?}");
    }

    let global = member(b"global", b'g', &record(b"hdrcharset", b"BINARY"));
    let local = [record(b"hdrcharset", b""), record(b"uname", b"user-\xff")].concat();
    let archive = [
        global,
        member(b"local", b'x', &local),
        member(b"snapshot/file", b'0', b""),
    ]
    .concat();
    // Empty local hdrcharset deletes the inherited BINARY value, so PAX uname
    // returns to the default UTF-8 validation policy.
    assert_eq!(parse(&archive).unwrap_err(), TarError::TextEncoding);
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn global_and_local_pax_resolution_composes_all_supported_fields() {
    let global = [
        record(b"path", b"snapshot/global"),
        record(b"linkpath", b"global-link"),
        record(b"size", b"5"),
        record(b"mtime", b"2"),
        record(b"uid", b"9"),
        record(b"gid", b"10"),
        record(b"uname", b"global-user"),
        record(b"gname", b"global-group"),
        record(b"atime", b"9"),
        record(b"ctime", b"10"),
        record(b"hdrcharset", b"UTF-8"),
    ]
    .concat();
    let local = [
        record(b"path", b"snapshot/local"),
        record(b"linkpath", b"local-link"),
        record(b"size", b""),
        record(b"mtime", b""),
        record(b"uid", b""),
        record(b"gid", b"42"),
        record(b"uname", b""),
        record(b"atime", b""),
        record(b"ctime", b"11.5"),
        record(b"hdrcharset", b""),
    ]
    .concat();
    let archive = [
        member(b"global", b'g', &global),
        member(b"local", b'x', &local),
        member(b"snapshot/header", b'0', b"abc"),
    ]
    .concat();
    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    let entry = &parsed.entries[0];
    assert_eq!(entry.name, b"snapshot/local");
    assert_eq!(entry.linkname, b"local-link");
    assert_eq!(entry.payload, b"abc");
    assert_eq!(
        entry.mtime,
        TarTime {
            seconds: 1,
            nanoseconds: 0
        }
    );
    assert_eq!(entry.uid, 0);
    assert_eq!(entry.gid, 42);
    assert!(entry.uname.is_empty());
    assert_eq!(entry.gname, b"global-group");
    assert_eq!(entry.atime, None);
    assert_eq!(
        entry.ctime,
        Some(TarTime {
            seconds: 11,
            nanoseconds: 500_000_000
        })
    );
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# For a successful PAX extended-header stream, the reader MUST process records in input order and accumulate each supported, unique, NUL-free key/value pair into its assigned one of the eleven metadata slots.
#[test]
fn successful_pax_stream_accumulates_all_eleven_fields_into_native_metadata() {
    let pax_stream = [
        record(b"path", b"snapshot/accumulated"),
        record(b"linkpath", b""),
        record(b"size", b"3"),
        record(b"mtime", b"13.25"),
        record(b"uid", b"11"),
        record(b"gid", b"12"),
        record(b"uname", b"user"),
        record(b"gname", b"group"),
        record(b"atime", b"14"),
        record(b"ctime", b"15"),
        record(b"hdrcharset", b"UTF-8"),
    ]
    .concat();
    let global = member(b"global", b'g', &record(b"linkpath", b"global-target"));
    let mut raw_entry = member(b"snapshot/header", b'0', b"abc");
    raw_entry[157..157 + b"header-target".len()].copy_from_slice(b"header-target");
    checksum(&mut raw_entry[..512]);
    let archive = [global, member(b"local", b'x', &pax_stream), raw_entry].concat();

    let parsed = parse(&archive).expect("valid multi-record PAX stream");
    assert_eq!(parsed.entries.len(), 1);
    let entry = &parsed.entries[0];
    assert_eq!(entry.name, b"snapshot/accumulated");
    assert_eq!(entry.linkname, b"header-target");
    assert_eq!(entry.payload, b"abc");
    assert_eq!(entry.uid, 11);
    assert_eq!(entry.gid, 12);
    assert_eq!(entry.uname, b"user");
    assert_eq!(entry.gname, b"group");
    assert_eq!(
        entry.mtime,
        TarTime {
            seconds: 13,
            nanoseconds: 250_000_000
        }
    );
    assert_eq!(
        entry.atime,
        Some(TarTime {
            seconds: 14,
            nanoseconds: 0
        })
    );
    assert_eq!(
        entry.ctime,
        Some(TarTime {
            seconds: 15,
            nanoseconds: 0
        })
    );
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# For each PAX extended-header stream, the reader MUST process records in input order and report the first failing record's error class using framing before key support, key support before duplicate/NUL admission, and admission before value syntax: malformed framing or duplicate/NUL values for a supported key classify as Pax, unsupported keys (including those with NUL values) and unsupported nonempty hdrcharset values as UnsupportedPax, and invalid decimal/time values as Numeric; empty values are accepted, and no later record changes the first error.
#[test]
fn pax_stream_reports_the_first_error_by_record_and_stage() {
    let cases: [(Vec<u8>, TarError); 9] = [
        (
            [record(b"size", b"bad"), record(b"unknown", b"later")].concat(),
            TarError::Numeric,
        ),
        (
            [
                record(b"mtime", b"not-a-time"),
                record(b"unknown", b"later"),
            ]
            .concat(),
            TarError::Numeric,
        ),
        (
            [record(b"unknown", b"value"), b"bad frame".to_vec()].concat(),
            TarError::UnsupportedPax,
        ),
        (
            [b"bad frame".to_vec(), record(b"unknown", b"later")].concat(),
            TarError::Pax,
        ),
        (
            [record(b"size", b"1"), record(b"size", b"bad")].concat(),
            TarError::Pax,
        ),
        (
            [record(b"size", b"1\0"), record(b"path", b"later")].concat(),
            TarError::Pax,
        ),
        (
            [record(b"hdrcharset", b"unknown"), b"bad frame".to_vec()].concat(),
            TarError::UnsupportedPax,
        ),
        (
            [record(b"size", b"1"), b"bad frame".to_vec()].concat(),
            TarError::Pax,
        ),
        (record(b"unknown", b"bad\0value"), TarError::UnsupportedPax),
    ];

    for (stream, expected) in cases {
        let archive = member(b"local", b'x', &stream);
        assert_eq!(parse(&archive).unwrap_err(), expected, "{stream:?}");
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_record_admission_preserves_duplicate_nul_unknown_and_empty_value_behavior() {
    let duplicate = [
        record(b"path", b"snapshot/first"),
        record(b"path", b"snapshot/second"),
    ]
    .concat();
    assert_eq!(
        parse(&member(b"pax", b'x', &duplicate)).unwrap_err(),
        TarError::Pax
    );

    let nul_value = member(b"pax", b'x', &record(b"path", b"snapshot/\0bad"));
    assert_eq!(parse(&nul_value).unwrap_err(), TarError::Pax);

    let unknown_key = member(b"pax", b'x', &record(b"unknown.key", b"value"));
    assert_eq!(parse(&unknown_key).unwrap_err(), TarError::UnsupportedPax);

    let global = member(b"global", b'g', &record(b"path", b"snapshot/global"));
    let local_delete = member(b"local", b'x', &record(b"path", b""));
    let bytes = [
        global,
        local_delete,
        member(b"snapshot/header", b'0', b""),
        member(b"raw", b'0', b""),
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].name, b"snapshot/header");
    assert_eq!(parsed.entries[1].name, b"snapshot/global");
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_owner_names_fallback_override_delete_and_resume_global_values() {
    let global = [
        record(b"uname", b"global-user"),
        record(b"gname", b"global-group"),
    ]
    .concat();
    let local = [
        record(b"uname", b"local-user"),
        record(b"gname", b"local-group"),
    ]
    .concat();
    let deletion = [record(b"uname", b""), record(b"gname", b"")].concat();
    let clear_global = [record(b"uname", b""), record(b"gname", b"")].concat();
    let header = member_with_owner_names(b"header-user", b"header-group");
    let bytes = [
        member(b"global", b'g', &global),
        member(b"local", b'x', &local),
        header.clone(),
        member(b"local", b'x', &deletion),
        header.clone(),
        header.clone(),
        member(b"global", b'g', &clear_global),
        header,
    ]
    .concat();
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries[0].uname, b"local-user");
    assert_eq!(parsed.entries[0].gname, b"local-group");
    assert_eq!(parsed.entries[1].uname, b"header-user");
    assert_eq!(parsed.entries[1].gname, b"header-group");
    assert_eq!(parsed.entries[2].uname, b"global-user");
    assert_eq!(parsed.entries[2].gname, b"global-group");
    assert_eq!(parsed.entries[3].uname, b"header-user");
    assert_eq!(parsed.entries[3].gname, b"header-group");
}

//= docs/parsing-requirements.md#parse-tar-003
//= type=test
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
#[test]
fn pax_owner_names_follow_utf8_and_binary_charset_policy() {
    let valid_utf8 = [
        record(b"uname", "José".as_bytes()),
        record(b"gname", "Grüppe".as_bytes()),
    ]
    .concat();
    let archive = [
        member(b"local", b'x', &valid_utf8),
        member_with_owner_names(b"header-user", b"header-group"),
    ]
    .concat();
    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries[0].uname, "José".as_bytes());
    assert_eq!(parsed.entries[0].gname, "Grüppe".as_bytes());

    let invalid_utf8 = [
        record(b"uname", b"user-\xff"),
        record(b"gname", b"group-\xff"),
    ]
    .concat();
    let archive = [
        member(b"local", b'x', &invalid_utf8),
        member(b"snapshot/file", b'0', b""),
    ]
    .concat();
    assert_eq!(parse(&archive).unwrap_err(), TarError::TextEncoding);

    let binary = [
        record(b"hdrcharset", b"BINARY"),
        record(b"uname", b"user-\xff"),
        record(b"gname", b"group-\xff"),
    ]
    .concat();
    let archive = [
        member(b"local", b'x', &binary),
        member(b"snapshot/file", b'0', b""),
    ]
    .concat();
    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries[0].uname, b"user-\xff");
    assert_eq!(parsed.entries[0].gname, b"group-\xff");

    // Existing header fallback remains byte-preserving and is not PAX-validated.
    let archive = member_with_owner_names(b"header-\xff", b"group-\xff");
    let parsed = parse(&archive).unwrap();
    assert_eq!(parsed.entries[0].uname, b"header-\xff");
    assert_eq!(parsed.entries[0].gname, b"group-\xff");
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
//= docs/parsing-requirements.md#parse-tar-008
//= type=test
//# It MUST reject configured entry-size excess and truncated extents with explicit errors.

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
//= docs/parsing-requirements.md#parse-tar-011
//= type=test
//# Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.
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
//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling.
//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# A real entry clears pending local PAX or GNU long-name/link override state only after the entry is accepted.
//= docs/parsing-requirements.md#parse-tar-005
//= type=test
//# A dangling local PAX or GNU long-name/link override MUST produce `DanglingOverride` at EOF.
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
    let pending_then_global = [
        member(b"local", b'x', &record(b"uid", b"1")),
        member(b"global", b'g', &record(b"uid", b"2")),
    ]
    .concat();
    assert_eq!(
        parse(&pending_then_global).unwrap_err(),
        TarError::DanglingOverride
    );
    let accepted_entry = [
        member(b"local", b'x', &record(b"uid", b"1")),
        member(b"snapshot/file", b'0', b""),
    ]
    .concat();
    assert_eq!(parse(&accepted_entry).unwrap().entries.len(), 1);
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

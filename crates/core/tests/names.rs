use duplicity_core::names::{
    parse_long_timestamp, parse_name, Envelope, NameConfig, NameError, ObjectKind, Timestamp,
};

//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.

//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.

#[test]
fn gregorian_dates_and_zones_are_exact() {
    let parse = |s: &[u8]| parse_long_timestamp(s, b':');
    assert_eq!(parse(b"19700101T000000Z"), Ok(Timestamp(0)));
    assert_eq!(parse(b"19010101T000000Z"), Ok(Timestamp(-2_177_452_800)));
    assert_eq!(parse(b"20231114t221320z"), Ok(Timestamp(1_700_000_000)));
    assert_eq!(parse(b"1970-01-01T01:30:00+01:30"), Ok(Timestamp(0)));
    assert_eq!(parse(b"1969-12-31T22:30:00-01:30"), Ok(Timestamp(0)));
    assert!(parse(b"20000229T235959Z").is_ok());
    for bad in [
        b"19000229T000000Z" as &[u8],
        b"21000101T000000Z",
        b"19001231T235959Z",
        b"20010229T000000Z",
        b"20230431T000000Z",
        b"20230001T000000Z",
        b"20230100T000000Z",
        b"20230101T240000Z",
        b"20230101T000060Z",
        b"1970-01-01T00:00:00+24:00",
        b"1970-01-01T00:00:00+00:60",
        b"1970-01-01T00:00:00",
        b"19700101T000000Zx",
    ] {
        assert_eq!(parse(bad), Err(NameError::Malformed), "{bad:?}");
    }
    for sep in 0..=255u8 {
        let valid = (0x21..=0x7e).contains(&sep)
            && !sep.is_ascii_digit()
            && !matches!(sep, b'/' | b'.' | b'+' | b'-' | b'T' | b't' | b'Z' | b'z');
        assert_eq!(
            parse_long_timestamp(b"19700101T000000Z", sep).is_ok(),
            valid
        );
    }
    assert_eq!(
        parse_long_timestamp(b"1970-01-01T00_00_00Z", b'_'),
        Ok(Timestamp(0))
    );
}

//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.

#[test]
fn month_lengths_follow_gregorian_leap_year_rules() {
    let common_year = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap_year = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (year, lengths) in [(2001, common_year), (2000, leap_year)] {
        for (index, last_day) in lengths.into_iter().enumerate() {
            let month = index + 1;
            let valid = format!("{year:04}{month:02}{last_day:02}T000000Z");
            let invalid = format!("{year:04}{month:02}{:02}T000000Z", last_day + 1);
            assert!(parse_long_timestamp(valid.as_bytes(), b':').is_ok());
            assert_eq!(
                parse_long_timestamp(invalid.as_bytes(), b':'),
                Err(NameError::Malformed),
                "year {year}, month {month}"
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.

#[test]
fn explicit_zone_offset_boundaries_are_exact() {
    let parse = |s: &[u8]| parse_long_timestamp(s, b':');
    for valid in [
        b"1970-01-01T23:59:00+23:59" as &[u8],
        b"1969-12-31T00:01:00-23:59",
        b"1970-01-01T00:00:00+00:00",
        b"1970-01-01T00:00:00-00:00",
    ] {
        assert_eq!(parse(valid), Ok(Timestamp(0)), "{valid:?}");
    }
    for invalid in [
        b"1970-01-01T00:00:00+24:00" as &[u8],
        b"1970-01-01T00:00:00-24:00",
        b"1970-01-01T00:00:00+00:60",
        b"1970-01-01T00:00:00-00:60",
    ] {
        assert_eq!(parse(invalid), Err(NameError::Malformed), "{invalid:?}");
    }
}

//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.

#[test]
fn explicit_zone_offset_uses_the_configured_separator() {
    assert_eq!(
        parse_long_timestamp(b"1970-01-01T00_00_00+00_00", b'_'),
        Ok(Timestamp(0))
    );
    assert_eq!(
        parse_long_timestamp(b"1970-01-01T00_00_00+00:00", b'_'),
        Err(NameError::Malformed)
    );
}

//= docs/parsing-requirements.md#parse-name-002
//= type=test
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.

#[test]
fn exact_names_suffixes_prefixes_and_limits() {
    let config = NameConfig::default();
    let base = b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar";
    let name = parse_name(base, config).unwrap().unwrap();
    assert_eq!(name.kind, ObjectKind::IncrementalVolume);
    assert_eq!(name.volume, Some(1));
    assert!(name.completed());
    for (suffix, envelope, partial) in [
        (".part", Envelope::Plain, true),
        (".part.gpg", Envelope::OpenPgp, true),
        (".gz", Envelope::Gzip, false),
        (".p7m", Envelope::Pkcs7, false),
    ] {
        let bytes = [base, suffix.as_bytes()].concat();
        let name = parse_name(&bytes, config).unwrap().unwrap();
        assert_eq!((name.envelope, name.partial), (envelope, partial));
        assert_eq!(name.completed(), !partial && envelope != Envelope::Pkcs7);
    }
    for bad in [".gz.extra", ".", ".p", ".g", ".part.part", ".gz.part"] {
        assert_eq!(
            parse_name(&[base, bad.as_bytes()].concat(), config),
            Err(NameError::Malformed)
        );
    }
    let custom = NameConfig {
        global_prefix: b"[g]",
        archive_prefix: b"a+",
        ..config
    };
    assert!(parse_name(&[b"[g]a+" as &[u8], base].concat(), custom)
        .unwrap()
        .is_some());
    assert_eq!(
        parse_name(&[b"ga+" as &[u8], base].concat(), custom),
        Ok(None)
    );
    assert_eq!(
        parse_name(
            base,
            NameConfig {
                max_name_bytes: base.len() - 1,
                ..config
            }
        ),
        Err(NameError::Limit)
    );
    assert!(parse_name(
        base,
        NameConfig {
            max_name_bytes: base.len(),
            ..config
        }
    )
    .is_ok());
    assert_eq!(parse_name(b"random-object", config), Ok(None));
    assert_eq!(
        parse_name(b"prefix/file", config),
        Err(NameError::Malformed)
    );
    assert_eq!(
        parse_name(
            base,
            NameConfig {
                global_prefix: b"bad/",
                ..config
            }
        ),
        Err(NameError::Configuration)
    );
}

//= docs/parsing-requirements.md#parse-name-002
//= type=test
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
#[test]
fn configured_name_prefixes_are_consumed_global_then_kind_specific() {
    let base = b"duplicity-full.20231114T221320Z.vol1.difftar";
    let config = NameConfig {
        global_prefix: b"a",
        archive_prefix: b"ab",
        ..NameConfig::default()
    };
    let parsed = parse_name(&[b"aab" as &[u8], base].concat(), config)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.kind, ObjectKind::FullVolume);
    assert_eq!(parsed.volume, Some(1));
    assert_eq!(
        parse_name(&[b"ab" as &[u8], base].concat(), config),
        Ok(None)
    );
    assert_eq!(
        parse_name(&[b"aa" as &[u8], base].concat(), config),
        Ok(None)
    );

    let metacharacters = NameConfig {
        global_prefix: b"[g]",
        archive_prefix: b"a+",
        ..NameConfig::default()
    };
    assert!(
        parse_name(&[b"[g]a+" as &[u8], base].concat(), metacharacters)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        parse_name(&[b"ga+" as &[u8], base].concat(), metacharacters),
        Ok(None)
    );
}

//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.

//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.

#[test]
fn base36_overflow_and_increment_order() {
    let config = NameConfig::default();
    assert_eq!(
        parse_name(b"df.3w5e11264sgsf.1.dt.z", config)
            .unwrap()
            .unwrap()
            .start,
        Timestamp(i128::from(u64::MAX))
    );
    for bad in [
        b"df.3w5e11264sgsg.1.dt" as &[u8],
        b"df..1.dt",
        b"df.A.1.dt",
        b"df.1.0.dt",
        b"df.1.3w5e11264sgsg.dt",
        b"di.2.2.m",
        b"di.2.1.m",
        b"df.1.1.dt.gz",
        b"duplicity-full.20231114T221320Z.vol18446744073709551616.difftar",
    ] {
        assert_eq!(
            parse_name(bad, config),
            Err(NameError::Malformed),
            "{bad:?}"
        );
    }
    let partial = parse_name(b"di.1.2.a.dt.p.g", config).unwrap().unwrap();
    assert_eq!(partial.volume, Some(10));
    assert!(partial.partial);
    assert_eq!(partial.envelope, Envelope::OpenPgp);
}

//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
#[test]
fn short_name_timestamps_are_exact_epoch_seconds() {
    let config = NameConfig::default();
    for (name, expected) in [
        (b"df.0.1.dt" as &[u8], 0),
        (b"df.1.1.dt", 1),
        (b"df.z.1.dt", 35),
        (b"df.10.1.dt", 36),
        (b"df.3w5e11264sgsf.1.dt", u64::MAX),
    ] {
        assert_eq!(
            parse_name(name, config).unwrap().unwrap().start,
            Timestamp(expected as i128),
            "{name:?}"
        );
    }
    for malformed in [b"df..1.dt" as &[u8], b"df.A.1.dt", b"df.3w5e11264sgsg.1.dt"] {
        assert_eq!(parse_name(malformed, config), Err(NameError::Malformed));
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
#[test]
fn short_volume_numbers_are_positive_base36_and_long_volumes_remain_decimal() {
    let config = NameConfig::default();
    for (name, volume) in [
        (b"df.1.1.dt" as &[u8], 1),
        (b"df.1.z.dt", 35),
        (b"df.1.10.dt", 36),
    ] {
        assert_eq!(
            parse_name(name, config).unwrap().unwrap().volume,
            Some(volume)
        );
    }
    for malformed in [b"df.1.0.dt" as &[u8], b"df.1.00.dt", b"df.1.A.dt"] {
        assert_eq!(parse_name(malformed, config), Err(NameError::Malformed));
    }
    assert_eq!(
        parse_name(b"duplicity-full.20231114T221320Z.vol10.difftar", config)
            .unwrap()
            .unwrap()
            .volume,
        Some(10)
    );
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-gen-003
//= type=test
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
#[test]
fn long_volume_fields_require_positive_decimal_after_exact_vol_prefix() {
    let config = NameConfig::default();
    for (field, expected) in [(b"vol1" as &[u8], 1), (b"vol01", 1), (b"vol10", 10)] {
        let mut name = b"duplicity-full.20231114T221320Z.".to_vec();
        name.extend_from_slice(field);
        name.extend_from_slice(b".difftar");
        assert_eq!(
            parse_name(&name, config).unwrap().unwrap().volume,
            Some(expected)
        );
    }
    for field in [
        b"vol" as &[u8],
        b"vol0",
        b"vol00",
        b"Vol1",
        b"foo1",
        b"vol1a",
    ] {
        let mut name = b"duplicity-full.20231114T221320Z.".to_vec();
        name.extend_from_slice(field);
        name.extend_from_slice(b".difftar");
        assert_eq!(
            parse_name(&name, config),
            Err(NameError::Malformed),
            "{field:?}"
        );
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
#[test]
fn short_name_intervals_follow_full_and_incremental_kind_policy() {
    let config = NameConfig::default();
    for (name, kind) in [
        (b"df.1.1.dt" as &[u8], ObjectKind::FullVolume),
        (b"df.1.m", ObjectKind::FullManifest),
        (b"dfs.1.st", ObjectKind::FullSignatures),
        (b"di.1.2.1.dt", ObjectKind::IncrementalVolume),
        (b"di.1.2.m", ObjectKind::IncrementalManifest),
        (b"dns.1.2.st", ObjectKind::IncrementalSignatures),
    ] {
        let parsed = parse_name(name, config).unwrap().unwrap();
        assert_eq!(parsed.kind, kind, "{name:?}");
        assert_eq!(parsed.start, Timestamp(1), "{name:?}");
        assert_eq!(
            parsed.end,
            if matches!(
                kind,
                ObjectKind::IncrementalVolume
                    | ObjectKind::IncrementalManifest
                    | ObjectKind::IncrementalSignatures
            ) {
                Timestamp(2)
            } else {
                Timestamp(1)
            },
            "{name:?}"
        );
    }
    for malformed in [
        b"di.1.1.1.dt" as &[u8],
        b"di.2.1.1.dt",
        b"di.3w5e11264sgsf.3w5e11264sgsf.1.dt",
        b"di.1.3w5e11264sgsg.m",
        b"dns.2.1.st",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }
    let max = parse_name(b"df.3w5e11264sgsf.m", config).unwrap().unwrap();
    assert_eq!(max.start, Timestamp(u64::MAX as i128));
    assert_eq!(max.end, max.start);
    let max_interval = parse_name(b"di.3w5e11264sgse.3w5e11264sgsf.m", config)
        .unwrap()
        .unwrap();
    assert_eq!(max_interval.end, Timestamp(u64::MAX as i128));
}

//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
#[test]
fn long_name_intervals_follow_all_kind_and_timestamp_profiles() {
    let config = NameConfig::default();
    for (name, kind, start, end) in [
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar" as &[u8],
            ObjectKind::FullVolume,
            1_700_000_000,
            1_700_000_000,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar",
            ObjectKind::IncrementalVolume,
            1_700_000_000,
            1_700_086_400,
        ),
        (
            b"duplicity-full.2023-11-14T22:13:20Z.manifest",
            ObjectKind::FullManifest,
            1_700_000_000,
            1_700_000_000,
        ),
        (
            b"duplicity-inc.2023-11-14T22:13:20Z.to.2023-11-15T22:13:20Z.manifest",
            ObjectKind::IncrementalManifest,
            1_700_000_000,
            1_700_086_400,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar",
            ObjectKind::FullSignatures,
            1_700_000_000,
            1_700_000_000,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar",
            ObjectKind::IncrementalSignatures,
            1_700_000_000,
            1_700_086_400,
        ),
    ] {
        let parsed = parse_name(name, config).unwrap().unwrap();
        assert_eq!(parsed.kind, kind, "{name:?}");
        assert_eq!(parsed.start, Timestamp(start), "{name:?}");
        assert_eq!(parsed.end, Timestamp(end), "{name:?}");
    }

    for malformed in [
        b"duplicity-inc.20231114T221320Z.20231115T221320Z.vol1.difftar" as &[u8],
        b"duplicity-inc.20231114T221320Z.from.20231115T221320Z.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.to.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231114T221320Z.vol1.difftar",
        b"duplicity-inc.20231115T221320Z.to.20231114T221320Z.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.to.20230230T221320Z.vol1.difftar",
        b"duplicity-new-signatures.20231114T221320Z.from.20231115T221320Z.sigtar",
        b"duplicity-inc.2023-11-14T22:13:20Z.to.2023-11-13T22:13:20Z.manifest",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let custom_separator = NameConfig {
        time_separator: b'_',
        ..NameConfig::default()
    };
    let custom_legacy = parse_name(
        b"duplicity-inc.2023-11-14T22_13_20+00_00.to.2023-11-15T22_13_20Z.manifest",
        custom_separator,
    )
    .unwrap()
    .unwrap();
    assert_eq!(custom_legacy.kind, ObjectKind::IncrementalManifest);
    assert_eq!(custom_legacy.start, Timestamp(1_700_000_000));
    assert_eq!(custom_legacy.end, Timestamp(1_700_086_400));

    let earliest = parse_name(
        b"duplicity-inc.19010101T000000Z.to.19010102T000000Z.manifest",
        config,
    )
    .unwrap()
    .unwrap();
    assert_eq!(earliest.start, Timestamp(-2_177_452_800));
    assert_eq!(earliest.end, Timestamp(-2_177_366_400));
    let latest = parse_name(
        b"duplicity-inc.20991230T000000Z.to.20991231T000000Z.manifest",
        config,
    )
    .unwrap()
    .unwrap();
    assert_eq!(latest.start, Timestamp(4_102_272_000));
    assert_eq!(latest.end, Timestamp(4_102_358_400));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[test]
fn short_trailer_suffixes_classify_kind_envelope_and_partial_status() {
    let config = NameConfig::default();
    for (name, kind, envelope, partial) in [
        (
            b"df.1.1.dt" as &[u8],
            ObjectKind::FullVolume,
            Envelope::Plain,
            false,
        ),
        (
            b"df.1.1.dt.p",
            ObjectKind::FullVolume,
            Envelope::Plain,
            true,
        ),
        (
            b"df.1.1.dt.z",
            ObjectKind::FullVolume,
            Envelope::Gzip,
            false,
        ),
        (
            b"df.1.1.dt.p.z",
            ObjectKind::FullVolume,
            Envelope::Gzip,
            true,
        ),
        (
            b"di.1.2.1.dt.g",
            ObjectKind::IncrementalVolume,
            Envelope::OpenPgp,
            false,
        ),
        (
            b"di.1.2.1.dt.p.g",
            ObjectKind::IncrementalVolume,
            Envelope::OpenPgp,
            true,
        ),
        (b"df.1.m", ObjectKind::FullManifest, Envelope::Plain, false),
        (
            b"di.1.2.m.p",
            ObjectKind::IncrementalManifest,
            Envelope::Plain,
            true,
        ),
        (
            b"dfs.1.st.z",
            ObjectKind::FullSignatures,
            Envelope::Gzip,
            false,
        ),
        (
            b"dns.1.2.st.p.g",
            ObjectKind::IncrementalSignatures,
            Envelope::OpenPgp,
            true,
        ),
    ] {
        let parsed = parse_name(name, config).unwrap().unwrap();
        assert_eq!(
            (parsed.kind, parsed.envelope, parsed.partial),
            (kind, envelope, partial),
            "{name:?}"
        );
    }
    for malformed in [
        b"df.1.1.m" as &[u8],
        b"df.1.1.dt.",
        b"df.1.1.dt.p.",
        b"df.1.1.dt.x",
        b"df.1.1.dt.z.p",
        b"df.1.1.dt.g.p",
        b"df.1.1.dt.g.z",
        b"df.1.1.dt.z.g",
        b"df.1.1.dt.p.p",
        b"df.1.1.dt.x.p",
        b"df.1.1.dt.p.z.extra",
        b"df.1.1.dt.p.g.z",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.

#[test]
fn every_official_archive_basename_is_classified() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut pending = vec![root];
    let mut count = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
                continue;
            }
            let name = entry.file_name();
            let bytes = name.as_encoded_bytes();
            if bytes.starts_with(b"duplicity-")
                || bytes.starts_with(b"df.")
                || bytes.starts_with(b"di.")
                || bytes.starts_with(b"dfs.")
                || bytes.starts_with(b"dns.")
            {
                let separator = if bytes.contains(&b'_') { b'_' } else { b':' };
                let parsed = parse_name(
                    bytes,
                    NameConfig {
                        time_separator: separator,
                        ..NameConfig::default()
                    },
                )
                .unwrap_or_else(|error| panic!("{name:?}: {error:?}"))
                .unwrap();
                assert!(parsed.completed());
                assert!(parsed.end >= parsed.start);
                count += 1;
            }
        }
    }
    assert_eq!(count, 96, "producer artifact coverage changed");
}

#[test]
fn every_supported_calendar_day_is_contiguous() {
    let mut expected = -2_177_452_800i128;
    for year in 1901..=2099 {
        let mut lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        // Within this profile every multiple of four is a leap year: 2000 is
        // divisible by 400, and the non-leap centuries are outside the bounds.
        if year % 4 == 0 {
            lengths[1] = 29;
        }
        for (index, length) in lengths.into_iter().enumerate() {
            for day in 1..=length {
                let text = format!("{year:04}{:02}{day:02}T000000Z", index + 1);
                assert_eq!(
                    parse_long_timestamp(text.as_bytes(), b':'),
                    Ok(Timestamp(expected))
                );
                expected += 86400;
            }
        }
    }
    assert_eq!(expected, 4_102_444_800); // 2100-01-01 UTC, outside accepted dates.
}

#[test]
fn truncated_and_byte_invalid_framing_is_rejected() {
    for date in [b"20231114T221320Z" as &[u8], b"2023-11-14T22:13:20+05:30"] {
        for size in 0..date.len() {
            assert_eq!(
                parse_long_timestamp(&date[..size], b':'),
                Err(NameError::Malformed)
            );
        }
    }
    let name = b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gpg";
    for size in 0..=name.len() {
        let _ = parse_name(&name[..size], NameConfig::default());
    }
    for byte in 0..=255u8 {
        let mut date = *b"20231114T221320Z";
        date[0] = byte;
        if !byte.is_ascii_digit() {
            assert_eq!(parse_long_timestamp(&date, b':'), Err(NameError::Malformed));
        }
    }
}

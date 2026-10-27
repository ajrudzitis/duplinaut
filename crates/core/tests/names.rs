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
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem. Volume kinds MUST select the archive prefix, manifest kinds MUST select the manifest prefix, and signature kinds MUST select the signature prefix.

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

//= docs/parsing-requirements.md#parse-name-008
//= type=test
//# For a valid parser configuration, the reader MUST return the name-size limit error when a basename exceeds its configured byte limit, before checking basename contents. At or below that limit, the reader MUST reject a basename containing NUL or `/` with the malformed-name error.
#[test]
fn parse_name_preflight_checks_size_before_basename_bytes() {
    let config = NameConfig::default();

    assert_eq!(
        parse_name(
            b"",
            NameConfig {
                max_name_bytes: 0,
                ..config
            }
        ),
        Ok(None)
    );
    for invalid_basename in [b"/" as &[u8], b"a\0b"] {
        assert_eq!(
            parse_name(
                invalid_basename,
                NameConfig {
                    max_name_bytes: 0,
                    ..config
                },
            ),
            Err(NameError::Limit),
            "size limit takes precedence for {invalid_basename:?}"
        );
        assert_eq!(
            parse_name(
                invalid_basename,
                NameConfig {
                    max_name_bytes: invalid_basename.len(),
                    ..config
                },
            ),
            Err(NameError::Malformed),
            "invalid basename byte is rejected at the exact limit: {invalid_basename:?}"
        );
        assert_eq!(
            parse_name(
                invalid_basename,
                NameConfig {
                    max_name_bytes: invalid_basename.len() + 1,
                    ..config
                },
            ),
            Err(NameError::Malformed),
            "invalid basename byte is rejected below the limit: {invalid_basename:?}"
        );
    }
    assert_eq!(
        parse_name(
            b"opaque",
            NameConfig {
                max_name_bytes: 6,
                ..config
            }
        ),
        Ok(None)
    );
    assert_eq!(
        parse_name(
            b"opaque",
            NameConfig {
                max_name_bytes: 5,
                ..config
            }
        ),
        Err(NameError::Limit)
    );
}

//= docs/parsing-requirements.md#parse-name-009
//= type=test
//# Before checking basename size or bytes, the reader MUST validate the parser configuration. The time separator MUST be in the valid set in PARSE-NAME-007. Configured global, archive, manifest, and signature prefixes MAY be empty and otherwise contain arbitrary bytes, but MUST NOT contain NUL or `/`. Invalid configuration MUST return the configuration error before basename limit or malformed-name errors; valid configuration MUST proceed to the checks in PARSE-NAME-008.
#[test]
fn parse_name_rejects_invalid_configuration_before_basename_errors() {
    let base = NameConfig::default();
    let inputs: [&[u8]; 3] = [b"/", b"a\0b", b"oversized"];

    let invalid_separator = NameConfig {
        time_separator: 0,
        max_name_bytes: 0,
        ..base
    };
    for input in inputs {
        assert_eq!(
            parse_name(input, invalid_separator),
            Err(NameError::Configuration),
            "invalid separator takes precedence for {input:?}"
        );
    }

    for invalid_prefix in [b"/" as &[u8], b"x\0y"] {
        for field in 0..4 {
            let config = match field {
                0 => NameConfig {
                    global_prefix: invalid_prefix,
                    max_name_bytes: 0,
                    ..base
                },
                1 => NameConfig {
                    archive_prefix: invalid_prefix,
                    max_name_bytes: 0,
                    ..base
                },
                2 => NameConfig {
                    manifest_prefix: invalid_prefix,
                    max_name_bytes: 0,
                    ..base
                },
                _ => NameConfig {
                    signature_prefix: invalid_prefix,
                    max_name_bytes: 0,
                    ..base
                },
            };
            for input in inputs {
                assert_eq!(
                    parse_name(input, config),
                    Err(NameError::Configuration),
                    "invalid prefix {field} takes precedence for {input:?}"
                );
            }
        }
    }

    let arbitrary_prefix_byte = NameConfig {
        global_prefix: b"\xff",
        ..base
    };
    assert_eq!(parse_name(b"unrelated", arbitrary_prefix_byte), Ok(None));
}

//= docs/parsing-requirements.md#parse-name-002
//= type=test
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem. Volume kinds MUST select the archive prefix, manifest kinds MUST select the manifest prefix, and signature kinds MUST select the signature prefix.
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

//= docs/parsing-requirements.md#parse-name-002
//= type=test
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem. Volume kinds MUST select the archive prefix, manifest kinds MUST select the manifest prefix, and signature kinds MUST select the signature prefix.
#[test]
fn each_object_kind_uses_its_configured_prefix_family() {
    let config = NameConfig {
        global_prefix: b"\x80",
        archive_prefix: b"archive-",
        manifest_prefix: b"manifest-",
        signature_prefix: b"\xfe",
        ..NameConfig::default()
    };
    let examples: &[(&[u8], &[u8], ObjectKind)] = &[
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar",
            config.archive_prefix,
            ObjectKind::FullVolume,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar",
            config.archive_prefix,
            ObjectKind::IncrementalVolume,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest",
            config.manifest_prefix,
            ObjectKind::FullManifest,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
            config.manifest_prefix,
            ObjectKind::IncrementalManifest,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar",
            config.signature_prefix,
            ObjectKind::FullSignatures,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar",
            config.signature_prefix,
            ObjectKind::IncrementalSignatures,
        ),
    ];

    for (stemmed, selected_prefix, expected_kind) in examples {
        let input = [config.global_prefix, *selected_prefix, *stemmed].concat();
        assert_eq!(
            parse_name(&input, config).unwrap().unwrap().kind,
            *expected_kind
        );

        for wrong_prefix in [
            config.archive_prefix,
            config.manifest_prefix,
            config.signature_prefix,
        ] {
            if wrong_prefix != *selected_prefix {
                let wrong = [config.global_prefix, wrong_prefix, *stemmed].concat();
                assert_ne!(
                    parse_name(&wrong, config)
                        .ok()
                        .flatten()
                        .map(|name| name.kind),
                    Some(*expected_kind)
                );
            }
        }
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[test]
fn near_miss_object_stems_remain_unrecognized() {
    let config = NameConfig::default();
    for near_miss in [
        b"duplicity-fullx.20231114T221320Z.vol1.difftar" as &[u8],
        b"duplicity-incx.20231114T221320Z.to.20231115T221320Z.vol1.difftar",
        b"duplicity-full-signaturesx.20231114T221320Z.sigtar",
        b"dfx.1.1.dt",
        b"dix.1.2.1.dt",
        b"dfsx.1.st",
        b"dnsx.1.2.st",
    ] {
        assert_eq!(parse_name(near_miss, config), Ok(None), "{near_miss:?}");
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[test]
fn parse_name_finalizes_unrelated_malformed_and_valid_candidates() {
    let config = NameConfig::default();
    assert_eq!(parse_name(b"unrelated object", config), Ok(None));
    assert_eq!(
        parse_name(b"duplicity-full.bad.vol1.difftar", config),
        Err(NameError::Malformed)
    );
    let valid = parse_name(b"duplicity-full.20231114T221320Z.vol1.difftar", config)
        .unwrap()
        .unwrap();
    assert_eq!(valid.kind, ObjectKind::FullVolume);
    assert_eq!(valid.volume, Some(1));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
#[test]
fn successful_name_survives_a_malformed_overlapping_kind_stem() {
    let parsed = parse_name(
        b"duplicity-full.20231114T221320Z.vol1.difftar",
        NameConfig::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(parsed.kind, ObjectKind::FullVolume);
    assert_eq!(parsed.volume, Some(1));
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
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
#[test]
fn short_full_volume_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"df.1.1.dt", Envelope::Plain, false),
        (b"df.1.1.dt.p", Envelope::Plain, true),
        (b"df.1.1.dt.z", Envelope::Gzip, false),
        (b"df.1.1.dt.g", Envelope::OpenPgp, false),
        (b"df.1.1.dt.p.z", Envelope::Gzip, true),
        (b"df.1.1.dt.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullVolume, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(1), "{bytes:?}");
        assert_eq!(name.volume, Some(1), "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    let nontrivial_fields = parse_name(b"df.2.z.dt", config).unwrap().unwrap();
    assert_eq!(nontrivial_fields.kind, ObjectKind::FullVolume);
    assert_eq!(nontrivial_fields.start, Timestamp(2));
    assert_eq!(nontrivial_fields.end, Timestamp(2));
    assert_eq!(nontrivial_fields.volume, Some(35));
    assert_eq!(nontrivial_fields.envelope, Envelope::Plain);
    assert!(!nontrivial_fields.partial);
    assert!(nontrivial_fields.short);

    for malformed in [
        b"df.1" as &[u8],
        b"df.1.1",
        b"df.Invalid.1.dt" as &[u8],
        b"df.1.0.dt",
        b"df.1.A.dt",
        b"df.1.1.bad",
        b"df.1.1.dt.x",
        b"df.1.1.dt.z.p",
        b"df.1.1.dt.p.p",
        b"df.1.1.dt.p.g.z",
        b"df.1.1.dt.p.z.extra",
        b"df.1.1.dt.",
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
//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
//= docs/parsing-requirements.md#parse-gen-003
//= type=test
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
#[test]
fn long_full_volume_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected_timestamp = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullVolume, "{bytes:?}");
        assert_eq!(
            (name.start, name.end),
            (expected_timestamp, expected_timestamp),
            "{bytes:?}"
        );
        assert_eq!(name.volume, Some(1), "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-full.bad.vol1.difftar" as &[u8],
        b"duplicity-full.20231114T221320Z.difftar",
        b"duplicity-full.20231114T221320Z.vol.difftar",
        b"duplicity-full.20231114T221320Z.vol0.difftar",
        b"duplicity-full.20231114T221320Z.volume1.difftar",
        b"duplicity-full.20231114T221320Z.vol1",
        b"duplicity-full.20231114T221320Z.vol1.manifest",
        b"duplicity-full.20231114T221320Z.vol1.difftar.gz.part",
        b"duplicity-full.20231114T221320Z.vol1.difftar.part.part",
        b"duplicity-full.20231114T221320Z.vol1.difftar.part.gz.extra",
        b"duplicity-full.20231114T221320Z.vol1.difftar.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-full.1970-01-01T00_00_00Z.vol1.difftar",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!((legacy.start, legacy.end), (Timestamp(0), Timestamp(0)));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
#[test]
fn long_full_manifest_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-full.20231114T221320Z.manifest",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullManifest, "{bytes:?}");
        assert_eq!((name.start, name.end), (expected, expected), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-full.bad.manifest" as &[u8],
        b"duplicity-full.20231114T221320Z",
        b"duplicity-full.20231114T221320Z.wrong",
        b"duplicity-full.20231114T221320Z.manifest.x",
        b"duplicity-full.20231114T221320Z.manifest.gz.part",
        b"duplicity-full.20231114T221320Z.manifest.part.part",
        b"duplicity-full.20231114T221320Z.manifest.part.gz.extra",
        b"duplicity-full.20231114T221320Z.manifest.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-full.1970-01-01T00_00_00Z.manifest",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!((legacy.start, legacy.end), (Timestamp(0), Timestamp(0)));
    assert_eq!(legacy.volume, None);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
#[test]
fn long_full_signatures_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullSignatures, "{bytes:?}");
        assert_eq!((name.start, name.end), (expected, expected), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-full-signatures.bad.sigtar" as &[u8],
        b"duplicity-full-signatures.20231114T221320Z",
        b"duplicity-full-signatures.20231114T221320Z.wrong",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.x",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.gz.part",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.part.part",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.part.gz.extra",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-full-signatures.1970-01-01T00_00_00Z.sigtar",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!((legacy.start, legacy.end), (Timestamp(0), Timestamp(0)));
    assert_eq!(legacy.volume, None);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
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
fn long_incremental_manifest_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected_start = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let expected_end = parse_long_timestamp(b"20231115T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalManifest, "{bytes:?}");
        assert_eq!(
            (name.start, name.end),
            (expected_start, expected_end),
            "{bytes:?}"
        );
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-inc.bad.to.20231115T221320Z.manifest" as &[u8],
        b"duplicity-inc.20231114T221320Z.to.bad.manifest",
        b"duplicity-inc.20231114T221320Z.to.20231114T221320Z.manifest",
        b"duplicity-inc.20231115T221320Z.to.20231114T221320Z.manifest",
        b"duplicity-inc.20231114T221320Z.from.20231115T221320Z.manifest",
        b"duplicity-inc.20231114T221320Z.20231115T221320Z.manifest",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.wrong",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.x",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gz.part",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part.part",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part.gz.extra",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-inc.1970-01-01T00_00_00Z.to.1970-01-02T00_00_00Z.manifest",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        (legacy.start, legacy.end),
        (Timestamp(0), Timestamp(86_400))
    );
    assert_eq!(legacy.volume, None);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
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
fn long_incremental_signatures_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected_start = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let expected_end = parse_long_timestamp(b"20231115T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalSignatures, "{bytes:?}");
        assert_eq!(
            (name.start, name.end),
            (expected_start, expected_end),
            "{bytes:?}"
        );
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-new-signatures.bad.to.20231115T221320Z.sigtar" as &[u8],
        b"duplicity-new-signatures.20231114T221320Z.to.bad.sigtar",
        b"duplicity-new-signatures.20231114T221320Z.to.20231114T221320Z.sigtar",
        b"duplicity-new-signatures.20231115T221320Z.to.20231114T221320Z.sigtar",
        b"duplicity-new-signatures.20231114T221320Z.from.20231115T221320Z.sigtar",
        b"duplicity-new-signatures.20231114T221320Z.20231115T221320Z.sigtar",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.wrong",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.x",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz.part",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.part",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gz.extra",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-new-signatures.1970-01-01T00_00_00Z.to.1970-01-02T00_00_00Z.sigtar",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        (legacy.start, legacy.end),
        (Timestamp(0), Timestamp(86_400))
    );
    assert_eq!(legacy.volume, None);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=test
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=test
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
//= docs/parsing-requirements.md#parse-gen-003
//= type=test
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
#[test]
fn long_incremental_volume_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let expected_start = parse_long_timestamp(b"20231114T221320Z", b':').unwrap();
    let expected_end = parse_long_timestamp(b"20231115T221320Z", b':').unwrap();
    let profiles: [(&[u8], Envelope, bool); 8] = [
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar",
            Envelope::Plain,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part",
            Envelope::Plain,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
            Envelope::Gzip,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gpg",
            Envelope::OpenPgp,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.p7m",
            Envelope::Pkcs7,
            false,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.gz",
            Envelope::Gzip,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.gpg",
            Envelope::OpenPgp,
            true,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.p7m",
            Envelope::Pkcs7,
            true,
        ),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalVolume, "{bytes:?}");
        assert_eq!(
            (name.start, name.end),
            (expected_start, expected_end),
            "{bytes:?}"
        );
        assert_eq!(name.volume, Some(1), "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(!name.short, "{bytes:?}");
    }

    for malformed in [
        b"duplicity-inc.bad.to.20231115T221320Z.vol1.difftar" as &[u8],
        b"duplicity-inc.20231114T221320Z.to.bad.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231114T221320Z.vol1.difftar",
        b"duplicity-inc.20231115T221320Z.to.20231114T221320Z.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.from.20231115T221320Z.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.20231115T221320Z.vol1.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol0.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol18446744073709551616.difftar",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz.part",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.part",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.gz.extra",
        b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.",
    ] {
        assert_eq!(
            parse_name(malformed, config),
            Err(NameError::Malformed),
            "{malformed:?}"
        );
    }

    let legacy = parse_name(
        b"duplicity-inc.1970-01-01T00_00_00Z.to.1970-01-02T00_00_00Z.vol1.difftar",
        NameConfig {
            time_separator: b'_',
            ..config
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        (legacy.start, legacy.end),
        (Timestamp(0), Timestamp(86_400))
    );
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
#[test]
fn short_full_manifest_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"df.1.m", Envelope::Plain, false),
        (b"df.1.m.p", Envelope::Plain, true),
        (b"df.1.m.z", Envelope::Gzip, false),
        (b"df.1.m.g", Envelope::OpenPgp, false),
        (b"df.1.m.p.z", Envelope::Gzip, true),
        (b"df.1.m.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullManifest, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(1), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    for malformed in [
        b"df.1" as &[u8],
        b"df.Invalid.m" as &[u8],
        b"df.1.bad",
        b"df.1.m.x",
        b"df.1.m.g.p",
        b"df.1.m.p.p",
        b"df.1.m.p.g.z",
        b"df.1.m.p.z.extra",
        b"df.1.m.",
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
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
#[test]
fn short_full_signatures_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"dfs.1.st", Envelope::Plain, false),
        (b"dfs.1.st.p", Envelope::Plain, true),
        (b"dfs.1.st.z", Envelope::Gzip, false),
        (b"dfs.1.st.g", Envelope::OpenPgp, false),
        (b"dfs.1.st.p.z", Envelope::Gzip, true),
        (b"dfs.1.st.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::FullSignatures, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(1), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    for malformed in [
        b"dfs.1" as &[u8],
        b"dfs.Invalid.st" as &[u8],
        b"dfs.1.bad",
        b"dfs.1.st.x",
        b"dfs.1.st.g.p",
        b"dfs.1.st.p.p",
        b"dfs.1.st.p.g.z",
        b"dfs.1.st.p.z.extra",
        b"dfs.1.st.",
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
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
#[test]
fn short_incremental_volume_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"di.1.2.1.dt", Envelope::Plain, false),
        (b"di.1.2.1.dt.p", Envelope::Plain, true),
        (b"di.1.2.1.dt.z", Envelope::Gzip, false),
        (b"di.1.2.1.dt.g", Envelope::OpenPgp, false),
        (b"di.1.2.1.dt.p.z", Envelope::Gzip, true),
        (b"di.1.2.1.dt.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalVolume, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(2), "{bytes:?}");
        assert_eq!(name.volume, Some(1), "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    for malformed in [
        b"di.1.1.1.dt" as &[u8],
        b"di.2.1.1.dt",
        b"di.Invalid.2.1.dt",
        b"di.1.Invalid.1.dt",
        b"di.1.2.0.dt",
        b"di.1.2.A.dt",
        b"di.1.2.1.bad",
        b"di.1.2.1.dt.x",
        b"di.1.2.1.dt.z.p",
        b"di.1.2.1.dt.p.p",
        b"di.1.2.1.dt.p.g.extra",
        b"di.1.2.1.dt.",
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
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
#[test]
fn short_incremental_manifest_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"di.1.2.m", Envelope::Plain, false),
        (b"di.1.2.m.p", Envelope::Plain, true),
        (b"di.1.2.m.z", Envelope::Gzip, false),
        (b"di.1.2.m.g", Envelope::OpenPgp, false),
        (b"di.1.2.m.p.z", Envelope::Gzip, true),
        (b"di.1.2.m.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalManifest, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(2), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    for malformed in [
        b"di.1.1.m" as &[u8],
        b"di.2.1.m",
        b"di.Invalid.2.m",
        b"di.1.Invalid.m",
        b"di.1.2.bad",
        b"di.1.2.m.x",
        b"di.1.2.m.z.p",
        b"di.1.2.m.p.p",
        b"di.1.2.m.p.g.extra",
        b"di.1.2.m.",
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
//= docs/parsing-requirements.md#parse-name-003
//= type=test
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=test
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
#[test]
fn short_incremental_signatures_body_composes_fields_and_rejects_malformed_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], Envelope, bool); 6] = [
        (b"dns.1.2.st", Envelope::Plain, false),
        (b"dns.1.2.st.p", Envelope::Plain, true),
        (b"dns.1.2.st.z", Envelope::Gzip, false),
        (b"dns.1.2.st.g", Envelope::OpenPgp, false),
        (b"dns.1.2.st.p.z", Envelope::Gzip, true),
        (b"dns.1.2.st.p.g", Envelope::OpenPgp, true),
    ];
    for (bytes, envelope, partial) in profiles {
        let name = parse_name(bytes, config).unwrap().unwrap();
        assert_eq!(name.kind, ObjectKind::IncrementalSignatures, "{bytes:?}");
        assert_eq!(name.start, Timestamp(1), "{bytes:?}");
        assert_eq!(name.end, Timestamp(2), "{bytes:?}");
        assert_eq!(name.volume, None, "{bytes:?}");
        assert_eq!(
            (name.envelope, name.partial),
            (envelope, partial),
            "{bytes:?}"
        );
        assert!(name.short, "{bytes:?}");
    }

    for malformed in [
        b"dns.1.1.st" as &[u8],
        b"dns.2.1.st",
        b"dns.Invalid.2.st",
        b"dns.1.Invalid.st",
        b"dns.1.2.bad",
        b"dns.1.2.st.x",
        b"dns.1.2.st.z.p",
        b"dns.1.2.st.p.p",
        b"dns.1.2.st.p.g.extra",
        b"dns.1.2.st.",
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
fn long_trailer_suffixes_classify_every_kind_and_reject_extra_fields() {
    let config = NameConfig::default();
    let profiles: [(&[u8], ObjectKind); 6] = [
        (
            b"duplicity-full.20231114T221320Z.vol1.difftar",
            ObjectKind::FullVolume,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar",
            ObjectKind::IncrementalVolume,
        ),
        (
            b"duplicity-full.20231114T221320Z.manifest",
            ObjectKind::FullManifest,
        ),
        (
            b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
            ObjectKind::IncrementalManifest,
        ),
        (
            b"duplicity-full-signatures.20231114T221320Z.sigtar",
            ObjectKind::FullSignatures,
        ),
        (
            b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar",
            ObjectKind::IncrementalSignatures,
        ),
    ];
    let suffixes: [(&[u8], Envelope, bool); 8] = [
        (b"", Envelope::Plain, false),
        (b".part", Envelope::Plain, true),
        (b".gz", Envelope::Gzip, false),
        (b".gpg", Envelope::OpenPgp, false),
        (b".p7m", Envelope::Pkcs7, false),
        (b".part.gz", Envelope::Gzip, true),
        (b".part.gpg", Envelope::OpenPgp, true),
        (b".part.p7m", Envelope::Pkcs7, true),
    ];
    for (base, kind) in profiles {
        for (suffix, envelope, partial) in suffixes {
            let name = [base, suffix].concat();
            let parsed = parse_name(&name, config).unwrap().unwrap();
            assert_eq!(
                (parsed.kind, parsed.envelope, parsed.partial),
                (kind, envelope, partial),
                "{name:?}"
            );
        }
    }

    for malformed in [
        b"duplicity-full.20231114T221320Z.vol1.manifest" as &[u8],
        b"duplicity-full.20231114T221320Z.vol1.difftar.",
        b"duplicity-full.20231114T221320Z.vol1.difftar.gz.part",
        b"duplicity-full.20231114T221320Z.vol1.difftar.part.part",
        b"duplicity-full.20231114T221320Z.vol1.difftar.part.p7m.extra",
        b"duplicity-full.20231114T221320Z.manifest.gz.part",
        b"duplicity-full-signatures.20231114T221320Z.sigtar.part.gpg.p7m",
        b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.manifest",
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

use duplicity_core::chain::{
    ChainError, ChainLimits, ChainPlan, DescriptorError, SetDescriptor, SetKind,
};
use duplicity_core::names::{parse_name, Envelope, NameConfig, ObjectKind, ObjectName, Timestamp};

fn name(kind: ObjectKind, start: i128, end: i128) -> ObjectName {
    ObjectName {
        kind,
        start: Timestamp(start),
        end: Timestamp(end),
        volume: None,
        envelope: Envelope::Plain,
        partial: false,
        short: false,
    }
}
fn full(time: i128) -> SetDescriptor {
    SetDescriptor::from_manifest_name(name(ObjectKind::FullManifest, time, time)).unwrap()
}
fn inc(start: i128, end: i128) -> SetDescriptor {
    SetDescriptor::from_manifest_name(name(ObjectKind::IncrementalManifest, start, end)).unwrap()
}
fn error(sets: &[SetDescriptor], anchor: usize, target: i128) -> ChainError {
    match ChainPlan::new(sets, anchor, Timestamp(target), ChainLimits::default()) {
        Err(error) => error,
        Ok(_) => panic!("unexpected successful plan"),
    }
}

#[test]
fn descriptors_revalidate_completed_manifest_structure() {
    for kind in [
        ObjectKind::FullVolume,
        ObjectKind::IncrementalVolume,
        ObjectKind::FullSignatures,
        ObjectKind::IncrementalSignatures,
    ] {
        assert_eq!(
            SetDescriptor::from_manifest_name(name(kind, 0, 1)),
            Err(DescriptorError::NotCompletedManifest)
        );
    }
    for mutate in 0..3 {
        let mut input = name(ObjectKind::FullManifest, 1, 1);
        match mutate {
            0 => input.partial = true,
            1 => input.envelope = Envelope::Pkcs7,
            _ => input.volume = Some(1),
        }
        assert!(SetDescriptor::from_manifest_name(input).is_err());
    }
    for input in [
        name(ObjectKind::FullManifest, 0, 1),
        name(ObjectKind::IncrementalManifest, 1, 1),
        name(ObjectKind::IncrementalManifest, 2, 1),
    ] {
        assert_eq!(
            SetDescriptor::from_manifest_name(input),
            Err(DescriptorError::InvalidInterval)
        );
    }
    let mut encrypted = name(ObjectKind::IncrementalManifest, -1, 0);
    encrypted.envelope = Envelope::OpenPgp;
    assert_eq!(
        SetDescriptor::from_manifest_name(encrypted).unwrap().kind(),
        SetKind::Incremental
    );
}

//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
#[test]
fn explicit_anchor_exact_target_and_borrowed_order() {
    let sets = [inc(20, 30), full(10), inc(10, 20), full(20), inc(30, 40)];
    let plan = ChainPlan::new(&sets, 1, Timestamp(30), ChainLimits::default()).unwrap();
    assert_eq!(plan.indices(), [1, 2, 0]);
    assert_eq!(plan.target(), Timestamp(30));
    for (entry, index) in plan.sets().zip(plan.indices()) {
        assert!(std::ptr::eq(entry, &sets[*index]));
    }
    let anchored = ChainPlan::new(&sets, 3, Timestamp(30), ChainLimits::default()).unwrap();
    assert_eq!(anchored.indices(), [3, 0]);
    let direct = ChainPlan::new(
        &sets,
        1,
        Timestamp(10),
        ChainLimits {
            max_candidate_checks: 0,
            ..ChainLimits::default()
        },
    )
    .unwrap();
    assert_eq!(direct.indices(), [1]);
    assert_eq!(error(&sets, 0, 30), ChainError::InvalidAnchor);
    assert_eq!(error(&sets, sets.len(), 30), ChainError::InvalidAnchor);
    assert_eq!(error(&sets, 1, 9), ChainError::TargetBeforeAnchor);
    assert_eq!(error(&sets, 1, 25), ChainError::Missing);
    let extreme = [full(i128::MIN), inc(i128::MIN, i128::MAX)];
    assert_eq!(
        ChainPlan::new(&extreme, 0, Timestamp(i128::MAX), ChainLimits::default())
            .unwrap()
            .indices(),
        [0, 1]
    );
}

//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn gaps_duplicate_intervals_and_dead_end_branches_fail_explicitly() {
    assert_eq!(error(&[full(0), inc(1, 2)], 0, 2), ChainError::Missing);
    assert_eq!(error(&[full(0), inc(0, 1)], 0, 2), ChainError::Missing);
    assert_eq!(
        error(&[full(0), inc(0, 2), inc(0, 2)], 0, 2),
        ChainError::Ambiguous
    );
    // The branch to1 dead-ends, but conservative local uniqueness still rejects
    // silently choosing the other complete path to2.
    assert_eq!(
        error(&[full(0), inc(0, 1), inc(0, 2)], 0, 2),
        ChainError::Ambiguous
    );
    let future = [full(0), inc(0, 2), inc(0, 3)];
    assert_eq!(
        ChainPlan::new(&future, 0, Timestamp(2), ChainLimits::default())
            .unwrap()
            .indices(),
        [0, 1]
    );
}

#[test]
fn metadata_record_and_candidate_budgets_are_independent() {
    let sets = [full(0), inc(0, 1), inc(1, 2)];
    for limits in [
        ChainLimits {
            max_records: 2,
            ..ChainLimits::default()
        },
        ChainLimits {
            max_metadata_bytes: 0,
            ..ChainLimits::default()
        },
        ChainLimits {
            max_candidate_checks: 5,
            ..ChainLimits::default()
        },
    ] {
        assert!(matches!(
            ChainPlan::new(&sets, 0, Timestamp(2), limits),
            Err(ChainError::Limit)
        ));
    }
    assert!(ChainPlan::new(
        &sets,
        0,
        Timestamp(2),
        ChainLimits {
            max_candidate_checks: 6,
            ..ChainLimits::default()
        }
    )
    .is_ok());
}

//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
#[test]
fn official_manifest_names_plan_all_plain_producer_profiles() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for (directory, separator) in [
        ("duplicity-0.8.23/standard/plain", b':'),
        ("duplicity-0.8.23/short/plain", b':'),
        ("duplicity-0.8.23/legacy/plain", b'_'),
        ("duplicity-2.1.3/standard/plain", b':'),
        ("duplicity-3.0.7/plain", b':'),
        ("duplicity-3.2.1/standard/plain", b':'),
    ] {
        let mut sets = Vec::new();
        for object in std::fs::read_dir(root.join(directory)).unwrap() {
            let object = object.unwrap();
            let filename = object.file_name();
            let parsed = parse_name(
                filename.as_encoded_bytes(),
                NameConfig {
                    time_separator: separator,
                    ..NameConfig::default()
                },
            )
            .unwrap();
            if let Some(parsed) = parsed {
                if matches!(
                    parsed.kind,
                    ObjectKind::FullManifest | ObjectKind::IncrementalManifest
                ) {
                    sets.push(SetDescriptor::from_manifest_name(parsed).unwrap());
                }
            }
        }
        assert_eq!(sets.len(), 3, "{directory}");
        let anchor = sets
            .iter()
            .position(|set| set.kind() == SetKind::Full)
            .unwrap();
        for day in 0..3 {
            let target = Timestamp(1700000000 + day * 86400);
            let plan = ChainPlan::new(&sets, anchor, target, ChainLimits::default()).unwrap();
            assert_eq!(plan.indices().len(), day as usize + 1, "{directory}");
            assert_eq!(plan.sets().last().unwrap().end(), target);
        }
    }
}

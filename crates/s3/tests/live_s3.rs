//! Explicitly opt-in acceptance against an operator-approved, read-only S3
//! mirror. All gate and inventory tests below are local and do not construct
//! an SDK client. The ignored live test validates every input before connect.
use duplicity_core::{
    names::{NameConfig, Timestamp},
    reader::{Catalog, DiscoveryStatus, ReaderLimits},
    source::{Cancellation, ObjectSource, SourceLimits},
    tar::EntryType,
};
use duplicity_s3::{S3Config, S3Source};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    io::{Read, Take},
    path::PathBuf,
};

const OPT_IN: &str = "DUPLINAUT_LIVE_S3";
const BUCKET: &str = "DUPLINAUT_LIVE_S3_BUCKET";
const PREFIX: &str = "DUPLINAUT_LIVE_S3_PREFIX";
const INVENTORY: &str = "DUPLINAUT_LIVE_S3_INVENTORY";
const MAX_OBJECTS: usize = 12;
const MAX_METADATA: usize = 64 * 1024;
const MAX_INVENTORY_BYTES: usize = 64 * 1024;
const MAX_STORED_BYTES: u64 = 2 * 1024 * 1024;
const FINAL_TIME: i64 = 1_700_172_800;

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveSettings {
    bucket: String,
    prefix: String,
    inventory_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpectedObject {
    fixture_path: String,
    key_suffix: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InventoryObject {
    key: String,
    fixture_path: String,
    size: u64,
    sha256: String,
    etag: String,
    version_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Inventory {
    bucket: String,
    prefix: String,
    versioning_enabled: bool,
    objects: Vec<InventoryObject>,
}

/// Absence is the normal offline state. A present but malformed opt-in is an
/// error, so misspelled/manual invocations cannot silently skip the live run.
fn live_settings_from(
    values: &HashMap<String, String>,
) -> Result<Option<LiveSettings>, &'static str> {
    let Some(opt_in) = values.get(OPT_IN) else {
        return Ok(None);
    };
    if opt_in != "1" {
        return Err("invalid live S3 opt-in");
    }
    let required = |key| {
        values
            .get(key)
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or("missing live S3 setting")
    };
    let bucket = required(BUCKET)?;
    let prefix = required(PREFIX)?;
    let inventory_path = PathBuf::from(required(INVENTORY)?);
    if prefix.starts_with('/')
        || !prefix.ends_with('/')
        || prefix.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err("invalid live S3 prefix");
    }
    Ok(Some(LiveSettings {
        bucket,
        prefix,
        inventory_path,
    }))
}

/// Gate before invoking the supplied loader. This makes the ordering testable:
/// without the exact opt-in and all required settings, inventory I/O is absent.
fn load_live_inputs<T>(
    values: &HashMap<String, String>,
    read_inventory: impl FnOnce(&PathBuf) -> Result<T, &'static str>,
) -> Result<Option<(LiveSettings, T)>, &'static str> {
    let Some(settings) = live_settings_from(values)? else {
        return Ok(None);
    };
    let inventory = read_inventory(&settings.inventory_path)?;
    Ok(Some((settings, inventory)))
}

fn fixture_objects() -> Result<Vec<ExpectedObject>, &'static str> {
    let provenance: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/provenance.json"
    ))
    .map_err(|_| "invalid checked-in fixture provenance")?;
    let objects = provenance["objects"]
        .as_array()
        .ok_or("missing checked-in fixture object table")?;
    let mut output = Vec::new();
    for object in objects {
        let path = object["path"]
            .as_str()
            .ok_or("invalid checked-in fixture path")?;
        let Some(suffix) = path.strip_prefix("plain/") else {
            continue;
        };
        let size = object["bytes"]
            .as_u64()
            .ok_or("invalid checked-in fixture size")?;
        let sha256 = object["sha256"]
            .as_str()
            .ok_or("invalid checked-in fixture digest")?;
        if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("invalid checked-in fixture digest");
        }
        if size > MAX_STORED_BYTES {
            return Err("checked-in fixture exceeds live object bound");
        }
        output.push(ExpectedObject {
            fixture_path: path.to_owned(),
            key_suffix: suffix.to_owned(),
            size,
            sha256: sha256.to_ascii_lowercase(),
        });
    }
    output.sort_by(|left, right| left.fixture_path.cmp(&right.fixture_path));
    if output.len() != MAX_OBJECTS
        || output.iter().map(|object| object.size).sum::<u64>() != 4_156_441
    {
        return Err("checked-in plain fixture inventory changed");
    }
    Ok(output)
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, &'static str> {
    value[field].as_str().ok_or("malformed live S3 inventory")
}

fn parse_inventory(text: &str) -> Result<Inventory, &'static str> {
    if text.len() > MAX_INVENTORY_BYTES {
        return Err("live S3 inventory exceeds size bound");
    }
    let value: Value = serde_json::from_str(text).map_err(|_| "malformed live S3 inventory")?;
    let bucket = string_field(&value, "bucket")?.to_owned();
    let prefix = string_field(&value, "prefix")?.to_owned();
    let captured_at = string_field(&value, "captured_at")?;
    if captured_at.trim().is_empty() {
        return Err("malformed live S3 inventory");
    }
    let versioning_enabled = value["versioning_enabled"]
        .as_bool()
        .ok_or("malformed live S3 inventory")?;
    let items = value["objects"]
        .as_array()
        .ok_or("malformed live S3 inventory")?;
    if items.len() != MAX_OBJECTS {
        return Err("live S3 inventory object count mismatch");
    }
    let mut objects = Vec::with_capacity(MAX_OBJECTS);
    for item in items {
        let key = string_field(item, "key")?.to_owned();
        let fixture_path = string_field(item, "fixture_path")?.to_owned();
        let size = item["size"].as_u64().ok_or("malformed live S3 inventory")?;
        let sha256 = string_field(item, "sha256")?.to_ascii_lowercase();
        let etag = string_field(item, "etag")?.to_owned();
        if etag.len() < 3
            || !etag.starts_with('"')
            || !etag.ends_with('"')
            || etag.bytes().any(|byte| byte.is_ascii_control())
            || sha256.len() != 64
            || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("malformed live S3 inventory");
        }
        let version_id = match item.get("version_id") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
            _ => return Err("malformed live S3 inventory"),
        };
        // These values are capture provenance only. Keep their types checked,
        // but do not interpret them as runtime reader identity or policy.
        if string_field(item, "last_modified")?.trim().is_empty() {
            return Err("malformed live S3 inventory");
        }
        if let Some(content_type) = item.get("content_type") {
            if !content_type.is_string() && !content_type.is_null() {
                return Err("malformed live S3 inventory");
            }
        }
        if let Some(metadata) = item.get("user_metadata") {
            if !metadata.is_object() && !metadata.is_null() {
                return Err("malformed live S3 inventory");
            }
        }
        objects.push(InventoryObject {
            key,
            fixture_path,
            size,
            sha256,
            etag,
            version_id,
        });
    }
    Ok(Inventory {
        bucket,
        prefix,
        versioning_enabled,
        objects,
    })
}

fn validate_inventory(
    settings: &LiveSettings,
    inventory: &Inventory,
    expected: &[ExpectedObject],
) -> Result<(), &'static str> {
    if inventory.bucket != settings.bucket || inventory.prefix != settings.prefix {
        return Err("live S3 inventory does not match selected source");
    }
    if inventory.objects.len() != expected.len() {
        return Err("live S3 inventory object count mismatch");
    }
    let by_path = inventory
        .objects
        .iter()
        .map(|object| (object.fixture_path.as_str(), object))
        .collect::<BTreeMap<_, _>>();
    let unique_keys = inventory
        .objects
        .iter()
        .map(|object| object.key.as_str())
        .collect::<HashSet<_>>();
    if by_path.len() != inventory.objects.len() || unique_keys.len() != inventory.objects.len() {
        return Err("duplicate live S3 inventory object");
    }
    for expected_object in expected {
        let actual = by_path
            .get(expected_object.fixture_path.as_str())
            .ok_or("live S3 inventory path mismatch")?;
        let expected_key = format!("{}{}", settings.prefix, expected_object.key_suffix);
        if actual.key != expected_key
            || actual.size != expected_object.size
            || actual.sha256 != expected_object.sha256
            || actual.size > MAX_STORED_BYTES
        {
            return Err("live S3 inventory differs from checked-in fixture oracle");
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("malformed checked-in snapshot oracle");
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| "malformed checked-in snapshot oracle")
        })
        .collect()
}

fn compare_final_snapshot(snapshot: &duplicity_core::reader::ValidatedSnapshot) {
    let expected: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .expect("checked-in snapshot oracle parses");
    let record = expected["snapshots"]
        .as_array()
        .expect("snapshot oracle records")
        .iter()
        .find(|record| record["mode"] == "plain" && record["index"] == 2)
        .expect("final plain snapshot record");
    assert!(record["official_comparison_exclusions"]
        .as_array()
        .expect("oracle exclusions")
        .iter()
        .any(|value| value == "symlink_mtime_ns"));
    let entries = record["entries"].as_array().expect("oracle entries");
    assert_eq!(snapshot.state().len(), 21, "final snapshot entry count");
    let mut regular_bytes = 0u64;
    for (path, entry) in snapshot.state().entries() {
        let raw_components = path.components();
        let path_hex = raw_components
            .iter()
            .map(|component| hex(component))
            .collect::<Vec<_>>();
        let oracle = entries
            .iter()
            .find(|candidate| {
                candidate["components_hex"]
                    .as_array()
                    .is_some_and(|components| {
                        components
                            .iter()
                            .map(|component| component.as_str())
                            .collect::<Option<Vec<_>>>()
                            .is_some_and(|values| {
                                values == path_hex.iter().map(String::as_str).collect::<Vec<_>>()
                            })
                    })
            })
            .expect("snapshot path has an oracle record");
        let oracle_components = oracle["components_hex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|component| decode_hex(component.as_str().unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert!(
            raw_components.as_slice() == oracle_components.as_slice(),
            "raw snapshot path components mismatch"
        );
        assert_eq!(
            entry.mode() & 0o777,
            oracle["mode"].as_u64().unwrap() as u32
        );
        let mtime = entry.mtime();
        let mtime_ns = (mtime.seconds as i128)
            .checked_mul(1_000_000_000)
            .and_then(|value| value.checked_add(mtime.nanoseconds as i128))
            .expect("oracle nanosecond conversion fits i128");
        if oracle["kind"] != "symlink" {
            assert_eq!(mtime_ns, oracle["mtime_ns"].as_i64().unwrap() as i128);
        }
        match oracle["kind"].as_str().unwrap() {
            "file" => {
                assert_eq!(entry.entry_type(), EntryType::Regular);
                let content = entry.content().expect("regular file content");
                regular_bytes += content.len() as u64;
                assert_eq!(content.len() as u64, oracle["size"].as_u64().unwrap());
                assert_eq!(
                    hex(&Sha256::digest(content)),
                    oracle["sha256"].as_str().unwrap()
                );
            }
            "directory" => assert_eq!(entry.entry_type(), EntryType::Directory),
            "symlink" => {
                assert_eq!(entry.entry_type(), EntryType::Symlink);
                assert_eq!(
                    hex(entry.linkname()),
                    oracle["symlink_target_hex"].as_str().unwrap()
                );
            }
            _ => panic!("unsupported checked-in oracle type"),
        }
    }
    assert_eq!(regular_bytes, 2_621_564, "final snapshot payload bytes");
}

fn hash_object(mut reader: impl Read, expected_size: u64) -> Result<String, &'static str> {
    if expected_size > MAX_STORED_BYTES {
        return Err("object exceeds live stored-byte bound");
    }
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| "live object stream failed")?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .filter(|total| *total <= expected_size && *total <= MAX_STORED_BYTES)
            .ok_or("live object byte count exceeded bound")?;
        hash.update(&buffer[..count]);
    }
    if total != expected_size {
        return Err("live object byte count mismatch");
    }
    Ok(hex(&hash.finalize()))
}

fn safe<T>(result: Result<T, impl std::fmt::Debug>, stage: &'static str) -> T {
    result.unwrap_or_else(|_| panic!("live S3 acceptance failed at {stage}"))
}

fn read_inventory_file(path: &PathBuf) -> Result<String, &'static str> {
    let file = fs::File::open(path).map_err(|_| "unable to open live S3 inventory")?;
    let mut bytes = Vec::with_capacity(MAX_INVENTORY_BYTES + 1);
    let read_limit = MAX_INVENTORY_BYTES as u64 + 1;
    let mut reader: Take<fs::File> = file.take(read_limit);
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| "unable to read live S3 inventory")?;
    if bytes.len() > MAX_INVENTORY_BYTES {
        return Err("live S3 inventory exceeds size bound");
    }
    String::from_utf8(bytes).map_err(|_| "live S3 inventory is not UTF-8")
}

#[test]
fn live_s3_gate_is_default_off_and_requires_exact_opt_in_before_inventory_io() {
    let empty = HashMap::new();
    let mut inventory_read = false;
    let disabled = load_live_inputs(&empty, |_| {
        inventory_read = true;
        Ok::<_, &'static str>(())
    })
    .unwrap();
    assert!(disabled.is_none());
    assert!(!inventory_read, "disabled gate must not read inventory");

    let mut values = HashMap::from([(OPT_IN.to_owned(), "yes".to_owned())]);
    inventory_read = false;
    assert!(load_live_inputs(&values, |_| {
        inventory_read = true;
        Ok::<_, &'static str>(())
    })
    .is_err());
    assert!(
        !inventory_read,
        "invalid opt-in must fail before inventory I/O"
    );

    values.insert(OPT_IN.to_owned(), "1".to_owned());
    assert!(load_live_inputs(&values, |_| {
        inventory_read = true;
        Ok::<_, &'static str>(())
    })
    .is_err());
    assert!(
        !inventory_read,
        "missing settings must fail before inventory I/O"
    );
}

#[test]
fn synthetic_inventory_parser_and_oracle_gate_reject_mismatch_without_sdk() {
    let settings = LiveSettings {
        bucket: "approved-bucket".into(),
        prefix: "duplinaut/fixture/".into(),
        inventory_path: PathBuf::from("synthetic-inventory.json"),
    };
    let expected = fixture_objects().unwrap();
    let objects = expected
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{{\"key\":\"{}{}\",\"fixture_path\":\"{}\",\"size\":{},\"sha256\":\"{}\",\"etag\":\"\\\"fixture-{index}\\\"\",\"version_id\":null,\"last_modified\":\"2026-01-01T00:00:00Z\"}}",
                settings.prefix, item.key_suffix, item.fixture_path, item.size, item.sha256
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let text = format!(
        "{{\"bucket\":\"{}\",\"prefix\":\"{}\",\"captured_at\":\"2026-01-01T00:00:00Z\",\"versioning_enabled\":false,\"objects\":[{}]}}",
        settings.bucket, settings.prefix, objects
    );
    let inventory = parse_inventory(&text).unwrap();
    assert!(!inventory.versioning_enabled);
    validate_inventory(&settings, &inventory, &expected).unwrap();

    assert!(parse_inventory("{}").is_err());
    assert!(parse_inventory(&" ".repeat(MAX_INVENTORY_BYTES + 1)).is_err());
    assert!(
        parse_inventory(&text.replace(",\"last_modified\":\"2026-01-01T00:00:00Z\"", "")).is_err()
    );
    assert!(parse_inventory(
        &text.replace("\"etag\":\"\\\"fixture-0\\\"", "\"etag\":\"fixture-0\"")
    )
    .is_err());
    let extra_object = text.replace(
        "\"objects\":[",
        "\"objects\":[{\"key\":\"extra\",\"fixture_path\":\"plain/extra\",\"size\":1,\"sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"etag\":\"\\\"extra\\\"\"},",
    );
    assert!(parse_inventory(&extra_object).is_err());
    let wrong_source = LiveSettings {
        bucket: "other-bucket".into(),
        ..settings
    };
    assert!(validate_inventory(&wrong_source, &inventory, &expected).is_err());
}

//= docs/parsing-requirements.md#parse-source-001
//= type=test
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
#[test]
#[ignore = "requires an operator-approved read-only S3 fixture and explicit environment opt-in"]
fn live_s3_official_plain_fixture_read_only_acceptance() {
    let values = [OPT_IN, BUCKET, PREFIX, INVENTORY]
        .into_iter()
        .filter_map(|name| env::var(name).ok().map(|value| (name.to_owned(), value)))
        .collect::<HashMap<_, _>>();
    let (settings, inventory_text) = safe(
        load_live_inputs(&values, read_inventory_file)
            .and_then(|value| value.ok_or("explicit live S3 opt-in required")),
        "input gate",
    );
    let inventory = safe(parse_inventory(&inventory_text), "inventory parse");
    let expected_objects = safe(fixture_objects(), "fixture oracle");
    safe(
        validate_inventory(&settings, &inventory, &expected_objects),
        "inventory validation",
    );

    // This is the first SDK client construction, after every opt-in, setting,
    // inventory, fixture path, size and digest check has succeeded.
    let mut config = S3Config::new(settings.bucket.clone());
    config.prefix = settings.prefix.clone();
    config.request_timeout = std::time::Duration::from_secs(45);
    config.stream_idle_timeout = std::time::Duration::from_secs(20);
    let cancel = Cancellation::default();
    let source = safe(S3Source::connect(config, &cancel), "SDK connection");
    let source_limits = SourceLimits {
        max_objects: MAX_OBJECTS,
        max_metadata_bytes: MAX_METADATA,
        max_stored_bytes: MAX_STORED_BYTES,
    };
    let limits = ReaderLimits {
        source: source_limits,
        stored_buffer_bytes: MAX_STORED_BYTES as usize,
        catalog_metadata_bytes: MAX_METADATA,
        ..ReaderLimits::default()
    };
    let catalog = safe(
        Catalog::discover(&source, "", NameConfig::default(), limits, &cancel),
        "catalog discovery",
    );
    assert_eq!(catalog.objects().len(), MAX_OBJECTS, "listed object count");

    let expected_by_key = expected_objects
        .iter()
        .map(|expected| {
            (
                format!("{}{}", settings.prefix, expected.key_suffix),
                expected,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let inventory_by_key = inventory
        .objects
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut listed_keys = HashSet::new();
    let mut heads = Vec::with_capacity(MAX_OBJECTS);
    for (index, listed) in catalog.objects().iter().enumerate() {
        let object = listed.object();
        assert!(
            listed_keys.insert(object.key.as_str()),
            "duplicate listed object {index}"
        );
        let expected = expected_by_key
            .get(&object.key)
            .unwrap_or_else(|| panic!("unexpected listed object at index {index}"));
        let recorded = inventory_by_key
            .get(object.key.as_str())
            .unwrap_or_else(|| panic!("missing inventory object at index {index}"));
        assert_eq!(object.size, expected.size, "listed size at object {index}");
        assert_eq!(
            object.size, recorded.size,
            "inventory size at object {index}"
        );
        assert!(
            object.identity.etag.as_deref() == Some(recorded.etag.as_str()),
            "listed ETag mismatch at object {index}"
        );
        let head = safe(
            source.head(&object.key, source_limits, &cancel),
            "object HEAD",
        );
        assert!(
            head.key == object.key,
            "HEAD key mismatch at object {index}"
        );
        assert_eq!(head.size, object.size, "HEAD size at object {index}");
        assert!(
            head.identity.etag == object.identity.etag,
            "HEAD ETag mismatch at object {index}"
        );
        if let Some(version_id) = &recorded.version_id {
            assert!(
                head.identity.version_id.as_deref() == Some(version_id.as_str()),
                "HEAD version mismatch at object {index}"
            );
        }
        heads.push((object.key.clone(), head));
        let mut stream = safe(
            source.open(object, source_limits, &cancel),
            "conditional GET",
        );
        let digest = safe(hash_object(&mut stream, object.size), "object stream");
        assert_eq!(
            digest, expected.sha256,
            "official object digest at index {index}"
        );
    }
    assert_eq!(listed_keys.len(), MAX_OBJECTS, "complete listed key set");

    let manifest_chain = catalog
        .manifests()
        .map(|(_, _object, descriptor)| (descriptor.start().0, descriptor.end().0))
        .collect::<Vec<_>>();
    assert_eq!(
        manifest_chain.len(),
        3,
        "completed manifest descriptor count"
    );
    assert_eq!(
        manifest_chain
            .iter()
            .map(|(start, end)| (*start, *end))
            .collect::<Vec<_>>(),
        [
            (1_700_000_000i128, 1_700_000_000i128),
            (1_700_000_000i128, 1_700_086_400i128),
            (1_700_086_400i128, FINAL_TIME as i128),
        ],
        "full and incremental chain timestamps"
    );
    assert!(catalog
        .objects()
        .iter()
        .all(|object| object.status() == DiscoveryStatus::Completed));

    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| descriptor.start() == descriptor.end())
        .map(|(index, _, _)| index)
        .expect("unique full anchor");
    let snapshot = safe(
        catalog.read_snapshot(anchor, Timestamp(FINAL_TIME as i128), &cancel),
        "final snapshot read",
    );
    compare_final_snapshot(&snapshot);

    // Recheck the optional immutable identity and ETag after reconstruction;
    // listing-based object reads remain conditional on ETag rather than pinned
    // to VersionId, as required by the adapter contract.
    for (index, (key, before)) in heads.iter().enumerate() {
        let after = safe(source.head(key, source_limits, &cancel), "post-read HEAD");
        assert_eq!(after.size, before.size, "post-read size at object {index}");
        assert!(
            after.identity.etag == before.identity.etag,
            "post-read ETag mismatch at object {index}"
        );
        if let Some(version) = inventory_by_key[key.as_str()].version_id.as_deref() {
            assert!(
                after.identity.version_id.as_deref() == Some(version),
                "post-read version mismatch at object {index}"
            );
        }
    }
}

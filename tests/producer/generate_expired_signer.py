#!/usr/bin/env python3
"""Generate one official Duplicity 3.0.7 Full fixture with an expired signer."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

from generate import official_comparison, tree


PASSPHRASE = "duplicity-reader-rsa2048-test-fixture"
UID = "Duplicity Expired Signer Reader Fixture <expired-signer@example.invalid>"
TIMESTAMP = 1700000000
KEY_CREATED = TIMESTAMP - 86400
KEY_EXPIRY_DURATION = "1y"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_components(colons):
    components = []
    current = None
    for line in colons.decode("utf-8", "replace").splitlines():
        fields = line.split(":")
        if fields[0] in ("pub", "sub"):
            current = {
                "component": "primary" if fields[0] == "pub" else "subkey",
                "bits": int(fields[2]),
                "algorithm_id": int(fields[3]),
                "key_id": fields[4].upper(),
                "created_seconds": int(fields[5]),
                "expires_at_seconds": int(fields[6]) if fields[6] else None,
                "capabilities": fields[11] if len(fields) > 11 else "",
            }
            components.append(current)
        elif fields[0] == "fpr" and current is not None and len(fields) > 9:
            current["fingerprint"] = fields[9].upper()
    return components


def run_capture(argv, environment, commands):
    commands.append(argv)
    result = subprocess.run(argv, env=environment, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False)
    if result.returncode:
        safe_argv = [part.replace(PASSPHRASE, "${FIXTURE_PASSPHRASE}") for part in argv]
        safe_stderr = result.stderr.decode("utf-8", "replace").replace(
            PASSPHRASE, "${FIXTURE_PASSPHRASE}")
        raise RuntimeError(f"Command failed ({result.returncode}): {safe_argv!r}\n{safe_stderr}")
    return result.stdout, result.stderr


def run(argv, environment, commands):
    commands.append(argv)
    result = subprocess.run(argv, env=environment, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False)
    if result.returncode:
        safe_argv = [part.replace(PASSPHRASE, "${FIXTURE_PASSPHRASE}") for part in argv]
        safe_stderr = result.stderr.decode("utf-8", "replace").replace(
            PASSPHRASE, "${FIXTURE_PASSPHRASE}")
        raise RuntimeError(f"Command failed ({result.returncode}): {safe_argv!r}\n{safe_stderr}")
    return result.stdout


def primary_expiry_from_self_signature(
    packet_text, primary_key_id, primary_fingerprint, primary_created
):
    """Extract expiry only from a primary-issued primary self-signature."""
    text = packet_text.decode("utf-8", "replace")
    signature_classes = re.findall(r"sigclass (0x[0-9a-f]+)", text.lower())
    revocations = [value for value in signature_classes if value in ("0x20", "0x28", "0x30")]
    if revocations:
        raise RuntimeError(f"Fixture certificate contains revocation signature(s): {revocations}")
    packets = re.split(r"(?=:signature packet:)", text)
    candidates = []
    for packet in packets:
        if "signature packet:" not in packet:
            continue
        lowered = packet.lower()
        if f"keyid {primary_key_id.lower()}" not in lowered:
            continue
        signature_class = re.search(r"sigclass (0x[0-9a-f]+)", lowered)
        created = re.search(r"created (\d+)", lowered)
        expiry = re.search(r"key expires after (\d+)y(\d+)d(\d+)h(\d+)m", lowered)
        issuer_fingerprint = re.search(r"issuer fpr v4 ([0-9a-f]{40})", lowered)
        if (signature_class is None or created is None or expiry is None
                or issuer_fingerprint is None):
            continue
        class_value = int(signature_class.group(1), 16)
        if class_value != 0x1F and not 0x10 <= class_value <= 0x13:
            continue
        if int(created.group(1)) < primary_created:
            continue
        if issuer_fingerprint.group(1).upper() != primary_fingerprint.upper():
            continue
        duration = (((int(expiry.group(1)) * 365 + int(expiry.group(2))) * 24
                     + int(expiry.group(3))) * 60 + int(expiry.group(4))) * 60
        candidates.append({
            "signature_class": f"0x{class_value:02x}",
            "signature_created_seconds": int(created.group(1)),
            "key_expiry_seconds": duration,
            "key_expiry_subpacket_text": expiry.group(0),
            "issuer_key_id": primary_key_id.upper(),
            "issuer_fingerprint": issuer_fingerprint.group(1).upper(),
            "packet_excerpt": packet.strip(),
        })
    if not candidates:
        raise RuntimeError("No primary self-signature contains a finite key-expiry subpacket:\n" + text)
    durations = {item["key_expiry_seconds"] for item in candidates}
    if len(durations) != 1:
        raise RuntimeError(f"Primary self-signatures disagree on key expiry: {sorted(durations)}")
    candidates[0]["revocation_signature_classes"] = revocations
    return candidates[0]


def valid_signature_status(status, expected_fingerprint, expiry_seconds):
    records = []
    for line in status.decode("utf-8", "replace").splitlines():
        if not line.startswith("[GNUPG:] VALIDSIG "):
            continue
        fields = line.split()
        if len(fields) < 11:
            raise RuntimeError(f"Incomplete GnuPG VALIDSIG status: {line}")
        signer = fields[2].upper()
        created = int(fields[4])
        primary = fields[-1].upper() if len(fields) > 11 else signer
        if signer != expected_fingerprint or primary != expected_fingerprint:
            raise RuntimeError("Document signature is not made by the authorized primary itself")
        if not created < expiry_seconds:
            raise RuntimeError("Document signature was not created before primary key expiry")
        records.append({
            "signing_component_fingerprint": signer,
            "primary_fingerprint": primary,
            "signature_created_seconds": created,
            "public_key_algorithm_id": int(fields[8]),
            "hash_algorithm_id": int(fields[9]),
            "status_line": line,
        })
    if len(records) != 1:
        raise RuntimeError(f"Expected one VALIDSIG status, found {len(records)}:\n"
                           + status.decode("utf-8", "replace"))
    return records[0]


def normalized_command(command, scratch, repository_root):
    return [
        part.replace(str(scratch), "${WORK}")
        .replace(str(repository_root), "${REPO_ROOT}")
        .replace(PASSPHRASE, "${FIXTURE_PASSPHRASE}")
        for part in command
    ]


def make_rsa_source(root):
    """Create a deterministic UTF-8 source usable on macOS and Linux."""
    root.mkdir(mode=0o755)
    root.chmod(0o755)
    plain = root / "plain.txt"
    plain.write_bytes(b"RSA reader fixture\n")
    plain.chmod(0o644)
    empty = root / "empty.txt"
    empty.write_bytes(b"")
    empty.chmod(0o644)
    nested = root / "nested"
    nested.mkdir(mode=0o755)
    nested.chmod(0o755)
    payload = nested / "payload.bin"
    payload.write_bytes(bytes(range(256)) * 4)
    payload.chmod(0o640)
    (root / "link").symlink_to("plain.txt")
    timestamp_ns = (TIMESTAMP - 100) * 1_000_000_000
    for directory, child_dirs, child_files in os.walk(root, topdown=False):
        for name in child_files:
            os.utime(Path(directory) / name, ns=(timestamp_ns, timestamp_ns),
                     follow_symlinks=False)
        for name in child_dirs:
            os.utime(Path(directory) / name, ns=(timestamp_ns, timestamp_ns),
                     follow_symlinks=False)
        os.utime(directory, ns=(timestamp_ns, timestamp_ns), follow_symlinks=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--duplicity", type=Path, required=True)
    parser.add_argument("--gpg", type=Path, required=True)
    parser.add_argument("--expected-version", default="3.0.7")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists():
        raise SystemExit("Output must not already exist")

    commands = []
    environment = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", PASSPHRASE=PASSPHRASE)
    with tempfile.TemporaryDirectory(prefix="duplicity-expired-signer-fixture-", dir="/tmp") as scratch_name:
        scratch = Path(scratch_name)
        gnupg_home = scratch / "gnupg"
        gnupg_home.mkdir(mode=0o700)
        environment["GNUPGHOME"] = str(gnupg_home)
        environment["XDG_CACHE_HOME"] = str(scratch / "cache-home")
        environment["XDG_CONFIG_HOME"] = str(scratch / "config-home")

        duplicity_version = run([str(args.duplicity), "--version"], environment, commands).decode().strip()
        if args.expected_version not in duplicity_version.split():
            raise RuntimeError(f"Wrong official producer version: {duplicity_version}")
        gpg_version = run([str(args.gpg), "--version"], environment, commands).decode().splitlines()[0]

        key_arguments = [str(args.gpg), "--batch", "--pinentry-mode", "loopback",
                         "--passphrase", PASSPHRASE]
        run(key_arguments + ["--faked-system-time", f"{KEY_CREATED}!",
                             "--quick-generate-key", UID, "rsa2048", "sign",
                             KEY_EXPIRY_DURATION],
            environment, commands)
        key_listing = run([str(args.gpg), "--batch", "--with-colons", "--list-keys"],
                          environment, commands)
        primary = next(component for component in parse_components(key_listing)
                       if component["component"] == "primary")
        fingerprint = primary["fingerprint"]
        run(key_arguments + ["--faked-system-time", f"{KEY_CREATED + 1}!",
                             "--quick-add-key", fingerprint, "rsa2048", "encrypt", "0"],
            environment, commands)
        key_listing = run([str(args.gpg), "--batch", "--with-colons", "--list-keys", fingerprint],
                          environment, commands)
        components = parse_components(key_listing)
        primary = components[0]
        encryption_components = [component for component in components
                                 if "e" in component["capabilities"].lower()]
        if primary["bits"] != 2048 or primary["algorithm_id"] != 1:
            raise RuntimeError("Generated primary key is not RSA-2048")
        if "s" not in primary["capabilities"].lower():
            raise RuntimeError("The expiring primary key itself must have signing capability")
        if not encryption_components or any(
            component["bits"] != 2048 or component["algorithm_id"] != 1
            for component in encryption_components
        ):
            raise RuntimeError("Generated encryption component is not RSA-2048")
        if any("s" in component["capabilities"].lower()
               for component in components if component["component"] == "subkey"):
            raise RuntimeError("Fixture must not use a signing subkey")

        test_keys = scratch / "test-keys"
        test_keys.mkdir()
        public_export = run([str(args.gpg), "--armor", "--export", fingerprint], environment, commands)
        (test_keys / "public.asc").write_bytes(public_export)
        secret_export = run(key_arguments + ["--armor", "--export-secret-keys", fingerprint],
                            environment, commands)
        (test_keys / "secret.asc").write_bytes(secret_export)
        public_packets, _ = run_capture(
            [str(args.gpg), "--list-packets", str(test_keys / "public.asc")],
            environment, commands)
        public_packet_detail, _ = run_capture(
            [str(args.gpg), "--verbose", "--list-packets", str(test_keys / "public.asc")],
            environment, commands)
        key_id = primary["key_id"]
        primary_created = int(primary["created_seconds"])
        expiry_evidence = primary_expiry_from_self_signature(
            public_packet_detail, key_id, primary["fingerprint"], primary_created)
        expiry_seconds = primary_created + expiry_evidence["key_expiry_seconds"]
        if primary_created != KEY_CREATED:
            raise RuntimeError("GnuPG primary creation time differs from the pinned key clock")
        if expiry_seconds <= TIMESTAMP:
            raise RuntimeError("The fixed producer/signing time is not before key expiry")
        if expiry_seconds >= int(time.time()):
            raise RuntimeError("Fixture key expiry is not earlier than generation time")
        if primary["expires_at_seconds"] != expiry_seconds:
            raise RuntimeError("GnuPG key-list expiry differs from primary self-signature derivation")
        check_sigs_output, check_sigs_stderr = run_capture(
            [str(args.gpg), "--batch", "--faked-system-time", f"{TIMESTAMP}!",
             "--check-sigs", fingerprint], environment, commands)
        check_sigs_text = (check_sigs_output + check_sigs_stderr).decode("utf-8", "replace")
        if "sig!" not in check_sigs_text:
            raise RuntimeError("GnuPG did not report a valid certificate self-signature")
        packet_components = [
            {"version": int(version), "algorithm_id": int(algorithm), "bits": int(bits)}
            for version, algorithm, bits in re.findall(
                rb":public(?: sub)? key packet:\n\tversion (\d+), algo (\d+),[^\n]*\n\tpkey\[0\]: \[(\d+) bits\]",
                public_packets,
            )
        ]
        if len(packet_components) != 2 or any(
            component != {"version": 4, "algorithm_id": 1, "bits": 2048}
            for component in packet_components
        ):
            raise RuntimeError("Exported recipient is not a v4 RSA-2048 primary+subkey:\n" + public_packets.decode("utf-8", "replace"))
        (test_keys / "credentials.json").write_text(json.dumps({
            "fingerprint": fingerprint,
            "passphrase": PASSPHRASE,
            "purpose": "public RSA-2048 test fixture only",
            "secret_export_protected": True,
        }, indent=2) + "\n")

        source = scratch / "expired-signer-source"
        make_rsa_source(source)
        build = scratch / "producer-objects"
        build.mkdir()
        cache = scratch / "producer-cache"
        wrapper = Path(__file__).with_name("gpg_fixed_clock.py").resolve()
        wrapper.chmod(0o755)
        environment["DUPLICITY_TEST_GPG_BINARY"] = str(args.gpg.resolve())
        environment["DUPLICITY_TEST_GPG_TIME"] = str(TIMESTAMP)
        options = ["--archive-dir", str(cache), "--name", "reader-fixture-expired-signer",
                   "--volsize", "1", "--gpg-binary", str(wrapper),
                   "--gpg-options=--pinentry-mode=loopback",
                   "--gpg-options=--trust-model=always", "--encrypt-key", fingerprint,
                   "--sign-key", fingerprint]
        backup = [str(args.duplicity), "full"] + options + [
            "--current-time", str(TIMESTAMP), str(source), build.as_uri()
        ]
        run(backup, environment, commands)
        restore = scratch / "expired-signer-restore"
        restore_options = ["--archive-dir", str(cache), "--gpg-binary", str(wrapper),
                           "--gpg-options=--pinentry-mode=loopback",
                           "--gpg-options=--trust-model=always", "--time", str(TIMESTAMP)]
        run([str(args.duplicity), "restore"] + restore_options + [build.as_uri(), str(restore)],
            environment, commands)

        expected_entries = tree(os.fsencode(source))
        restored_entries = tree(os.fsencode(restore))
        if official_comparison(expected_entries) != official_comparison(restored_entries):
            raise RuntimeError("Official RSA-2048 restore differs from the deterministic source")

        manifest_paths = sorted(build.glob("*.manifest.gpg"))
        volume_paths = sorted(build.glob("*.vol*.difftar.gpg"))
        if len(manifest_paths) != 1 or len(volume_paths) != 1:
            raise RuntimeError("Expected exactly one official Full manifest and one data volume")
        manifest_path, volume_path = manifest_paths[0], volume_paths[0]
        if ".vol1." not in volume_path.name:
            raise RuntimeError("Single Full data volume is not volume 1")

        signature_records = {}
        for label, path in [("manifest", manifest_path), ("volume", volume_path)]:
            decoded = scratch / f"{label}.decoded"
            _, status = run_capture(
                [str(args.gpg), "--batch", "--faked-system-time", f"{TIMESTAMP}!",
                 "--pinentry-mode", "loopback", "--passphrase", PASSPHRASE,
                 "--status-fd=2", "--output", str(decoded), "--decrypt", str(path)],
                environment, commands)
            signature_records[label] = valid_signature_status(status, fingerprint, expiry_seconds)
            if signature_records[label]["signature_created_seconds"] > TIMESTAMP:
                raise RuntimeError("A document signature was created after the fixed producer time")

        output.mkdir(parents=True)
        objects_dir = output / "objects"
        objects_dir.mkdir()
        shutil.copyfile(manifest_path, objects_dir / manifest_path.name)
        shutil.copyfile(volume_path, objects_dir / volume_path.name)
        shutil.copytree(test_keys, output / "test-keys")
        (output / "expected.json").write_text(json.dumps({
            "producer": duplicity_version,
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "target": TIMESTAMP,
            "entries": expected_entries,
            "official_restore_verified": True,
            "official_comparison_exclusions": ["symlink_mtime_ns"],
        }, indent=2) + "\n")

        pkesk_output, _ = run_capture(
            [str(args.gpg), "--homedir", str(gnupg_home), "--list-packets", str(volume_path)],
            environment, commands)
        pkesk_match = re.search(
            rb":pubkey enc packet: version (\d+), algo (\d+), keyid ([0-9A-Fa-f]+)",
            pkesk_output)
        if not pkesk_match:
            raise RuntimeError("Cannot independently inspect the volume PKESKv3 packet")
        pkesk_version, pkesk_algorithm_id, recipient_key_id = (
            int(pkesk_match.group(1)), int(pkesk_match.group(2)), pkesk_match.group(3).decode().upper())
        if pkesk_version != 3 or pkesk_algorithm_id != 1:
            raise RuntimeError("Expected PKESKv3 with RSA public-key algorithm ID 1")
        recipient_component = next(
            (component for component in components if component["key_id"] == recipient_key_id), None)
        if recipient_component is None or "e" not in recipient_component["capabilities"].lower():
            raise RuntimeError("PKESK recipient does not identify the exported RSA encryption component")

        decoded_volume = scratch / "decoded-volume.tar"
        decrypt_command = [str(args.gpg), "--batch", "--pinentry-mode", "loopback",
                           "--passphrase", PASSPHRASE, "--status-fd=2", "--output",
                           str(decoded_volume), "--decrypt", str(volume_path)]
        _, decrypt_status = run_capture(decrypt_command, environment, commands)
        decryption_info = next((line.decode("utf-8", "replace") for line in decrypt_status.splitlines()
                                if line.startswith(b"[GNUPG:] DECRYPTION_INFO ")), None)
        if decryption_info is None:
            raise RuntimeError("GnuPG did not report the decrypted payload cipher")
        cipher_id = int(decryption_info.split()[3])
        if cipher_id != 9:
            raise RuntimeError(
                f"Expected this fixture's AES-256 payload cipher (algorithm ID 9), got {cipher_id}"
            )

        secret_packets = run([str(args.gpg), "--list-packets", str(test_keys / "secret.asc")],
                             environment, commands).decode("utf-8", "replace")
        protection_lines = [line.strip() for line in secret_packets.splitlines()
                            if "protect" in line.lower() or "secret key packet" in line.lower()
                            or "secret sub key packet" in line.lower()]
        protection_profiles = [line for line in protection_lines
                               if "iter+salt s2k" in line.lower()]
        if len(protection_profiles) != 2 or any(
            "algo: 7" not in line.lower() or "sha1 protection" not in line.lower()
            or "hash: 2" not in line.lower()
            for line in protection_profiles
        ):
            raise RuntimeError(
                "Secret export is not iterated/salted AES-128 with SHA1 integrity "
                "(the supported PARSE-ENV-009 usage-254 profile)"
            )

        object_records = []
        for path in (objects_dir / manifest_path.name, objects_dir / volume_path.name):
            data = path.read_bytes()
            object_records.append({"key": f"backup/{path.name}", "path": f"objects/{path.name}",
                                   "bytes": len(data), "sha256": sha256(data)})
        source_recipe = Path(__file__)
        oracle_helper = Path(__file__).with_name("generate.py")
        clock_wrapper = Path(__file__).with_name("gpg_fixed_clock.py")
        repository_root = Path(__file__).resolve().parents[2]
        provenance = {
            "producer": duplicity_version,
            "gpg": gpg_version,
            "source_recipe": "tests/producer/generate_expired_signer.py",
            "source_recipe_sha256": sha256(source_recipe.read_bytes()),
            "gpg_clock_wrapper": "tests/producer/gpg_fixed_clock.py",
            "gpg_clock_wrapper_sha256": sha256(clock_wrapper.read_bytes()),
            "gpg_clock": {
                "fixed_signature_time_seconds": TIMESTAMP,
                "key_creation_time_seconds": KEY_CREATED,
                "source": "DUPLICITY_TEST_GPG_TIME injected by checked-in wrapper",
            },
            "oracle_helper": "tests/producer/generate.py",
            "oracle_helper_sha256": sha256(oracle_helper.read_bytes()),
            "reference_command": [
                "python3", "tests/producer/generate_expired_signer.py",
                "--duplicity", "${DUPLICITY_3_0_7}", "--gpg", "${PINNED_GPG}",
                "--expected-version", "3.0.7",
                "--output", "tests/fixtures/duplicity-3.0.7/expired-signer",
            ],
            "commands": [normalized_command(command, scratch, repository_root)
                         for command in commands],
            "timestamp": TIMESTAMP,
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "primary_signer": {
                "fingerprint": fingerprint,
                "key_id": primary["key_id"],
                "algorithm_id": primary["algorithm_id"],
                "bits": primary["bits"],
                "capabilities": primary["capabilities"],
                "created_seconds": primary_created,
                "expires_at_seconds": expiry_seconds,
                "key_expiry_duration_seconds": expiry_evidence["key_expiry_seconds"],
                "expiry_derivation": "primary key creation timestamp plus key-expiration subpacket from primary-issued primary self-signature",
                "self_signature_expiry_evidence": expiry_evidence,
                "check_sigs_exit_status": 0,
                "check_sigs_output": check_sigs_output.decode("utf-8", "replace"),
                "check_sigs_stderr": check_sigs_stderr.decode("utf-8", "replace"),
                "signing_subkey": False,
                "revocation_present": False,
            },
            "document_signatures": signature_records,
            "recipient": {
                "fingerprint": fingerprint,
                "public_key_packet_components": packet_components,
                "primary_component": primary,
                "components": components,
                "pkesk": {"version": pkesk_version, "algorithm_id": pkesk_algorithm_id,
                          "recipient_key_id": recipient_key_id,
                          "recipient_component": recipient_component},
            },
            "payload_cipher": {"name": {7: "AES-128", 8: "AES-192", 9: "AES-256"}[cipher_id],
                               "algorithm_id": cipher_id,
                               "gpg_decryption_status": decryption_info},
            "source_identity_policy": {
                "version_id": "expired-signer-fixture-v1",
                "etag": "lowercase SHA-256 hex of each exact stored object body",
                "scope": "OrderedFixtureSource test adapter only; not a provider ETag claim",
            },
            "secret_export": {
                "path": "test-keys/secret.asc",
                "bytes": len(secret_export),
                "sha256": sha256(secret_export),
                "protected": True,
                "protection_usage": 254,
                "protection_profile": {
                    "s2k": "iterated-and-salted",
                    "s2k_type": 3,
                    "cipher": "AES-128",
                    "cipher_algorithm_id": 7,
                    "integrity": "SHA1",
                    "hash_algorithm_id": 2,
                },
                "protection_packet_lines": protection_lines,
                "credentials_path": "test-keys/credentials.json",
                "s2k_profile_supported_by_parse_env_009": True,
            },
            "objects": object_records,
            "snapshot_oracle": {
                "path": "expected.json",
                "entries": len(expected_entries),
                "official_comparison_exclusions": ["symlink_mtime_ns"],
            },
            "producer_extras_omitted_from_test_source": [
                path.name for path in sorted(build.iterdir())
                if path.name not in (manifest_path.name, volume_path.name)
            ],
        }
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
        gpgconf = args.gpg.with_name("gpgconf")
        subprocess.run([str(gpgconf), "--homedir", str(gnupg_home), "--kill", "gpg-agent"],
                       env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                       check=False)
        print(json.dumps({"output": str(output), "objects": object_records,
                          "primary_signer": provenance["primary_signer"],
                          "document_signatures": signature_records}, indent=2))


if __name__ == "__main__":
    main()

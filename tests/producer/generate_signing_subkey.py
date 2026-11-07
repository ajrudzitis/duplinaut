#!/usr/bin/env python3
"""Generate one official Duplicity 3.0.7 Ed25519 signing-subkey fixture."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from generate import normalize_command, official_comparison, run, tree


PASSPHRASE = "duplicity-reader-signing-subkey-test-fixture"
UID = "Duplicity Signing Subkey Reader Fixture <signing-subkey@example.invalid>"
TIMESTAMP = 1700000000


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def run_capture(argv, environment, commands):
    commands.append(argv)
    result = subprocess.run(argv, env=environment, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {argv!r}\n"
                           + result.stderr.decode("utf-8", "replace"))
    return result.stdout, result.stderr


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
                "capabilities": fields[11] if len(fields) > 11 else "",
            }
            components.append(current)
        elif fields[0] == "fpr" and current is not None and len(fields) > 9:
            current["fingerprint"] = fields[9].upper()
    return components


def make_source(root):
    """Create a small deterministic UTF-8 tree on both macOS and Linux."""
    root.mkdir(mode=0o755)
    root.chmod(0o755)
    (root / "plain.txt").write_bytes(b"signing-subkey reader fixture\n")
    (root / "plain.txt").chmod(0o644)
    (root / "empty.txt").write_bytes(b"")
    (root / "empty.txt").chmod(0o644)
    nested = root / "nested"
    nested.mkdir(mode=0o755)
    nested.chmod(0o755)
    (nested / "payload.bin").write_bytes(bytes(range(256)) * 4)
    (nested / "payload.bin").chmod(0o640)
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


def primary_fingerprint_status(status):
    records = []
    for line in status.decode("utf-8", "replace").splitlines():
        if line.startswith("[GNUPG:] VALIDSIG "):
            fields = line.split()
            # VALIDSIG reports the signing component first and the primary
            # certificate fingerprint in its final optional field.
            if len(fields) < 12:
                raise RuntimeError(f"Incomplete GnuPG VALIDSIG status: {line}")
            records.append({
                "signing_component_fingerprint": fields[2].upper(),
                "primary_fingerprint": fields[-1].upper(),
                "public_key_algorithm_id": int(fields[8]),
                "hash_algorithm_id": int(fields[9]),
                "status_line": line,
            })
    if len(records) != 1:
        raise RuntimeError(f"Expected exactly one VALIDSIG record, found {len(records)}")
    return records[0]


def signature_binding_evidence(packet_text, signing_subkey_key_id, primary_key_id):
    """Require a validly checked signing-subkey binding with an embedded backsig."""
    lines = packet_text.decode("utf-8", "replace").splitlines()
    subkey_at = next((i for i, line in enumerate(lines)
                      if ":public sub key packet:" in line
                      and i + 1 < len(lines)
                      and "algo 22" in lines[i + 1]), None)
    if subkey_at is None:
        raise RuntimeError("Exported public certificate has no Ed25519 public subkey packet")
    next_component = next((i for i in range(subkey_at + 1, len(lines))
                           if ":public sub key packet:" in lines[i]), len(lines))
    block = lines[subkey_at:next_component]
    primary_key_id = primary_key_id.lower()
    binding = next((i for i, line in enumerate(block)
                    if line.startswith(":signature packet:")
                    and f"keyid {primary_key_id}" in line.lower()), None)
    if binding is None:
        raise RuntimeError("Signing-subkey binding signature is not issued by the primary key")
    binding_lines = block[binding:]
    binding_text = "\n".join(binding_lines)
    if "sigclass 0x18" not in binding_text.lower():
        raise RuntimeError("Signing subkey has no subkey-binding signature (class 0x18)")
    embedded = next((line.strip() for line in binding_lines
                     if "subpkt 32" in line.lower() and "0x19" in line.lower()), None)
    if embedded is None:
        raise RuntimeError("Signing-subkey binding has no embedded primary-key-binding signature")
    if signing_subkey_key_id.lower() not in "\n".join(block).lower():
        raise RuntimeError("Inspected Ed25519 subkey block does not match signing subkey key ID")
    return {
        "binding_signature_packet": next(line.strip() for line in binding_lines
                                          if line.startswith(":signature packet:")),
        "binding_class": "0x18",
        "embedded_back_signature_subpacket": embedded,
        "embedded_back_signature_class": "0x19",
        "gpg_check_sigs_exit_status": 0,
    }


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
    with tempfile.TemporaryDirectory(prefix="duplicity-signing-subkey-fixture-", dir="/tmp") as scratch_name:
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
        run(key_arguments + ["--faked-system-time", f"{TIMESTAMP - 1000}!",
                             "--quick-generate-key", UID, "ed25519", "cert", "0"],
            environment, commands)
        initial_listing = run([str(args.gpg), "--with-colons", "--fingerprint",
                               "--list-secret-keys"], environment, commands)
        fingerprint = next(line.split(b":")[9].decode().upper()
                           for line in initial_listing.splitlines()
                           if line.startswith(b"fpr:"))
        run(key_arguments + ["--faked-system-time", f"{TIMESTAMP - 900}!",
                             "--quick-add-key", fingerprint, "ed25519", "sign", "0"],
            environment, commands)
        run(key_arguments + ["--faked-system-time", f"{TIMESTAMP - 800}!",
                             "--quick-add-key", fingerprint, "cv25519", "encrypt", "0"],
            environment, commands)
        listing = run([str(args.gpg), "--with-colons", "--fingerprint",
                       "--list-keys", fingerprint], environment, commands)
        components = parse_components(listing)
        primary = next(component for component in components if component["component"] == "primary")
        signing = [component for component in components
                   if component["component"] == "subkey"
                   and component["algorithm_id"] == 22
                   and "s" in component["capabilities"].lower()]
        encryption = [component for component in components
                      if component["component"] == "subkey"
                      and "e" in component["capabilities"].lower()]
        # GnuPG's primary capability field may include uppercase capabilities
        # aggregated from subkeys; lowercase `s` means the primary itself can
        # sign, while uppercase `S` may be supplied by the signing subkey.
        if primary["algorithm_id"] != 22 or "c" not in primary["capabilities"] \
                or "s" in primary["capabilities"]:
            raise RuntimeError("Primary key is not an Ed25519 certification-only key")
        if len(signing) != 1 or len(encryption) != 1:
            raise RuntimeError("Expected exactly one signing and one encryption subkey")
        signing_subkey = signing[0]
        if len(signing_subkey["fingerprint"]) != 40:
            raise RuntimeError("Signing subkey fingerprint is not v4 SHA-1 width")
        if signing_subkey["fingerprint"] == fingerprint:
            raise RuntimeError("Signing subkey must differ from its primary fingerprint")
        if encryption[0]["fingerprint"] in (fingerprint, signing_subkey["fingerprint"]):
            raise RuntimeError("Encryption subkey must be a separate certificate component")

        test_keys = scratch / "test-keys"
        test_keys.mkdir()
        public_export = run([str(args.gpg), "--armor", "--export", fingerprint], environment, commands)
        (test_keys / "public.asc").write_bytes(public_export)
        secret_export = run(key_arguments + ["--armor", "--export-secret-keys", fingerprint],
                            environment, commands)
        (test_keys / "secret.asc").write_bytes(secret_export)
        (test_keys / "credentials.json").write_text(json.dumps({
            "fingerprint": fingerprint,
            "passphrase": PASSPHRASE,
            "purpose": "public signing-subkey test fixture only",
            "secret_export_protected": True,
        }, indent=2) + "\n")
        packet_listing, packet_stderr = run_capture(
            [str(args.gpg), "--list-packets", "--verbose", str(test_keys / "public.asc")],
            environment, commands)
        secret_packet_listing, _ = run_capture(
            [str(args.gpg), "--list-packets", str(test_keys / "secret.asc")],
            environment, commands)
        secret_protection_lines = [line.strip() for line in
                                   secret_packet_listing.decode("utf-8", "replace").splitlines()
                                   if "protect" in line.lower()
                                   or "secret key packet" in line.lower()
                                   or "secret sub key packet" in line.lower()]
        protected_s2k_lines = [line for line in secret_protection_lines
                               if "iter+salt s2k" in line.lower()]
        if len(protected_s2k_lines) != 3 or any(
            "algo: 7" not in line.lower() or "sha1 protection" not in line.lower()
            or "hash: 2" not in line.lower()
            for line in protected_s2k_lines
        ):
            raise RuntimeError(
                "Secret export must use the reader-supported iterated/salted AES-128/SHA1 profile"
            )
        check_sigs_output, check_sigs_stderr = run_capture(
            [str(args.gpg), "--batch", "--check-sigs", fingerprint], environment, commands)
        binding = signature_binding_evidence(
            packet_listing,
            signing_subkey["key_id"],
            primary["key_id"],
        )

        source = scratch / "signing-subkey-source"
        make_source(source)
        build = scratch / "producer-objects"
        build.mkdir()
        cache = scratch / "producer-cache"
        options = ["--archive-dir", str(cache), "--name", "reader-fixture-signing-subkey",
                   "--volsize", "1", "--gpg-binary", str(args.gpg),
                   "--gpg-options=--pinentry-mode=loopback", "--encrypt-key", fingerprint,
                   "--sign-key", signing_subkey["fingerprint"]]
        run([str(args.duplicity), "full"] + options + [
            "--current-time", str(TIMESTAMP), str(source), build.as_uri()
        ], environment, commands)
        restore = scratch / "signing-subkey-restore"
        restore_options = ["--archive-dir", str(cache), "--gpg-binary", str(args.gpg),
                           "--gpg-options=--pinentry-mode=loopback", "--time", str(TIMESTAMP),
                           "--sign-key", signing_subkey["fingerprint"]]
        run([str(args.duplicity), "restore"] + restore_options + [build.as_uri(), str(restore)],
            environment, commands)
        expected_entries = tree(os.fsencode(source))
        restored_entries = tree(os.fsencode(restore))
        if official_comparison(expected_entries) != official_comparison(restored_entries):
            raise RuntimeError("Official signed-subkey restore differs from deterministic source")

        manifest_paths = sorted(build.glob("*.manifest.gpg"))
        volume_paths = sorted(build.glob("*.vol*.difftar.gpg"))
        if len(manifest_paths) != 1 or len(volume_paths) != 1 \
                or ".vol1." not in volume_paths[0].name:
            raise RuntimeError("Expected exactly one Full manifest and required volume 1")
        output.mkdir(parents=True)
        objects_dir = output / "objects"
        objects_dir.mkdir()
        shutil.copyfile(manifest_paths[0], objects_dir / manifest_paths[0].name)
        shutil.copyfile(volume_paths[0], objects_dir / volume_paths[0].name)
        shutil.copytree(test_keys, output / "test-keys")

        signatures = {}
        for label, path in [("manifest", manifest_paths[0]), ("volume", volume_paths[0])]:
            decoded = scratch / f"{label}.decoded"
            _, status = run_capture(
                [str(args.gpg), "--batch", "--pinentry-mode", "loopback", "--passphrase",
                 PASSPHRASE, "--status-fd=2", "--output", str(decoded), "--decrypt", str(path)],
                environment, commands)
            signature = primary_fingerprint_status(status)
            if signature["signing_component_fingerprint"] != signing_subkey["fingerprint"]:
                raise RuntimeError(f"{label} VALIDSIG did not identify the expected signing subkey")
            if signature["primary_fingerprint"] != fingerprint:
                raise RuntimeError(f"{label} VALIDSIG did not identify the expected primary key")
            if signature["public_key_algorithm_id"] != 22:
                raise RuntimeError(f"{label} signature did not use EdDSA algorithm 22")
            signatures[label] = signature

        object_records = []
        for path in [objects_dir / manifest_paths[0].name, objects_dir / volume_paths[0].name]:
            data = path.read_bytes()
            object_records.append({"key": f"backup/{path.name}", "path": f"objects/{path.name}",
                                   "bytes": len(data), "sha256": sha256(data)})
        source_recipe = Path(__file__)
        oracle_helper = Path(__file__).with_name("generate.py")
        provenance = {
            "producer": duplicity_version,
            "gpg": gpg_version,
            "source_recipe": "tests/producer/generate_signing_subkey.py",
            "source_recipe_sha256": sha256(source_recipe.read_bytes()),
            "oracle_helper": "tests/producer/generate.py",
            "oracle_helper_sha256": sha256(oracle_helper.read_bytes()),
            "reference_command": ["python3", "tests/producer/generate_signing_subkey.py",
                                  "--duplicity", "${DUPLICITY_3_0_7}", "--gpg", "${PINNED_GPG}",
                                  "--expected-version", "3.0.7", "--output",
                                  "tests/fixtures/duplicity-3.0.7/ed25519-signing-subkey"],
            "commands": [normalize_command(command, scratch) for command in commands],
            "timestamp": TIMESTAMP,
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "signer": {
                "primary_fingerprint": fingerprint,
                "primary_component": primary,
                "signing_subkey": signing_subkey,
                "encryption_subkey": encryption[0],
                "binding_evidence": binding,
                "check_sigs_output": check_sigs_output.decode("utf-8", "replace"),
                "check_sigs_stderr": check_sigs_stderr.decode("utf-8", "replace"),
                "public_certificate_packet_listing": packet_listing.decode("utf-8", "replace"),
                "packet_listing_sha256": sha256(packet_listing),
                "packet_listing_stderr": packet_stderr.decode("utf-8", "replace"),
            },
            "document_signatures": signatures,
            "recipient": {"encrypted_to_key_id": encryption[0]["key_id"]},
            "source_identity_policy": {
                "version_id": "ed25519-signing-subkey-fixture-v1",
                "etag": "lowercase SHA-256 hex of each exact stored object body",
                "scope": "OrderedFixtureSource test adapter only; not a provider ETag claim",
            },
            "secret_export": {"path": "test-keys/secret.asc", "bytes": len(secret_export),
                              "sha256": sha256(secret_export), "protected": True,
                              "credentials_path": "test-keys/credentials.json",
                              "protection_packet_lines": secret_protection_lines,
                              "protection_profile": {
                                  "s2k_type": 3, "cipher_algorithm_id": 7,
                                  "hash_algorithm_id": 2, "integrity": "SHA1",
                              }},
            "public_export": {"path": "test-keys/public.asc", "bytes": len(public_export),
                              "sha256": sha256(public_export)},
            "objects": object_records,
            "snapshot_oracle": {"path": "expected.json", "entries": len(expected_entries),
                                "official_comparison_exclusions": ["symlink_mtime_ns"]},
            "producer_extras_omitted_from_test_source": [
                path.name for path in sorted(build.iterdir())
                if path.name not in (manifest_paths[0].name, volume_paths[0].name)
            ],
        }
        (output / "expected.json").write_text(json.dumps({
            "producer": duplicity_version,
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "target": TIMESTAMP,
            "entries": expected_entries,
            "official_restore_verified": True,
            "official_comparison_exclusions": ["symlink_mtime_ns"],
        }, indent=2) + "\n")
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")

        gpgconf = args.gpg.with_name("gpgconf")
        subprocess.run([str(gpgconf), "--homedir", str(gnupg_home), "--kill", "gpg-agent"],
                       env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                       check=False)
        print(json.dumps({"output": str(output), "objects": object_records,
                          "signer": provenance["signer"],
                          "document_signatures": signatures}, indent=2))


if __name__ == "__main__":
    main()

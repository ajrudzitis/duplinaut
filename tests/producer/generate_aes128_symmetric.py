#!/usr/bin/env python3
"""Generate an official one-volume Duplicity 3.0.7 AES-128 fixture."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from generate import PASSPHRASE, TIMES, official_comparison, tree


TIMESTAMP = TIMES[0]
CIPHER_OPTION = "--gpg-options=--cipher-algo=AES128"
PINENTRY_OPTION = "--gpg-options=--pinentry-mode=loopback"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def redact(text):
    if isinstance(text, str):
        return text.replace(PASSPHRASE, "${FIXTURE_PASSPHRASE}")
    return text


def run(argv, environment, commands, *, input_bytes=None):
    commands.append(argv)
    result = subprocess.run(argv, input=input_bytes, env=environment,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        safe_argv = [redact(part) for part in argv]
        raise RuntimeError(
            f"Command failed ({result.returncode}): {safe_argv!r}\n"
            + redact(result.stderr.decode("utf-8", "replace"))
        )
    return result.stdout, result.stderr


def make_source(root):
    """Create an ASCII/UTF-8 source portable to macOS and Linux filesystems."""
    os.makedirs(root, mode=0o755)
    for name, content in [
        (b"plain.txt", b"original\n"),
        (b"nested/metadata.txt", b"symmetric AES-128 fixture\n"),
        ("café.txt".encode("utf-8"), b"unicode path\n"),
    ]:
        path = os.path.join(root, name)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as stream:
            stream.write(content)
        os.chmod(path, 0o644)
    os.symlink(b"plain.txt", os.path.join(root, b"link"))
    for parent, directories, files in os.walk(root):
        for name in directories + files:
            os.utime(os.path.join(parent, name), ns=(TIMESTAMP * 10**9,) * 2,
                     follow_symlinks=False)
        os.utime(parent, ns=(TIMESTAMP * 10**9,) * 2, follow_symlinks=False)


def inspect_object(gpg, path, scratch, environment, commands):
    packet_text, packet_stderr = run(
        [str(gpg), "--batch", "--pinentry-mode", "loopback", "--passphrase", PASSPHRASE,
         "--list-packets", str(path)], environment, commands)
    packet_listing = (packet_text + packet_stderr).decode("utf-8", "replace")
    skesk_packet_count = packet_listing.count(":symkey enc packet:")
    pkesk_packet_count = packet_listing.count(":pubkey enc packet:")
    if skesk_packet_count != 1:
        raise RuntimeError(
            f"{path.name} has {skesk_packet_count} SKESK packets, expected exactly one")
    if pkesk_packet_count != 0:
        raise RuntimeError(
            f"{path.name} has {pkesk_packet_count} PKESK packets, expected none")
    skesk = re.search(
        r":symkey enc packet: version (\d+), cipher (\d+), "
        r"(?:aead (\d+), )?s2k (\d+), hash (\d+)",
        packet_listing,
    )
    if skesk is None:
        raise RuntimeError(f"{path.name} has no inspectable SKESKv4 packet:\n{packet_listing}")
    skesk_version, skesk_cipher, aead_id, s2k_type, s2k_hash = (
        int(value) if value is not None else 0 for value in skesk.groups())
    if skesk_version != 4:
        raise RuntimeError(f"{path.name} uses SKESK version {skesk_version}, expected 4")
    if skesk_cipher != 7:
        raise RuntimeError(f"{path.name} SKESK cipher is {skesk_cipher}, expected AES-128 ID 7")
    if "mdc_method: 2" not in packet_listing or ":encrypted data packet:" not in packet_listing:
        raise RuntimeError(f"{path.name} is not an inspected SEIPDv1/MDC object:\n{packet_listing}")

    decoded = scratch / f"{path.name}.decoded"
    _, stderr = run(
        [str(gpg), "--batch", "--pinentry-mode", "loopback", "--passphrase", PASSPHRASE,
         "--status-fd=2", "--output", str(decoded), "--decrypt", str(path)],
        environment, commands)
    status_text = stderr.decode("utf-8", "replace")
    info_line = next((line for line in status_text.splitlines()
                      if line.startswith("[GNUPG:] DECRYPTION_INFO ")), None)
    if info_line is None:
        raise RuntimeError(f"{path.name} has no DECRYPTION_INFO status:\n{status_text}")
    info_fields = info_line.split()
    cipher_id = int(info_fields[3])
    if cipher_id != 7:
        raise RuntimeError(f"{path.name} payload cipher ID is {cipher_id}, expected AES-128 ID 7")
    if "[GNUPG:] DECRYPTION_OKAY" not in status_text:
        raise RuntimeError(f"{path.name} lacks DECRYPTION_OKAY status:\n{status_text}")
    if "[GNUPG:] GOODMDC" not in status_text:
        raise RuntimeError(f"{path.name} lacks authenticated GOODMDC status:\n{status_text}")
    return {
        "packet_counts": {"pkesk": pkesk_packet_count, "skesk": skesk_packet_count},
        "skesk": {"version": skesk_version, "cipher_algorithm_id": skesk_cipher,
                  "aead_algorithm_id": aead_id, "s2k_type": s2k_type,
                  "s2k_hash_algorithm_id": s2k_hash},
        "seipd": {"version": 1, "packet": "SEIPDv1", "mdc_method": 2,
                  "authenticated_mdc_status": "GOODMDC"},
        "payload_cipher": {"name": "AES-128", "algorithm_id": cipher_id,
                           "decryption_info_status": info_line},
        "decryption_ok_status": "DECRYPTION_OKAY",
        "packet_listing": packet_listing,
        "packet_listing_stderr": packet_stderr.decode("utf-8", "replace"),
        "decryption_status": status_text,
        "decrypted_bytes": decoded.stat().st_size,
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
    with tempfile.TemporaryDirectory(prefix="duplicity-aes128-symmetric-", dir="/tmp") as scratch_name:
        scratch = Path(scratch_name)
        gnupg_home = scratch / "gnupg"
        gnupg_home.mkdir(mode=0o700)
        environment["GNUPGHOME"] = str(gnupg_home)
        environment["XDG_CACHE_HOME"] = str(scratch / "cache-home")
        environment["XDG_CONFIG_HOME"] = str(scratch / "config-home")

        duplicity_version, _ = run([str(args.duplicity), "--version"], environment, commands)
        duplicity_version = duplicity_version.decode().strip()
        if args.expected_version not in duplicity_version.split():
            raise RuntimeError(f"Wrong official producer version: {duplicity_version}")
        gpg_version, _ = run([str(args.gpg), "--version"], environment, commands)
        gpg_version = gpg_version.decode().splitlines()[0]

        source = scratch / "aes128-source"
        make_source(os.fsencode(source))
        build = scratch / "producer-objects"
        build.mkdir()
        cache = scratch / "producer-cache"
        options = ["--archive-dir", str(cache), "--name", "reader-fixture-aes128",
                   "--volsize", "1", "--gpg-binary", str(args.gpg),
                   PINENTRY_OPTION, CIPHER_OPTION]
        backup = [str(args.duplicity), "full"] + options + [
            "--current-time", str(TIMESTAMP), str(source), build.as_uri()]
        run(backup, environment, commands)

        restore = scratch / "aes128-restore"
        restore_options = ["--archive-dir", str(cache), "--gpg-binary", str(args.gpg),
                           PINENTRY_OPTION, CIPHER_OPTION, "--time", str(TIMESTAMP)]
        run([str(args.duplicity), "restore"] + restore_options + [build.as_uri(), str(restore)],
            environment, commands)

        expected_entries = tree(os.fsencode(source))
        restored_entries = tree(os.fsencode(restore))
        if official_comparison(expected_entries) != official_comparison(restored_entries):
            raise RuntimeError("Official AES-128 restore differs from the deterministic source")

        manifests = sorted(build.glob("*.manifest.gpg"))
        volumes = sorted(build.glob("*.vol*.difftar.gpg"))
        if len(manifests) != 1 or len(volumes) != 1 or ".vol1." not in volumes[0].name:
            raise RuntimeError("Expected exactly one Full manifest and required volume 1")
        inspected = {
            "manifest": inspect_object(args.gpg, manifests[0], scratch, environment, commands),
            "volume": inspect_object(args.gpg, volumes[0], scratch, environment, commands),
        }

        output.mkdir(parents=True)
        objects_dir = output / "objects"
        objects_dir.mkdir()
        for path in [manifests[0], volumes[0]]:
            shutil.copyfile(path, objects_dir / path.name)
        test_keys = output / "test-keys"
        test_keys.mkdir()
        (test_keys / "credentials.json").write_text(json.dumps({
            "passphrase": PASSPHRASE,
            "purpose": "public AES-128 symmetric test fixture only",
        }, indent=2) + "\n")
        (output / "expected.json").write_text(json.dumps({
            "producer": duplicity_version,
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "target": TIMESTAMP,
            "entries": expected_entries,
            "official_restore_verified": True,
            "official_comparison_exclusions": ["symlink_mtime_ns"],
        }, indent=2) + "\n")

        object_records = []
        for path in sorted(objects_dir.iterdir()):
            data = path.read_bytes()
            object_records.append({
                "key": f"backup/{path.name}", "path": f"objects/{path.name}",
                "bytes": len(data), "sha256": sha256(data),
                "version_id": "symmetric-aes128-fixture-v1",
            })
        repository_root = Path(__file__).resolve().parents[2]
        helper = Path(__file__).with_name("generate.py")
        provenance = {
            "producer": duplicity_version,
            "gpg": gpg_version,
            "source_recipe": "tests/producer/generate_aes128_symmetric.py",
            "source_recipe_sha256": sha256(Path(__file__).read_bytes()),
            "oracle_helper": "tests/producer/generate.py",
            "oracle_helper_sha256": sha256(helper.read_bytes()),
            "reference_command": [
                "python3", "tests/producer/generate_aes128_symmetric.py",
                "--duplicity", "${DUPLICITY_3_0_7}", "--gpg", "${PINNED_GPG}",
                "--expected-version", "3.0.7", "--output",
                "tests/fixtures/duplicity-3.0.7/aes128-symmetric",
            ],
            "commands": [[str(part).replace(str(scratch), "${WORK}")
                          .replace(str(repository_root), "${REPO_ROOT}")
                          .replace(PASSPHRASE, "${FIXTURE_PASSPHRASE}")
                          for part in command] for command in commands],
            "source_interval": {"start": TIMESTAMP, "end": TIMESTAMP},
            "cipher_selection": {
                "duplicity_gpg_option": CIPHER_OPTION,
                "gnupg_name": "AES128",
                "openpgp_symmetric_algorithm_id": 7,
            },
            "symmetric_fixture": {"credentials_path": "test-keys/credentials.json",
                                  "credential_source": "public synthetic fixture passphrase only",
                                  "objects": inspected},
            "source_identity_policy": {
                "version_id": "symmetric-aes128-fixture-v1",
                "etag": "lowercase SHA-256 hex of each exact stored object body",
                "size": "exact stored object byte length",
                "scope": "OrderedFixtureSource test adapter only; not a provider ETag claim",
            },
            "objects": object_records,
            "snapshot_oracle": {
                "path": "expected.json",
                "entries": len(expected_entries),
                "official_comparison_exclusions": ["symlink_mtime_ns"],
            },
        }
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
        print(json.dumps({"output": str(output), "objects": object_records,
                          "cipher": inspected}, indent=2))


if __name__ == "__main__":
    main()

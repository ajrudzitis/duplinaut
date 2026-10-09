#!/usr/bin/env python3
"""Produce independent source/official-restoration oracles, never reader output.

This generator requires an official duplicity executable and GnuPG. It writes
only a new fixture directory, and keeps source trees/cache/GnuPG state in a
temporary directory. Archive ciphertext and gzip timestamps need not reproduce
byte-for-byte: source content, metadata, and backup timestamps are deterministic.
"""

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tarfile
import tempfile


TIMES = [1700000000, 1700086400, 1700172800]
PASSPHRASE = "duplicity-reader-public-test-fixture"
UID = "Duplicity Reader Fixture <fixture@example.invalid>"


def run(command, environment, commands):
    commands.append(command)
    completed = subprocess.run(command, env=environment, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, check=False)
    if completed.returncode:
        raise RuntimeError(f"Producer failed: {command!r}\n"
                           + completed.stdout.decode("utf-8", "replace"))
    return completed.stdout


def random_bytes(label, length):
    return hashlib.shake_256(label.encode("ascii")).digest(length)


def write(root, name, contents, mode=0o644):
    path = os.path.join(root, name)
    parent = os.path.dirname(path)
    os.makedirs(parent, exist_ok=True)
    with open(path, "wb") as stream:
        stream.write(contents)
    os.chmod(path, mode)


def set_times(root, timestamp):
    for parent, directories, files in os.walk(root):
        for name in directories + files:
            os.utime(os.path.join(parent, name), ns=(timestamp * 10**9,) * 2,
                     follow_symlinks=False)
        os.utime(parent, ns=(timestamp * 10**9,) * 2, follow_symlinks=False)


def make_source(root, large):
    os.makedirs(root, mode=0o755)
    for name, content in [
        (b"plain.txt", b"original\n"),
        (b"metadata.txt", b"unchanged-content\n"),
        (b"revived.txt", b"version-one\n"),
        (b"switch", b"initial-regular-file\n"),
        (b"space quote'\" backslash\\.txt", b"escaped-name\n"),
        (b"line\nbreak\ttab", b"control-name\n"),
        (b"raw-\xff.bin", b"raw-path\n"),
        ("café-λ.txt".encode(), b"unicode-path\n"),
        (b"a!/sibling.txt", b"punctuation\n"),
        (b"a/b/child.txt", b"component-order\n"),
        (b"long/" + b"x" * 180 + b"/" + b"y" * 120, b"long-path\n"),
        (b"empty.txt", b""),
    ]:
        write(root, name, content)
    os.mkdir(os.path.join(root, b"empty-directory"))
    os.symlink(b"plain.txt", os.path.join(root, b"link"))
    if large:
        write(root, b"large.bin", random_bytes("full-large-v1", 2621440))
    set_times(root, TIMES[0] - 100)


def mutate(root, index, large):
    if index == 1:
        write(root, b"plain.txt", b"second\n")
        os.unlink(os.path.join(root, b"revived.txt"))
        os.unlink(os.path.join(root, b"switch"))
        write(root, b"switch/child.txt", b"directory-child\n")
        write(root, b"added.txt", b"new-file\n")
        os.chmod(os.path.join(root, b"metadata.txt"), 0o600)
        if large:
            path = os.path.join(root, b"large.bin")
            with open(path, "r+b") as stream:
                stream.write(random_bytes("incremental-large-v2", 1468006))
    else:
        write(root, b"revived.txt", b"version-three\n", 0o640)
        shutil.rmtree(os.path.join(root, b"switch"))
        os.symlink(b"empty-directory", os.path.join(root, b"switch"))
        os.unlink(os.path.join(root, b"added.txt"))
        write(root, b"plain.txt", b"third\n")
    set_times(root, TIMES[index] - 100)


def tree(root):
    entries = []
    pending = [b""]
    while pending:
        relative = pending.pop()
        path = os.path.join(root, relative)
        info = os.lstat(path)
        kind = "directory" if stat.S_ISDIR(info.st_mode) else (
            "symlink" if stat.S_ISLNK(info.st_mode) else "file")
        entry = {"path_hex": relative.hex(),
                 "components_hex": [p.hex() for p in relative.split(b"/")] if relative else [],
                 "kind": kind, "mode": stat.S_IMODE(info.st_mode) & 0o777,
                 "mtime_ns": info.st_mtime_ns}
        if kind == "directory":
            pending.extend(os.path.join(relative, name) for name in os.listdir(path))
        elif kind == "symlink":
            entry["symlink_target_hex"] = os.readlink(path).hex()
        else:
            entry["size"] = info.st_size
            with open(path, "rb") as stream:
                entry["sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
        entries.append(entry)
    return sorted(entries, key=lambda entry: entry["components_hex"])


def normalize_command(command, temporary_root):
    # Provenance retains complete argv without machine-specific temporary paths.
    return [part.replace(str(temporary_root), "${WORK}") for part in command]


def official_comparison(entries):
    # Duplicity 3.0.7 path.py.copy_attribs deliberately skips symlink utime.
    # Keep source symlink mtimes in the oracle, but don't claim official fidelity.
    return [{key: value for key, value in entry.items()
             if not (entry["kind"] == "symlink" and key == "mtime_ns")}
            for entry in entries]


def observe_tar(data, stored, encrypted, final):
    offset = 0
    types = {}
    while offset + 512 <= len(data) and any(data[offset:offset + 512]):
        header = data[offset:offset + 512]
        flag = header[156:157].hex()
        types[flag] = types.get(flag, 0) + 1
        size = int(header[124:136].rstrip(b"\0 ").lstrip(b" ") or b"0", 8)
        offset += 512 + ((size + 511) // 512) * 512
    assert offset <= len(data), "Official archive has truncated payload"
    tail = data[offset:]
    if encrypted:
        zero_count = 10240 - (offset % 10240)
        assert tail[:zero_count] == bytes(zero_count)
        opaque = tail[zero_count:]
        assert not final or not opaque
        assert opaque == stored[:len(opaque)]
    else:
        assert len(tail) % 512 == 0 and not any(tail), "Unexpected unencrypted tar trailing bytes"
        zero_count = len(tail)
        opaque = b""
    members = list(tarfile.open(fileobj=io.BytesIO(data), mode="r:"))
    return {"entry_stream_bytes": offset, "raw_header_types_hex": types,
            "pax_fields": sorted({key for member in members for key in member.pax_headers}),
            "zero_footer_bytes": zero_count, "topoff_bytes": len(opaque),
            "topoff_equals_stored_prefix": bool(encrypted),
            "multipart": [{"name_hex": os.fsencode(member.name).hex(), "bytes": member.size}
                          for member in members if member.name.startswith("multivol_")]}


def decrypt(gpg, stored, environment):
    result = subprocess.run([gpg, "--batch", "--pinentry-mode", "loopback", "--passphrase", PASSPHRASE, "--decrypt"],
                            input=stored, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", "replace"))
    return result.stdout


def is_data_volume(path):
    return path.name.endswith((".difftar.gz", ".difftar.gpg", ".dt.z", ".dt.g"))


def extract_deltas(build, snapshots):
    delta_directory = build / "deltas"
    delta_directory.mkdir(exist_ok=True)
    records = []
    for snapshot in snapshots:
        if snapshot["mode"] != "plain" or snapshot["index"] == 0:
            continue
        index = snapshot["index"]
        for volume in sorted(build / name for name in snapshot["created_objects"] if is_data_volume(Path(name))):
            with tarfile.open(fileobj=io.BytesIO(gzip.decompress(volume.read_bytes())), mode="r:") as archive:
                for member in archive:
                    if member.name not in ["diff/plain.txt", "diff/metadata.txt"]:
                        continue
                    payload = archive.extractfile(member).read()
                    basename = member.name.removeprefix("diff/").removesuffix(".txt") + f"-inc{index}.rdiff"
                    (delta_directory / basename).write_bytes(payload)
                    records.append({"path": "deltas/" + basename,
                                    "source_object": str(volume.relative_to(build)),
                                    "source_object_sha256": hashlib.sha256(volume.read_bytes()).hexdigest(),
                                    "source_member": member.name, "bytes": len(payload),
                                    "sha256": hashlib.sha256(payload).hexdigest()})
    assert len(records) == 4, "Expected four official standalone delta payloads"
    (delta_directory / "provenance.json").write_text(json.dumps({"producer": "official duplicity", "extractions": records}, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--duplicity", required=True)
    parser.add_argument("--gpg", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--plain-only", action="store_true")
    parser.add_argument("--expected-version", default="3.0.7")
    parser.add_argument("--small", action="store_true", help="Omit incompressible multipart payloads")
    parser.add_argument("--producer-option", action="append", default=[],
                        help="Additional backup/restore option; pass as --producer-option=--option")
    arguments = parser.parse_args()
    if arguments.output.exists():
        raise SystemExit("Fixture output must not already exist")
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    commands = []
    environment = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", PASSPHRASE=PASSPHRASE)
    with tempfile.TemporaryDirectory(prefix="duplicity-reader-producer-") as scratch:
        scratch = Path(scratch)
        environment["GNUPGHOME"] = str(scratch / "gnupg")
        Path(environment["GNUPGHOME"]).mkdir(mode=0o700)
        environment["XDG_CACHE_HOME"] = str(scratch / "cache-home")
        environment["XDG_CONFIG_HOME"] = str(scratch / "config-home")
        version = run([arguments.duplicity, "--version"], environment, commands).decode().strip()
        if arguments.expected_version not in version.split():
            raise RuntimeError(f"Wrong official producer version: {version}")
        gpg_version = run([arguments.gpg, "--version"], environment, commands).decode().splitlines()[0]
        build = scratch / "fixture"
        build.mkdir()
        modes = ["plain"] if arguments.plain_only else ["plain", "symmetric", "public-key", "signed"]
        fingerprint = None
        if not arguments.plain_only:
            key_arguments = [arguments.gpg, "--batch", "--pinentry-mode", "loopback", "--passphrase", PASSPHRASE]
            run(key_arguments + ["--faked-system-time", str(TIMES[0] - 1000),
                                "--quick-generate-key", UID, "ed25519", "sign", "0"], environment, commands)
            keys = run([arguments.gpg, "--with-colons", "--list-secret-keys"], environment, commands)
            fingerprint = next(line.split(b":")[9].decode() for line in keys.splitlines() if line.startswith(b"fpr:"))
            run(key_arguments + ["--faked-system-time", str(TIMES[0] - 900),
                                "--quick-add-key", fingerprint, "cv25519", "encrypt", "0"], environment, commands)
            key_directory = build / "test-keys"
            key_directory.mkdir()
            (key_directory / "public.asc").write_bytes(run([arguments.gpg, "--armor", "--export", fingerprint], environment, commands))
            (key_directory / "secret.asc").write_bytes(run(key_arguments + ["--armor", "--export-secret-keys", fingerprint], environment, commands))
            (key_directory / "credentials.json").write_text(json.dumps({"fingerprint": fingerprint, "passphrase": PASSPHRASE, "purpose": "public test fixtures only"}, indent=2) + "\n")
        oracle = {"schema_version": 1, "producer": version, "snapshots": []}
        for mode in modes:
            root = os.fsencode(scratch / (mode + "-source"))
            make_source(root, large=(mode == "plain" and not arguments.small))
            bucket = build / mode
            bucket.mkdir()
            cache = scratch / (mode + "-archive")
            options = ["--archive-dir", str(cache), "--name", "reader-fixture-" + mode,
                       "--volsize", "1", "--gpg-binary", arguments.gpg]
            if mode == "plain":
                options += ["--no-encryption"]
            else:
                options += ["--gpg-options=--pinentry-mode=loopback"]
                if mode in ["public-key", "signed"]:
                    options += ["--encrypt-key", fingerprint]
                if mode == "signed":
                    options += ["--sign-key", fingerprint]
            options += arguments.producer_option
            for index, timestamp in enumerate(TIMES):
                if index:
                    mutate(root, index, large=(mode == "plain" and not arguments.small))
                expected = tree(root)
                backup = [arguments.duplicity, "full" if index == 0 else "incremental"] + options
                previous_objects = {path.name for path in bucket.iterdir()}
                run(backup + ["--current-time", str(timestamp), os.fsdecode(root), bucket.as_uri()], environment, commands)
                created_objects = [str(path.relative_to(build)) for path in sorted(bucket.iterdir())
                                   if path.name not in previous_objects]
                restore = scratch / (mode + "-restore-" + str(index))
                restore_options = ["--archive-dir", str(cache), "--gpg-binary", arguments.gpg,
                                   "--time", str(timestamp)]
                if mode != "plain":
                    restore_options += ["--gpg-options=--pinentry-mode=loopback"]
                restore_options += arguments.producer_option
                if mode == "signed":
                    restore_options += ["--sign-key", fingerprint]
                run([arguments.duplicity, "restore"] + restore_options + [bucket.as_uri(), str(restore)], environment, commands)
                actual = tree(os.fsencode(restore))
                if official_comparison(expected) != official_comparison(actual):
                    (scratch / "expected.json").write_text(json.dumps(expected, indent=2))
                    (scratch / "actual.json").write_text(json.dumps(actual, indent=2))
                    raise RuntimeError(f"Official restoration differs from source for {mode}/{index}:\n"
                                       + json.dumps({"expected_count": len(expected), "actual_count": len(actual),
                                                     "differences": [(left, right) for left, right in zip(official_comparison(expected), official_comparison(actual)) if left != right][:3]}, indent=2))
                oracle["snapshots"].append({"mode": mode, "time": timestamp,
                                            "index": index, "entries": expected,
                                            "created_objects": created_objects,
                                            "official_restore_verified": True,
                                            "official_comparison_exclusions": ["symlink_mtime_ns"]})
        observations = {"producer": version, "data_volumes": [], "encrypted_boundary_probe": []}
        for mode in modes:
            for path in sorted(path for path in (build / mode).iterdir() if is_data_volume(path)):
                stored = path.read_bytes()
                encrypted = path.suffix in [".gpg", ".g"]
                decoded = decrypt(arguments.gpg, stored, environment) if encrypted else gzip.decompress(stored)
                # All checked-in encrypted mode sets use one final volume.
                observation = observe_tar(decoded, stored, encrypted, final=encrypted)
                observations["data_volumes"].append({"path": str(path.relative_to(build)), **observation})
        if not arguments.plain_only:
            probe_source = scratch / "encrypted-boundary-source"
            probe_source.mkdir()
            write(os.fsencode(probe_source), b"probe.bin", random_bytes("encrypted-boundary", 1310720))
            set_times(os.fsencode(probe_source), TIMES[0] - 100)
            probe_bucket = scratch / "encrypted-boundary-bucket"
            probe_bucket.mkdir()
            run([arguments.duplicity, "full", "--archive-dir", str(scratch / "probe-cache"),
                 "--current-time", str(TIMES[0]), "--volsize", "1", "--encrypt-key", fingerprint,
                 "--gpg-binary", arguments.gpg, "--gpg-options=--pinentry-mode=loopback",
                 str(probe_source), probe_bucket.as_uri()], environment, commands)
            volumes = sorted(probe_bucket.glob("*.difftar.gpg"))
            assert len(volumes) >= 2, "Encrypted boundary probe did not split"
            for index, path in enumerate(volumes):
                stored = path.read_bytes()
                observed = observe_tar(decrypt(arguments.gpg, stored, environment), stored, encrypted=True,
                                       final=(index == len(volumes) - 1))
                observations["encrypted_boundary_probe"].append({"basename": path.name, **observed})
            assert observations["encrypted_boundary_probe"][0]["topoff_bytes"] > 0
        (build / "format-observations.json").write_text(json.dumps(observations, indent=2) + "\n")
        extract_deltas(build, oracle["snapshots"])
        (build / "expected.json").write_text(json.dumps(oracle, indent=2) + "\n")
        object_records = []
        for path in sorted(build.rglob("*")):
            if path.is_file():
                object_records.append({"path": str(path.relative_to(build)), "bytes": path.stat().st_size,
                                       "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        provenance = {"producer": version, "gpg": gpg_version,
                      "duplicity_executable": arguments.duplicity, "gpg_executable": arguments.gpg,
                      "source_recipe": "tests/producer/generate.py",
                      "source_recipe_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "times": TIMES,
                      "profile": {"small": arguments.small, "plain_only": arguments.plain_only,
                                  "producer_options": arguments.producer_option},
                      "source_deterministic": True, "archive_byte_reproducible": False,
                      "commands": [normalize_command(command, scratch) for command in commands],
                      "objects": object_records}
        (build / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
        shutil.copytree(build, arguments.output)
        print(json.dumps({"output": str(arguments.output), "snapshots": len(oracle["snapshots"]),
                          "objects": len(object_records), "bytes": sum(item["bytes"] for item in object_records)}, indent=2))


if __name__ == "__main__":
    main()

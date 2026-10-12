#!/usr/bin/env python3
"""Audit frozen official fixture bytes, source oracles, and boundary observations.

Uses Python's standard-library gzip/tar readers independently of project Rust.
It never extracts files or changes archives. This is fixture evidence, not proof.
"""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import tarfile

ROOT = Path(__file__).resolve().parent


def manifest_records(data):
    records = []
    for match in re.finditer(rb"(?:^|\n)Volume (\d+):\n(.*?)(?=\nVolume |\nFilelist |\Z)", data, re.S):
        number = int(match[1])
        body = match[2]
        start = re.search(rb"StartingPath\s+(\S+)(?:[ \t]+([0-9]+))?", body)
        end = re.search(rb"EndingPath\s+(\S+)(?:[ \t]+([0-9]+))?", body)
        digest = re.search(rb"Hash SHA1 ([0-9a-fA-F]{40})", body)
        assert start and end and digest
        records.append({"number": number, "start": start[1], "start_block": start[2],
                        "end": end[1], "end_block": end[2], "sha1": digest[1].decode().lower()})
    assert records
    normalized = {record["number"]: record for record in records}
    normalized = {number: record for number, record in normalized.items() if number <= records[-1]["number"]}
    assert sorted(normalized) == list(range(1, len(normalized) + 1))
    return records, normalized


def main():
    package = json.loads((ROOT / "package-provenance.json").read_text())
    for artifact in package["artifacts"]:
        path = ROOT / artifact["path"]
        data = path.read_bytes()
        assert len(data) == artifact["bytes"]
        assert hashlib.sha256(data).hexdigest() == artifact["sha256"], artifact["path"]
    profiles = json.loads((ROOT / "profiles.json").read_text())["profiles"]
    for profile in profiles:
        directory = ROOT / profile["profile"]
        provenance = json.loads((directory / "provenance.json").read_text())
        for artifact in provenance["objects"]:
            data = (directory / artifact["path"]).read_bytes()
            assert len(data) == artifact["bytes"] and hashlib.sha256(data).hexdigest() == artifact["sha256"]
        snapshot = json.loads((directory / "expected.json").read_text())["snapshots"][0]
        assert snapshot["official_restore_verified"]
        expected = {bytes.fromhex(entry["path_hex"]): {key: value for key, value in entry.items()
                    if key not in ["path_hex", "components_hex"]} for entry in snapshot["entries"]}
        for name, entry in expected.items():
            if entry["kind"] == "regular":
                assert re.fullmatch(rb"f[0-9]{3}", name) and entry["size"] == 65500
                independent = hashlib.shake_256(f"file-{int(name[1:])}".encode("ascii")).digest(entry["size"])
                assert hashlib.sha256(independent).hexdigest() == entry["sha256"]
        manifest = next((directory / "objects").glob("*.manifest"))
        raw, normalized = manifest_records(manifest.read_bytes())
        actual = {}
        seen = set()
        data_volumes = {}
        for path in (directory / "objects").glob("*.difftar.gz"):
            number = int(re.search(r"vol([0-9]+)\.", path.name)[1])
            data_volumes[number] = path
        assert set(data_volumes) == set(normalized)
        for number in sorted(data_volumes):
            path = data_volumes[number]
            assert hashlib.sha1(path.read_bytes()).hexdigest() == normalized[number]["sha1"]
            decoded = gzip.decompress(path.read_bytes())
            if not decoded:
                assert profile["profile"] == "empty-tail" and number == 2
                assert normalized[number]["start"] == b"." and normalized[number]["end"] == b"f015"
                continue
            with tarfile.open(fileobj=io.BytesIO(decoded), mode="r:") as archive:
                members = list(archive)
                stream_end = max(member.offset + 512 + ((member.size + 511) // 512) * 512 for member in members)
                assert stream_end == len(decoded), "These queued fixtures require no footer bytes"
                for member in members:
                    assert member.name.startswith("snapshot/")
                    name = b"" if member.name == "snapshot/." else os.fsencode(member.name[len("snapshot/"):])
                    assert name not in seen
                    seen.add(name)
                    entry = {"kind": "directory" if member.isdir() else "regular",
                             "mode": member.mode & 0o7777, "mtime_ns": int(member.mtime) * 10**9}
                    if member.isfile():
                        payload = archive.extractfile(member).read()
                        entry.update(size=len(payload), sha256=hashlib.sha256(payload).hexdigest())
                    else:
                        assert member.isdir()
                    actual[name] = entry
                last = os.fsencode(members[-1].name[len("snapshot/"):])
                assert normalized[number]["end"] == last and normalized[number]["end_block"] is None
                if profile["profile"] in ["one-entry", "two-entry"] and number == 2:
                    assert [record["number"] for record in raw] == [1, 2, 2]
                    start = b"." if len(members) == 1 else os.fsencode(members[1].name[len("snapshot/"):])
                    assert normalized[number]["start"] == start and normalized[number]["start_block"] is None
                    assert members[0].name == "snapshot/f016"
        assert actual == expected, profile["profile"]
        print(f"{profile['profile']}: {len(expected)-1} files; bytes, source recipe, metadata, normalized digests, boundaries passed")
    print(f"Package verified: {len(package['artifacts'])} SHA-256-pinned artifacts; no project-reader execution")


if __name__ == "__main__":
    main()

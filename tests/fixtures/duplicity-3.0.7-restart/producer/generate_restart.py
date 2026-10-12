#!/usr/bin/env python3
"""Produce official duplicity 3.0.7 restart/empty-tail archives and source oracles.

The controlled source omission forces the official missing-index restart path.
No producer code, manifest, archive, or expected data is hand-patched. Archive
bytes vary with gzip wall timestamps/hostname/Localdir; source bytes are fixed.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

TIMESTAMP = 1700000000
SOURCE_MTIME = 1699999900
SIZE = 65500


def entries(root):
    result = []
    for item in [root, *sorted(root.iterdir())]:
        name = b"" if item == root else os.fsencode(item.name)
        st = item.stat()
        result.append({"path_hex": name.hex(), "components_hex": [] if not name else [name.hex()],
                       "kind": "directory" if item.is_dir() else "regular", "mode": stat.S_IMODE(st.st_mode),
                       "mtime_ns": st.st_mtime_ns,
                       **({"size": st.st_size, "sha256": hashlib.sha256(item.read_bytes()).hexdigest()} if item.is_file() else {})})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--duplicity", required=True)
    parser.add_argument("--gpg", required=True)
    parser.add_argument("--profile", choices=["one-entry", "two-entry", "empty-tail"], required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise RuntimeError("Output must not exist")
    args.output.mkdir(parents=True)
    commands = []
    with tempfile.TemporaryDirectory(prefix="duplicity-restart-producer-") as temporary:
        work = Path(temporary)
        source = work / "source"
        source.mkdir(mode=0o755)
        count = 16 if args.profile == "empty-tail" else 200
        for i in range(count):
            path = source / f"f{i:03d}"
            path.write_bytes(hashlib.shake_256(f"file-{i}".encode("ascii")).digest(SIZE))
            os.chmod(path, 0o644)
            os.utime(path, (SOURCE_MTIME,) * 2)
        os.utime(source, (SOURCE_MTIME,) * 2)
        environment = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", XDG_CACHE_HOME=str(work / "cache-home"),
                           XDG_CONFIG_HOME=str(work / "config-home"))
        def run(command):
            commands.append(command)
            result = subprocess.run(command, env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            if result.returncode:
                raise RuntimeError((command, result.returncode, result.stdout.decode("utf-8", "replace")))
            return result.stdout
        version = run([args.duplicity, "--version"]).decode().strip()
        if "3.0.7" not in version.split():
            raise RuntimeError("This controlled recipe requires official duplicity 3.0.7")
        objects = args.output / "objects"
        objects.mkdir()
        common = ["--no-encryption", "--volsize", "1", "--concurrency", "0", "--archive-dir", str(work / "cache"),
                  "--name", "restart-fixture", "--gpg-binary", args.gpg, "--current-time", str(TIMESTAMP)]
        backup = [args.duplicity, "full", *common, str(source), objects.resolve().as_uri()]
        retained = missing_name = None
        if args.profile == "empty-tail":
            (args.output / "backup.log").write_bytes(run(backup))
        else:
            commands.append(backup)
            with (args.output / "initial-backup.log").open("wb") as logfile:
                process = subprocess.Popen(backup, env=environment, stdout=logfile, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 30
                while process.poll() is None and time.monotonic() < deadline:
                    if any(objects.glob("*.vol2.difftar.gz")):
                        os.kill(process.pid, signal.SIGSTOP)
                        os.kill(process.pid, signal.SIGKILL)
                        break
                    time.sleep(0.001)
                if process.poll() is None:
                    process.kill()
                process.wait()
            if process.returncode != -signal.SIGKILL or list(objects.glob("*.manifest")):
                raise RuntimeError("Did not establish the intended interrupted partial producer state")
            volumes = list(objects.glob("*.difftar.gz"))
            retained = max(int(re.search(r"vol(\d+)\.", path.name)[1]) for path in volumes) - 1
            partial = next((work / "cache").rglob("*.manifest.part")).read_bytes()
            (args.output / "partial-manifest.original").write_bytes(partial)
            records = re.findall(rb"Volume (\d+):\n\s*StartingPath[^\n]*\n\s*EndingPath\s+(\S+)", partial)
            missing_name = next(path.decode("ascii") for number, path in reversed(records) if int(number) == retained)
            if not re.fullmatch(r"f\d{3}", missing_name):
                raise RuntimeError("Interrupted source boundary is not the intended ordinary file")
            missing = source / missing_name
            saved = missing.read_bytes()
            missing.unlink()
            following = 1 if args.profile == "one-entry" else 2
            for path in list(source.iterdir()):
                if int(path.name[1:]) > int(missing_name[1:]) + following:
                    path.unlink()
            os.utime(source, (SOURCE_MTIME,) * 2)
            (args.output / "resume-backup.log").write_bytes(run(backup))
            missing.write_bytes(saved)
            os.chmod(missing, 0o644)
            os.utime(missing, (SOURCE_MTIME,) * 2)
            os.utime(source, (SOURCE_MTIME,) * 2)
        expected = entries(source)
        restore = work / "restored"
        restore_command = [args.duplicity, "restore", "--no-encryption", "--archive-dir", str(work / "restore-cache"),
                           "--gpg-binary", args.gpg, objects.resolve().as_uri(), str(restore)]
        (args.output / "restore.log").write_bytes(run(restore_command))
        if expected != entries(restore):
            raise RuntimeError("Official restored tree differs from independent source oracle")
        if args.profile == "empty-tail":
            volumes = list(objects.glob("*.difftar.gz"))
            if not any(gzip.decompress(path.read_bytes()) == b"" for path in volumes):
                raise RuntimeError("The producer did not emit an empty trailing volume")
        (args.output / "expected.json").write_text(json.dumps({"schema_version": 1, "producer": version,
            "snapshots": [{"mode": "plain", "time": TIMESTAMP, "index": 0, "entries": expected,
                           "official_restore_verified": True}]}, indent=2) + "\n")
        normalized = [[str(arg).replace(str(work), "${WORK}").replace(str(args.output.resolve()), "${OUTPUT}")
                       .replace(args.duplicity, "${DUPLICITY}").replace(args.gpg, "${GPG}") for arg in command]
                      for command in commands]
        (args.output / "run.json").write_text(json.dumps({"producer": version, "profile": args.profile,
            "commands": normalized, "retained_volume": retained, "transient_omitted_path": missing_name,
            "controlled_interruption": args.profile != "empty-tail", "source_recipe_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "official_restore_verified": True, "archive_byte_reproducible": False}, indent=2) + "\n")
        print(json.dumps({"profile": args.profile, "files": len(expected) - 1, "official_restore_verified": True}))


if __name__ == "__main__":
    main()

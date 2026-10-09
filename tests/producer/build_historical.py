#!/usr/bin/env python3
"""Build hash-pinned official producer sources in a selected isolated Nix shell.

This builds each release's own librsync extension directly, avoiding obsolete
setuptools commands. It makes no changes to upstream Python/C producer code.
Network downloads are restricted to fixed official PyPI distribution URLs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import sysconfig
import tarfile
import urllib.request

RELEASES = {
    "0.8.23": ("https://files.pythonhosted.org/packages/fc/cf/4c492053398633154062759b1b8f7a4da77e029145c432ecfe3f76ca730c/duplicity-0.8.23.tar.gz", "35e0d218bc6569aa99e51795c5b521093af21283af8c4db4f20652e76c20fefd"),
    "2.1.3": ("https://files.pythonhosted.org/packages/f4/83/b4e4e97e190af174c3275a69452455746939cd743f225dcb1bbd66a6e2f6/duplicity-2.1.3.tar.gz", "678b88c812cbab1a3a1fd34f7f18f86fd48ca9fa8b16bc4c74bbdce14b3a5a00"),
    "3.2.1": ("https://files.pythonhosted.org/packages/a8/31/94fac806c15bb2388f09c338f5f75a023c10eceffb496c18eec682976078/duplicity-3.2.1.tar.gz", "4284e3d8f0082a85d3dcfa634901532a1087463df7a33ead417670c0d63b9ef5"),
}
NIXPKGS_REVISION = "774debe7a0d1b496e35677ad955a1011c6ff74f3"


def command_output(command):
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return {"argv": command, "status": result.returncode,
            "output": result.stdout.decode("utf-8", "replace")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--gpg", type=Path, required=True)
    parser.add_argument("--version", action="append", choices=RELEASES)
    args = parser.parse_args()
    args.work = args.work.resolve()
    args.gpg = args.gpg.resolve()
    args.work.mkdir(parents=True, exist_ok=True)
    records = []
    for version in args.version or list(RELEASES):
        url, expected = RELEASES[version]
        archive = args.work / f"duplicity-{version}.tar.gz"
        if not archive.exists():
            with urllib.request.urlopen(url) as response:
                archive.write_bytes(response.read())
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if digest != expected:
            raise RuntimeError(f"Source archive digest mismatch for {version}")
        source = args.work / f"duplicity-{version}"
        if source.exists():
            raise RuntimeError(f"Refusing to replace an existing producer source: {source}")
        with tarfile.open(archive) as release:
            release.extractall(args.work, filter="data")
        extension = source / "duplicity" / ("_librsync" + sysconfig.get_config_var("EXT_SUFFIX"))
        command = [shutil.which("gcc"), "-shared", "-fPIC", "-O2",
                   "-I" + sysconfig.get_paths()["include"],
                   str(source / "duplicity/_librsyncmodule.c"), "-lrsync", "-o", str(extension)]
        build = command_output(command)
        record = {"version": version, "source_url": url, "source_sha256": digest,
                  "source_patches": [], "build": build}
        records.append(record)
        if build["status"]:
            raise RuntimeError(build["output"])
        wrapper = args.work / f"producer-{version}"
        entry = ("runpy.run_module('duplicity', run_name='__main__')" if version == "3.2.1"
                 else f"runpy.run_path({str(source / 'bin/duplicity')!r}, run_name='__main__')")
        wrapper.write_text(f"#!{sys.executable}\nimport runpy, sys, os\n"
                           f"sys.path.insert(0, {str(source)!r})\n"
                           f"os.environ['PATH'] = {str(args.gpg.parent)!r} + ':' + os.environ.get('PATH', '')\n"
                           f"{entry}\n")
        wrapper.chmod(0o755)
        record["wrapper_sha256"] = hashlib.sha256(wrapper.read_bytes()).hexdigest()
        record["extension_sha256"] = hashlib.sha256(extension.read_bytes()).hexdigest()
        record["linked_libraries"] = command_output([shutil.which("ldd"), str(extension)])
        record["version_check"] = command_output([str(wrapper), "--version"])
        if record["version_check"]["status"] or version not in record["version_check"]["output"].split():
            raise RuntimeError(record["version_check"]["output"])
        print(record["version_check"]["output"].strip(), flush=True)
    provenance = {"nixpkgs_revision": NIXPKGS_REVISION, "python": sys.executable,
                  "python_version": sys.version, "gpg": str(args.gpg),
                  "compiler": command_output([shutil.which("gcc"), "--version"]),
                  "librsync": command_output([shutil.which("rdiff"), "--version"]),
                  "nix_cflags_compile": os.environ.get("NIX_CFLAGS_COMPILE"),
                  "nix_ldflags": os.environ.get("NIX_LDFLAGS"),
                  "build_recipe_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  "records": records}
    (args.work / "build-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    main()

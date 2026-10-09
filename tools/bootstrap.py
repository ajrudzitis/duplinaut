"""Install pinned development tools; checksums identify downloaded native tools."""
import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tomllib
import urllib.request
import zipfile


def run(*args):
    subprocess.run(args, check=True)


def download(url, digest, destination):
    if not destination.exists() or hashlib.sha256(destination.read_bytes()).hexdigest() != digest:
        with urllib.request.urlopen(url) as response, destination.open("wb") as out:
            shutil.copyfileobj(response, out)
    actual = hashlib.sha256(destination.read_bytes()).hexdigest()
    if actual != digest:
        raise RuntimeError(f"checksum mismatch for {url}: {actual}")


def main():
    pins = tomllib.loads(Path(sys.argv[1]).read_text())
    root = Path(sys.argv[2]).resolve()
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise RuntimeError("The initial native-tool bootstrap supports Linux x86_64 only")
    root.mkdir(parents=True, exist_ok=True)
    run("rustup", "toolchain", "install", pins["rust"], "--profile", "minimal", "--component", "rustfmt", "--component", "clippy")
    host = pins["linux_x86_64"]
    for name in ("verus", "z3"):
        archive = root / f"{name}.zip"
        download(host[f"{name}_url"], host[f"{name}_sha256"], archive)
        with zipfile.ZipFile(archive) as contents:
            # Native release archives are checksum-pinned. Preserve executable modes.
            contents.extractall(root)
            for entry in contents.infolist():
                mode = entry.external_attr >> 16
                if mode:
                    (root / entry.filename).chmod(mode)
    shutil.copy2(root / host["z3_directory"] / "bin" / "z3", root / host["verus_directory"] / "z3")
    for package in ("vest", "duvet"):
        run("cargo", f"+{pins['rust']}", "install", package, "--version", pins[package], "--locked", "--root", str(root))
    run(str(root / host["verus_directory"] / "verus"), "--version")
    run(str(root / host["verus_directory"] / "z3"), "--version")


if __name__ == "__main__":
    main()

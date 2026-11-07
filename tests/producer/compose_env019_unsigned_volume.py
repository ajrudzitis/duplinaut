#!/usr/bin/env python3
"""Build the PARSE-ENV-019 adversarial mixed-envelope reader fixture.

This composes checked-in Duplicity 3.0.7 outputs; it is not a Duplicity backup
profile. The signed manifest is freshly re-encrypted and signed after changing
its one stored-volume digest to the official symmetric volume's digest.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


MANIFEST = "duplicity-full.20231114T221320Z.manifest.gpg"
VOLUME = "duplicity-full.20231114T221320Z.vol1.difftar.gpg"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def run(argv, environment, *, stdin=None):
    result = subprocess.run(argv, env=environment, input=stdin,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {argv!r}\n"
                           + result.stderr.decode("utf-8", "replace"))
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, required=True,
                        help="checked-in tests/fixtures/duplicity-3.0.7 directory")
    parser.add_argument("--gpg", type=Path, required=True,
                        help="explicit GnuPG executable matching the corpus provenance")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.fixture_root.resolve()
    output = args.output.resolve()
    if output.exists():
        raise SystemExit("Output must not already exist")
    signed_manifest_path = root / "signed" / MANIFEST
    signed_volume_path = root / "signed" / VOLUME
    symmetric_volume_path = root / "symmetric" / VOLUME
    secret_path = root / "test-keys" / "secret.asc"
    public_path = root / "test-keys" / "public.asc"
    credentials_path = root / "test-keys" / "credentials.json"
    for path in (signed_manifest_path, signed_volume_path, symmetric_volume_path,
                 secret_path, public_path, credentials_path):
        if not path.is_file():
            raise FileNotFoundError(path)
    credentials = json.loads(credentials_path.read_text())
    passphrase = credentials["passphrase"]
    fingerprint = credentials["fingerprint"]
    if len(fingerprint) != 40 or any(character not in "0123456789ABCDEF" for character in fingerprint):
        raise ValueError("Fixture credentials must contain a 40-character uppercase fingerprint")

    signed_manifest = signed_manifest_path.read_bytes()
    signed_volume = signed_volume_path.read_bytes()
    symmetric_volume = symmetric_volume_path.read_bytes()
    original_signed_sha1 = hashlib.sha1(signed_volume).hexdigest()
    symmetric_sha1 = hashlib.sha1(symmetric_volume).hexdigest()
    if original_signed_sha1 == symmetric_sha1:
        raise RuntimeError("Expected distinct signed and symmetric stored-volume digests")

    output.mkdir(parents=True)
    environment = dict(os.environ, LC_ALL="C", TZ="UTC",
                       GNUPGHOME="", GPG_TTY="")
    commands = []
    with tempfile.TemporaryDirectory(prefix="reader-env019-compose-", dir="/tmp") as temporary:
        temporary = Path(temporary)
        gnupg_home = temporary / "gnupg"
        gnupg_home.mkdir(mode=0o700)
        environment["GNUPGHOME"] = str(gnupg_home)
        plaintext_manifest = temporary / "signed-manifest.txt"
        changed_manifest = temporary / "mixed-manifest.txt"
        final_manifest = output / MANIFEST

        commands.append([str(args.gpg), "--batch", "--import", str(secret_path)])
        run([str(args.gpg), "--batch", "--import", str(secret_path)], environment)
        commands.append([str(args.gpg), "--batch", "--import", str(public_path)])
        run([str(args.gpg), "--batch", "--import", str(public_path)], environment)

        decrypt_argv = [str(args.gpg), "--batch", "--pinentry-mode", "loopback",
                        "--passphrase", passphrase, "--output", str(plaintext_manifest),
                        "--decrypt", str(signed_manifest_path)]
        commands.append(decrypt_argv)
        run(decrypt_argv, environment)
        manifest_bytes = plaintext_manifest.read_bytes()
        old_hash = f"Hash SHA1 {original_signed_sha1}".encode("ascii")
        new_hash = f"Hash SHA1 {symmetric_sha1}".encode("ascii")
        if manifest_bytes.count(old_hash) != 1:
            raise RuntimeError("Signed manifest does not contain exactly one expected volume digest")
        changed = manifest_bytes.replace(old_hash, new_hash, 1)
        if changed == manifest_bytes:
            raise RuntimeError("Volume digest update made no change")
        changed_manifest.write_bytes(changed)

        encrypt_argv = [str(args.gpg), "--batch", "--yes", "--pinentry-mode", "loopback",
                        "--passphrase", passphrase, "--trust-model", "always",
                        "--local-user", fingerprint, "--recipient", fingerprint,
                        "--sign", "--encrypt", "--output", str(final_manifest),
                        str(changed_manifest)]
        commands.append(encrypt_argv)
        run(encrypt_argv, environment)

        signed_tar = temporary / "signed-volume.tar"
        symmetric_tar = temporary / "symmetric-volume.tar"
        for source, decoded, passphrase in (
            (signed_volume_path, signed_tar, credentials["passphrase"]),
            (symmetric_volume_path, symmetric_tar, credentials["passphrase"]),
        ):
            decrypt_volume_argv = [str(args.gpg), "--batch", "--pinentry-mode", "loopback",
                                   "--passphrase", passphrase, "--output", str(decoded),
                                   "--decrypt", str(source)]
            commands.append(decrypt_volume_argv)
            run(decrypt_volume_argv, environment)
        signed_payload_sha256 = sha256(signed_tar.read_bytes())
        symmetric_payload_sha256 = sha256(symmetric_tar.read_bytes())
        if signed_payload_sha256 != symmetric_payload_sha256:
            raise RuntimeError("Signed and symmetric official volumes do not have identical logical tar bytes")
        gpgconf = args.gpg.with_name("gpgconf")
        kill_agent_argv = [str(gpgconf), "--homedir", str(gnupg_home), "--kill", "gpg-agent"]
        commands.append(kill_agent_argv)
        run(kill_agent_argv, environment)

    def normalized(command):
        replacements = [
            (str(output), "${OUTPUT}"),
            (str(root), "${FIXTURE_ROOT}"),
            (str(temporary), "${WORK}"),
            (str(args.gpg.with_name("gpgconf")), "${GPGCONF}"),
            (str(args.gpg), "${GPG}"),
        ]
        return [next((value.replace(old, replacement) for old, replacement in replacements
                      if old in value), value) for value in command]

    final_volume = output / VOLUME
    shutil.copyfile(symmetric_volume_path, final_volume)
    final_manifest_bytes = final_manifest.read_bytes()
    final_volume_bytes = final_volume.read_bytes()

    provenance = {
        "purpose": "PARSE-ENV-019 adversarial reader fixture; not a single ordinary Duplicity output",
        "official_producer": "Duplicity 3.0.7",
        "official_producer_provenance": "../provenance.json",
        "test_credentials_source": "../test-keys/credentials.json",
        "generator": "tests/producer/compose_env019_unsigned_volume.py",
        "generator_sha256": sha256(Path(__file__).read_bytes()),
        "reference_command": [
            "python3", "tests/producer/compose_env019_unsigned_volume.py",
            "--fixture-root", "tests/fixtures/duplicity-3.0.7",
            "--gpg", "${PINNED_GPG}",
            "--output", "tests/fixtures/duplicity-3.0.7/unsigned-data-volume",
        ],
        "transformations": [
            {
                "input": f"signed/{MANIFEST}",
                "operation": "decrypt and verify with checked-in fixture key; replace the single SHA1 volume digest; re-encrypt and re-sign to the same checked-in recipient/signer",
                "original_manifest_plaintext_sha256": sha256(manifest_bytes),
                "changed_manifest_plaintext_sha256": sha256(changed),
                "changed_digest_field": {"algorithm": "SHA1", "from": original_signed_sha1,
                                         "to": symmetric_sha1},
            },
            {
                "input": f"symmetric/{VOLUME}",
                "operation": "copy the official symmetric-encrypted volume unchanged; its decrypted logical tar is byte-identical to the signed control volume",
            },
        ],
        "logical_volume_tar_payload_sha256": {
            "authorized_signed_control": signed_payload_sha256,
            "unsigned_symmetric_volume": symmetric_payload_sha256,
        },
        "objects": [
            {"key": f"backup/{MANIFEST}", "bytes": len(final_manifest_bytes),
             "sha256": sha256(final_manifest_bytes)},
            {"key": f"backup/{VOLUME}", "bytes": len(final_volume_bytes),
             "sha256": sha256(final_volume_bytes)},
        ],
        "input_objects": [
            {"path": f"signed/{MANIFEST}", "bytes": len(signed_manifest),
             "sha256": sha256(signed_manifest)},
            {"path": f"signed/{VOLUME}", "bytes": len(signed_volume),
             "sha256": sha256(signed_volume)},
            {"path": f"symmetric/{VOLUME}", "bytes": len(symmetric_volume),
             "sha256": sha256(symmetric_volume)},
        ],
        "commands": [normalized(command) for command in commands],
    }
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print(json.dumps({"output": str(output), "objects": provenance["objects"]}, indent=2))


if __name__ == "__main__":
    main()

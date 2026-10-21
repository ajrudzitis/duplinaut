"""Reference-only check using public fixture credentials; never reader runtime."""
from pathlib import Path
import tempfile
import subprocess
import hashlib
import json
import re
import sys

repo = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
gpg = '/nix/store/krh2gxql1h8yx1h4l57vvphaw5wf1wqz-gnupg-2.4.9/bin/gpg'
base = repo / 'tests/fixtures/duplicity-3.0.7'
with tempfile.TemporaryDirectory(prefix='duplicity-pipeline-gpg-') as temp:
    home = Path(temp) / 'home'
    home.mkdir(mode=0o700)
    prefix = [gpg, '--homedir', str(home), '--batch', '--pinentry-mode', 'loopback',
              '--passphrase', 'duplicity-reader-public-test-fixture']
    try:
        subprocess.run(prefix + ['--import', str(base / 'test-keys/secret.asc')],
                       capture_output=True, check=True)
        checks = []
        for profile in ['symmetric', 'public-key', 'signed']:
            for manifest in sorted((base / profile).glob('*.manifest.gpg')):
                proc = subprocess.run(prefix + ['--decrypt', str(manifest)],
                                      capture_output=True, check=True)
                sections = re.split(rb'Volume\s+(\d+):', proc.stdout)
                for i in range(1, len(sections), 2):
                    num = int(sections[i])
                    match = re.search(rb'Hash\s+SHA1\s+([0-9a-fA-F]{40})', sections[i+1])
                    assert match
                    volume = manifest.with_name(manifest.name.removesuffix('.manifest.gpg')
                                                + f'.vol{num}.difftar.gpg')
                    stored = volume.read_bytes()
                    decoded = subprocess.run(prefix + ['--decrypt', str(volume)],
                                             capture_output=True, check=True).stdout
                    expected = match[1].decode().lower()
                    actual = hashlib.sha1(stored).hexdigest()
                    plain = hashlib.sha1(decoded).hexdigest()
                    assert expected == actual and expected != plain
                    checks.append({
                        'profile': profile,
                        'manifest': str(manifest.relative_to(base)),
                        'manifest_sha256': hashlib.sha256(manifest.read_bytes()).hexdigest(),
                        'volume': str(volume.relative_to(base)),
                        'volume_sha256': hashlib.sha256(stored).hexdigest(),
                        'declared_sha1': expected, 'stored_sha1': actual,
                        'plaintext_sha1': plain, 'stored_matches': True,
                        'plaintext_differs': True, 'gpg_decrypt_exit': 0})
    finally:
        subprocess.run([str(Path(gpg).with_name('gpgconf')), '--homedir', str(home),
                        '--kill', 'gpg-agent'], capture_output=True, check=False)
assert len(checks) == 9
record = {
    'schema': 1,
    'scope': 'Official reference-only check; no production reader/runtime subprocess dependency',
    'gpg': subprocess.check_output([gpg, '--version'], text=True).splitlines()[0],
    'gpg_executable_sha256': hashlib.sha256(Path(gpg).read_bytes()).hexdigest(),
    'credentials': 'Intentionally public test keys/passphrase in checked-in corpus; not production',
    'checks': checks}
out.write_text(json.dumps(record, indent=2) + '\n')
print(f'Official GnuPG: {len(checks)} manifests hash original stored ciphertext; all differ from decoded plaintext SHA1')

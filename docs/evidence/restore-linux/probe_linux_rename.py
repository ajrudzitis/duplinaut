#!/usr/bin/env python3
"""Independent permission reference probe; never substitutes for Rust tests."""
import errno
import json
import os
from pathlib import Path
import platform
import stat
import tempfile


def mount_identity(path):
    selected = None
    for line in Path('/proc/self/mountinfo').read_text().splitlines():
        before, after = line.split(' - ', 1)
        fields = before.split()
        mount = fields[4].replace('\\040', ' ').replace('\\134', '\\')
        if path == mount or path.startswith(mount.rstrip('/') + '/'):
            if selected is None or len(mount) > len(selected['mountpoint']):
                selected = {'mountpoint': mount, 'device': fields[2],
                            'filesystem_type': after.split()[0]}
    return selected


def main():
    status = Path('/proc/self/status').read_text().splitlines()
    capabilities = next(line.split()[1] for line in status if line.startswith('CapEff:'))
    receipt = {'schema': 1, 'euid': os.geteuid(), 'effective_capabilities_hex': capabilities,
               'kernel': platform.release(), 'python': platform.python_version(),
               'mount': mount_identity('/tmp'), 'observations': [],
               'limitations': ['ordinary rename permission probe, not NOREPLACE evidence',
                               'not production implementation or formal verification',
                               'no hostile other-UID or ACL race demonstrated']}
    if receipt['euid'] == 0 or int(capabilities, 16):
        raise RuntimeError('reference probe requires unprivileged identity without effective capabilities')
    with tempfile.TemporaryDirectory(prefix='duplicity-restore-profile-reference-', dir='/tmp') as tmp:
        os.chmod(tmp, 0o700)
        for profile in ('cross-parent-wrapper', 'private-parent-same-parent'):
            for mode in (0o000, 0o555, 0o400, 0o200, 0o700, 0o777):
                if profile == 'cross-parent-wrapper':
                    wrapper = Path(tmp) / f'wrapper-{mode:o}'
                    wrapper.mkdir(mode=0o700)
                    src = wrapper / 'root'
                else:
                    wrapper = None
                    src = Path(tmp) / f'stage-{mode:o}'
                src.mkdir(mode=0o700)
                os.chmod(src, mode)
                dst = Path(tmp) / f'published-{profile}-{mode:o}'
                outcome = {'profile': profile, 'parent_ordinary_mode': oct(stat.S_IMODE(os.stat(tmp).st_mode)),
                           'root_mode': oct(mode)}
                try:
                    os.rename(src, dst)
                    outcome.update(result='success', observed_mode=oct(stat.S_IMODE(os.stat(dst).st_mode)))
                    actual = dst
                except OSError as error:
                    outcome.update(result='error', errno=error.errno, errno_name=errno.errorcode.get(error.errno))
                    actual = src
                expected_success = profile == 'private-parent-same-parent' or bool(mode & 0o200)
                if (outcome['result'] == 'success') != expected_success:
                    raise RuntimeError(outcome)
                if expected_success and outcome['observed_mode'] != oct(mode):
                    raise RuntimeError(outcome)
                if not expected_success and outcome['errno'] != errno.EACCES:
                    raise RuntimeError(outcome)
                receipt['observations'].append(outcome)
                os.chmod(actual, 0o700)
                actual.rmdir()
                if wrapper is not None:
                    wrapper.rmdir()
    print(json.dumps(receipt, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()

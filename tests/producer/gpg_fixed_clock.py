#!/usr/bin/env python3
"""Exec pinned GnuPG with the fixture's recorded fixed clock."""

import os
import sys


def main():
    gpg = os.environ.get("DUPLICITY_TEST_GPG_BINARY")
    fixed_time = os.environ.get("DUPLICITY_TEST_GPG_TIME")
    if not gpg or not fixed_time or not fixed_time.isdecimal():
        raise SystemExit("fixed-clock GPG wrapper requires binary and decimal time")
    argv = [gpg, "--faked-system-time", f"{fixed_time}!", *sys.argv[1:]]
    os.execv(gpg, argv)


if __name__ == "__main__":
    main()

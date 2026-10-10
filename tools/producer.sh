#!/usr/bin/env bash
# Regenerate reference data only with explicitly selected official tools.
set -euo pipefail

: "${PRODUCER_PYTHON:?Set PRODUCER_PYTHON to Python 3.11 or newer}"
: "${PRODUCER_DUPLICITY:?Set PRODUCER_DUPLICITY to the official duplicity executable}"
: "${PRODUCER_GPG:?Set PRODUCER_GPG to the official GnuPG executable}"
: "${PRODUCER_OUTPUT:?Set PRODUCER_OUTPUT to a new output directory}"

producer_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
producer_args=(
  "$producer_root/tests/producer/generate.py"
  --duplicity "$PRODUCER_DUPLICITY"
  --gpg "$PRODUCER_GPG"
  --expected-version "${PRODUCER_VERSION:-3.0.7}"
  --output "$PRODUCER_OUTPUT"
)
case "${PRODUCER_PROFILE:-full}" in
  full) ;;
  plain) producer_args+=(--plain-only) ;;
  small-plain) producer_args+=(--plain-only --small) ;;
  *) printf '%s\n' 'Unsupported PRODUCER_PROFILE; use full, plain, or small-plain' >&2; exit 1 ;;
esac
exec "$PRODUCER_PYTHON" "${producer_args[@]}"

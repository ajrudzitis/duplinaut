#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
tools_root="${DUPLICITY_TOOLS_DIR:-$repo_root/.tools}"
export DUPLICITY_TOOLS_DIR="$tools_root"
python3 "$repo_root/tools/bootstrap.py" "$repo_root/tools/toolchain.toml" "$tools_root"
printf 'Tools installed. Activate with: source tools/env.sh\n'

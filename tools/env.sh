# Source from the repository root after tools/bootstrap.sh.
if [ -n "${BASH_VERSION:-}" ]; then
  duplicity_env_file="${BASH_SOURCE[0]}"
elif [ -n "${ZSH_VERSION:-}" ]; then
  duplicity_env_file="${(%):-%N}"
else
  printf 'Source tools/env.sh using Bash or Zsh.\n' >&2
  return 1
fi
duplicity_repo_root="$(cd "$(dirname "$duplicity_env_file")/.." && pwd)"
duplicity_tools_root="${DUPLICITY_TOOLS_DIR:-$duplicity_repo_root/.tools}"
export VERUS="${VERUS:-$duplicity_tools_root/verus-x86-linux/verus}"
export CARGO_VERUS="${CARGO_VERUS:-$(dirname "$VERUS")/cargo-verus}"
export VEST="${VEST:-$duplicity_tools_root/bin/vest}"
export DUVET="${DUVET:-$duplicity_tools_root/bin/duvet}"
export PATH="$(dirname "$VERUS"):$duplicity_tools_root/bin:$PATH"
unset duplicity_repo_root duplicity_tools_root duplicity_env_file

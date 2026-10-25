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
case "$(uname -s):$(uname -m)" in
  Linux:x86_64) duplicity_verus_dir=verus-x86-linux ;;
  Darwin:arm64) duplicity_verus_dir=verus-arm64-macos ;;
  *)
    printf 'No pinned Verus tool configured for %s %s.\n' "$(uname -s)" "$(uname -m)" >&2
    return 1
    ;;
esac
export VERUS="${VERUS:-$duplicity_tools_root/$duplicity_verus_dir/verus}"
export CARGO_VERUS="${CARGO_VERUS:-$(dirname "$VERUS")/cargo-verus}"
export VEST="${VEST:-$duplicity_tools_root/bin/vest}"
export DUVET="${DUVET:-$duplicity_tools_root/bin/duvet}"
export PATH="$(dirname "$VERUS"):$duplicity_tools_root/bin:$PATH"
unset duplicity_repo_root duplicity_tools_root duplicity_env_file duplicity_verus_dir

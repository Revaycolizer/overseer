#!/bin/sh
set -eu
# Run from an extracted release; no sudo or shell profile changes.
source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install_dir=${OVERSEER_INSTALL_DIR:-"$HOME/.local/bin"}
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) expected=aarch64-apple-darwin ;;
  Darwin-x86_64) expected=x86_64-apple-darwin ;;
  Linux-x86_64) expected=x86_64-unknown-linux-gnu ;;
  *) echo 'Unsupported platform for this installer.' >&2; exit 1 ;;
esac
[ "$(cat "$source_dir/TARGET")" = "$expected" ] || { echo 'Download the archive matching your platform.' >&2; exit 1; }
(cd "$source_dir" && if command -v sha256sum >/dev/null 2>&1; then sha256sum -c CHECKSUMS.txt; else shasum -a 256 -c CHECKSUMS.txt; fi)
mkdir -p "$install_dir"
# Replace atomically; an existing executable remains usable if copying fails.
staged=$(mktemp "$install_dir/.overseer-tui.XXXXXX")
trap 'rm -f "$staged"' EXIT HUP INT TERM
install -m 755 "$source_dir/overseer-tui" "$staged"
mv -f "$staged" "$install_dir/overseer-tui"
echo "Installed: $install_dir/overseer-tui"
echo 'Run overseer-tui. If the command is missing, add the install directory to PATH.'
echo 'Default PATH setting: export PATH="$HOME/.local/bin:$PATH"'
echo 'Uninstall by removing the executable; your data is retained.'

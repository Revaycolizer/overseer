#!/usr/bin/env bash
# Reliable desktop dev launcher for Overseer.
#
# Fixes two environment problems that break `dx serve --desktop`:
#   1. A stray system rustc (e.g. /usr/bin/rustc 1.75) shadowing the rustup
#      toolchain on PATH -> "rustc X is not supported by the following packages".
#      We put ~/.cargo/bin (the rustup shims, which honor rust-toolchain.toml)
#      first on PATH.
#   2. Snap-confined terminals (VS Code snap) inject GTK/GIO/pixbuf/locale env
#      vars pointing at /snap/... libraries. A natively-built GUI binary then
#      loads snap's old glibc modules and crashes with
#      "undefined symbol: __libc_pthread_init, GLIBC_PRIVATE".
#      We strip those vars so the app uses the system libraries.
#
# Usage:
#   ./scripts/dev-desktop.sh          # dx serve --desktop
#   ./scripts/dev-desktop.sh build    # dx build --desktop
#   ./scripts/dev-desktop.sh <args…>  # passthrough to dx
set -euo pipefail

cd "$(dirname "$0")/.."

# Put rustup shims ahead of any system rustc.
export PATH="$HOME/.cargo/bin:$PATH"

# Strip snap module/library injections (no-op if unset).
unset GTK_PATH GTK_EXE_PREFIX GTK_IM_MODULE_FILE \
      GDK_PIXBUF_MODULE_FILE GDK_PIXBUF_MODULEDIR \
      GIO_MODULE_DIR LOCPATH GSETTINGS_SCHEMA_DIR \
      LD_LIBRARY_PATH LD_PRELOAD || true

if ! command -v dx >/dev/null 2>&1; then
  echo "error: 'dx' (Dioxus CLI) not found. Install with: cargo install dioxus-cli" >&2
  exit 1
fi

if [ "$#" -eq 0 ]; then
  exec dx serve --desktop
elif [ "$1" = "build" ]; then
  exec dx build --desktop
else
  exec dx "$@"
fi

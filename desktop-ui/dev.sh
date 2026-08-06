#!/usr/bin/env bash
# Launch the Overseer desktop app OUTSIDE the snap (e.g. VS Code) environment.
#
# Snap injects snap's own glibc/GTK into the dynamic loader via LD_LIBRARY_PATH
# and friends. Those override the system libs and crash native GUI apps with:
#   symbol lookup error: /snap/core20/.../libpthread.so.0: undefined symbol:
#   __libc_pthread_init, version GLIBC_PRIVATE
# Unsetting them makes the app use the system libraries again.
#
# Usage (from desktop-ui/):  ./dev.sh        (runs `pnpm tauri dev`)
#                            ./dev.sh build  (or any tauri subcommand)

unset LD_LIBRARY_PATH LD_PRELOAD \
      GTK_PATH GTK_EXE_PREFIX GTK_IM_MODULE_FILE \
      GDK_PIXBUF_MODULE_FILE GDK_PIXBUF_MODULEDIR \
      GIO_MODULE_DIR LOCPATH GSETTINGS_SCHEMA_DIR

export PATH="$HOME/.cargo/bin:$PATH"

if [ $# -eq 0 ]; then
  exec pnpm tauri dev
else
  exec pnpm tauri "$@"
fi

#!/usr/bin/env bash

set -euo pipefail

VERSION=""
TAG=""
CHANNEL="stable"
SKIP_BUILD="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --tag)
      TAG="$2"
      shift 2
      ;;
    --channel)
      CHANNEL="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD="true"
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

get_cargo_version() {
  local cargo_toml_path="$1"
  grep -E '^version = "[^"]+"' "$cargo_toml_path" | head -n 1 | sed -E 's/^version = "([^"]+)"/\1/'
}

copy_required_item() {
  local source="$1"
  local destination="$2"
  if [[ ! -e "$source" ]]; then
    echo "Required release asset not found: $source" >&2
    exit 1
  fi
  cp -R "$source" "$destination"
}

resolve_first_existing_path() {
  local candidate
  for candidate in "$@"; do
    if [[ -e "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_toml="$repo_root/Cargo.toml"

if [[ -z "$VERSION" ]]; then
  VERSION="$(get_cargo_version "$cargo_toml")"
fi

if [[ -z "$TAG" ]]; then
  TAG="v$VERSION"
fi

dist_root="$repo_root/dist"
release_root="$dist_root/$TAG"
portable_root="$release_root/portable"
archive_name="Overseer-$TAG-linux-x64.tar.gz"
archive_path="$release_root/$archive_name"
manifest_path="$release_root/release-linux-x64.json"
checksums_path="$release_root/SHA256SUMS-linux-x64.txt"
binary_source="$repo_root/target/release/overseer"
tool_scripts_source="$(resolve_first_existing_path \
  "$repo_root/tool_scripts" \
  "$repo_root/src/engine/scripts" \
  "$repo_root/agents-scripts" || true)"
agent_seeds_source="$(resolve_first_existing_path \
  "$repo_root/src/engine/agents" \
  "$repo_root/config/agents" || true)"
release_notes_source="$repo_root/releases/notes/$TAG.md"

if [[ "$SKIP_BUILD" != "true" ]]; then
  echo "[INFO] Building Overseer $TAG for Linux..."
  # Use dx bundle to create an AppImage
  dx bundle --release --platform desktop --package-types "appimage"
fi

# Dioxus 0.7 puts the AppImage in a specific location
appimage_source="$(resolve_first_existing_path \
  "$repo_root/target/dx/overseer/release/linux/bundle/appimage/Overseer.AppImage" \
  "$repo_root/target/dx/overseer/release/linux/app/Overseer.AppImage" || true)"

if [[ -z "$appimage_source" || ! -f "$appimage_source" ]]; then
  echo "Bundled AppImage not found. Expected it in target/dx/..." >&2
  exit 1
fi

rm -rf "$release_root"
mkdir -p "$portable_root"

echo "[INFO] Staging Linux release assets..."
copy_required_item "$appimage_source" "$portable_root/Overseer.AppImage"
chmod +x "$portable_root/Overseer.AppImage"

# We still copy assets and config for reference or if the AppImage needs them externally (though it shouldn't)
# But we skip tool_scripts and agents because they are in the DB.
copy_required_item "$repo_root/assets" "$portable_root/assets"
copy_required_item "$repo_root/config" "$portable_root/config"
copy_required_item "$repo_root/CHANGELOG.md" "$portable_root/CHANGELOG.md"
copy_required_item "$repo_root/README.md" "$portable_root/README.md"

if [[ -f "$release_notes_source" ]]; then
  copy_required_item "$release_notes_source" "$portable_root/$TAG.md"
fi

# Create a simple tarball of the AppImage and its reference assets
tar -C "$portable_root" -czf "$archive_path" .

archive_hash="$(sha256sum "$archive_path" | awk '{print $1}')"
appimage_hash="$(sha256sum "$appimage_source" | awk '{print $1}')"

printf '%s  %s\n' "$archive_hash" "$archive_name" > "$checksums_path"
printf '%s  %s\n' "$appimage_hash" "Overseer.AppImage" >> "$checksums_path"

archive_size="$(stat -c%s "$archive_path")"
appimage_size="$(stat -c%s "$appimage_source")"

cat > "$manifest_path" <<EOF
{
  "product": "Overseer",
  "version": "$VERSION",
  "tag": "$TAG",
  "channel": "$CHANNEL",
  "built_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platform": "linux-x64",
  "assets": [
    {
      "name": "$archive_name",
      "kind": "portable-tarball",
      "sha256": "$archive_hash",
      "size": $archive_size
    },
    {
      "name": "Overseer.AppImage",
      "kind": "appimage",
      "sha256": "$appimage_hash",
      "size": $appimage_size
    }
  ]
}
EOF

echo "[INFO] Linux release ready at $release_root"

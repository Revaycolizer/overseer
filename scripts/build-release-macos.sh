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
manifest_path="$release_root/release-macos-x64.json"
checksums_path="$release_root/SHA256SUMS-macos-x64.txt"

if [[ "$SKIP_BUILD" != "true" ]]; then
  echo "[INFO] Building Overseer $TAG for macOS..."
  # Use dx bundle to create a .dmg package
  dx bundle --release --platform desktop --package-types "dmg"
fi

# Dioxus 0.7 puts the assets in target/dx/overseer/bundle/macos/bundle/[type]/
echo "[INFO] Searching for bundled .dmg..."
dmg_source=$(find "$repo_root/target/dx" -type f -name "*.dmg" | grep -v "latest" | head -n 1 || true)

if [[ -z "$dmg_source" || ! -f "$dmg_source" ]]; then
  echo "[ERROR] Bundled .dmg not found in target/dx/" >&2
  exit 1
fi

rm -rf "$release_root"
mkdir -p "$release_root"

echo "[INFO] Staging macOS .dmg package..."
copy_required_item "$dmg_source" "$release_root/Overseer-$TAG.dmg"

dmg_hash="$(shasum -a 256 "$dmg_source" | awk '{print $1}')"
dmg_size="$(stat -f%z "$dmg_source")"

cat > "$manifest_path" <<EOF
{
  "product": "Overseer",
  "version": "$VERSION",
  "tag": "$TAG",
  "channel": "$CHANNEL",
  "built_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platform": "macos-x64",
  "assets": [
    {
      "name": "Overseer-$TAG.dmg",
      "kind": "dmg",
      "sha256": "$dmg_hash",
      "size": $dmg_size
    }
  ]
}
EOF

# Update checksums
printf '%s  %s\n' "$dmg_hash" "Overseer-$TAG.dmg" > "$checksums_path"

echo "[INFO] macOS release ready at $release_root"

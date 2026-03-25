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
  # Use dx bundle to create ONLY a .deb package
  dx bundle --release --platform desktop --package-types "deb"
fi

# Dioxus 0.7 puts the assets in target/dx/overseer/bundle/linux/bundle/[type]/
# Filenames often include version and arch, e.g., Overseer_0.1.2_amd64.deb
echo "[INFO] Searching for bundled .deb..."
deb_source=$(find "$repo_root/target/dx" -type f -name "*.deb" | grep -v "latest" | head -n 1 || true)

if [[ -z "$deb_source" || ! -f "$deb_source" ]]; then
  echo "[ERROR] Bundled .deb not found in target/dx/" >&2
  exit 1
fi

rm -rf "$release_root"
mkdir -p "$release_root"

echo "[INFO] Staging Linux .deb package..."
copy_required_item "$deb_source" "$release_root/Overseer-$TAG-amd64.deb"

deb_hash="$(sha256sum "$deb_source" | awk '{print $1}')"
deb_size="$(stat -c%s "$deb_source")"

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
      "name": "Overseer-$TAG-amd64.deb",
      "kind": "deb",
      "sha256": "$deb_hash",
      "size": $deb_size
    }
  ]
}
EOF

# Update checksums
printf '%s  %s\n' "$deb_hash" "Overseer-$TAG-amd64.deb" > "$checksums_path"

echo "[INFO] Linux release ready at $release_root"

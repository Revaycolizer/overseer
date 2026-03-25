#!/usr/bin/env bash

set -euo pipefail

VERSION=""
PUSH="false"

COMMIT="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --push)
      PUSH="true"
      shift
      ;;
    --commit)
      COMMIT="true"
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

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_toml="$repo_root/Cargo.toml"

if [[ -z "$VERSION" ]]; then
  VERSION="$(get_cargo_version "$cargo_toml")"
fi

TAG="v$VERSION"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Git tag $TAG already exists." >&2
  exit 1
fi

if [[ "$COMMIT" == "true" ]]; then
  echo "[INFO] Staging release metadata..."
  git add "$cargo_toml"
  if [[ -d "$repo_root/releases/notes" ]]; then
    git add "$repo_root/releases/notes/"
  fi
  
  if ! git diff --cached --quiet; then
    echo "[INFO] Committing release metadata for $TAG..."
    git commit -m "chore: release $TAG"
  else
    echo "[INFO] No metadata changes to commit."
  fi
fi

echo "[INFO] Creating git tag $TAG..."
git tag -a "$TAG" -m "Overseer $TAG"

if [[ "$PUSH" == "true" ]]; then
  echo "[INFO] Pushing changes and tag $TAG to origin..."
  if [[ "$COMMIT" == "true" ]]; then
    git push origin main
  fi
  git push origin "$TAG"
fi

echo "[INFO] Created git tag $TAG"

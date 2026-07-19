#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Overseer Skill Publisher
# Packages a skill directory, computes checksums, and prepares
# it for submission to the official registry.
#
# Usage: ./scripts/publish-skill.sh <skill-dir> [version]
#   <skill-dir>  Path to the skill directory (must contain SKILL.md)
#   [version]    Override version (optional, reads from SKILL.md by default)
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SKILL_DIR="${1:?Usage: publish-skill.sh <skill-dir> [version]}"
VERSION_OVERRIDE="${2:-}"

if [ ! -f "$SKILL_DIR/SKILL.md" ]; then
    echo "❌ SKILL.md not found in $SKILL_DIR"
    exit 1
fi

# Extract metadata from SKILL.md frontmatter
NAME=$(grep -E '^name:' "$SKILL_DIR/SKILL.md" | head -1 | sed 's/^name:[[:space:]]*//' | tr -d '"')
DESCRIPTION=$(grep -E '^description:' "$SKILL_DIR/SKILL.md" | head -1 | sed 's/^description:[[:space:]]*//' | tr -d '"')
VERSION="${VERSION_OVERRIDE:-$(grep -E '^version:' "$SKILL_DIR/SKILL.md" | head -1 | sed 's/^version:[[:space:]]*//' | tr -d '"')}"
AUTHOR=$(grep -E '^author:' "$SKILL_DIR/SKILL.md" | head -1 | sed 's/^author:[[:space:]]*//' | tr -d '"')
TAGS=$(grep -E '^tags:' "$SKILL_DIR/SKILL.md" | head -1 | sed 's/^tags:[[:space:]]*//' | tr -d '[]"' | tr ',' ' ')

if [ -z "$NAME" ]; then
    echo "❌ Could not extract 'name' from SKILL.md frontmatter"
    exit 1
fi

if [ -z "$VERSION" ]; then
    VERSION="1.0.0"
    echo "⚠️  No version found, using $VERSION"
fi

# Create output directory
OUTPUT_DIR="dist/skills/$NAME/$VERSION"
mkdir -p "$OUTPUT_DIR"

# Copy files
echo "📦 Packaging skill: $NAME v$VERSION"
cp -r "$SKILL_DIR/"* "$OUTPUT_DIR/"
rm -f "$OUTPUT_DIR/.gitkeep" 2>/dev/null || true

# Create archive
ARCHIVE_NAME="$NAME-$VERSION.tar.gz"
cd "dist/skills/$NAME"
tar -czf "../$ARCHIVE_NAME" "$VERSION/"
cd - > /dev/null

# Compute checksum
if command -v sha256sum &> /dev/null; then
    CHECKSUM=$(sha256sum "dist/skills/$ARCHIVE_NAME" | cut -d' ' -f1)
elif command -v shasum &> /dev/null; then
    CHECKSUM=$(shasum -a 256 "dist/skills/$ARCHIVE_NAME" | cut -d' ' -f1)
else
    echo "⚠️  No sha256sum found, skipping checksum"
    CHECKSUM="pending"
fi

# Generate index entry
INDEX_FILE="dist/skills/index.json"
mkdir -p "$(dirname "$INDEX_FILE")"
cat > "$INDEX_FILE" << EOF
{
  "skills": [
    {
      "name": "$NAME",
      "description": "$DESCRIPTION",
      "version": "$VERSION",
      "author": "$AUTHOR",
      "tags": [$([ -n "$TAGS" ] && echo "\"$(echo "$TAGS" | sed 's/ /", "/g')\"" || echo "")],
      "download_url": "https://github.com/revaycolizer/overseer-skills/releases/download/v${VERSION}/${ARCHIVE_NAME}",
      "checksum": "$CHECKSUM"
    }
  ]
}
EOF

echo ""
echo "✅ Skill packaged successfully!"
echo ""
echo "   Name:     $NAME"
echo "   Version:  $VERSION"
echo "   Archive:  dist/skills/$ARCHIVE_NAME"
echo "   Checksum: $CHECKSUM"
echo ""
echo "   To submit to the registry:"
echo "   1. Create a GitHub release with the archive"
echo "   2. Open a PR at https://github.com/revaycolizer/overseer-skills"
echo "      to update the index.json with this entry"
echo ""

# Print the index entry for easy copy-paste
echo "─── Index Entry ───"
cat "$INDEX_FILE"
echo "───────────────────"

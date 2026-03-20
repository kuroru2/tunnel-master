#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/release.sh <major|minor|patch>
# Bumps version in all manifest files, generates release notes from
# conventional commits since the last tag, commits, and creates an
# annotated tag.

BUMP_TYPE="${1:-}"
if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
  echo "Usage: $0 <major|minor|patch>"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# --- Read current version from package.json ---
CURRENT=$(grep -m1 '"version"' package.json | sed 's/[^0-9.]//g')
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP_TYPE" in
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  patch) PATCH=$((PATCH + 1)) ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "Bumping $CURRENT → $NEW_VERSION"

# --- Bump version in manifest files ---
sed -i '' "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW_VERSION\"/" package.json
sed -i '' "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json
sed -i '' "s/^version = \"$CURRENT\"/version = \"$NEW_VERSION\"/" src-tauri/Cargo.toml

# --- Generate release notes from commits since last tag ---
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [[ -n "$LAST_TAG" ]]; then
  RANGE="${LAST_TAG}..HEAD"
else
  RANGE="HEAD"
fi

# Collect commits, skip version bump commits and merge commits
FEATS=""
FIXES=""
OTHER=""

while IFS= read -r line; do
  # Strip leading whitespace
  line="$(echo "$line" | sed 's/^ *//')"
  [[ -z "$line" ]] && continue

  # Skip chore: bump version commits
  [[ "$line" =~ ^chore:\ bump\ version ]] && continue
  [[ "$line" =~ ^chore\(release\) ]] && continue

  # Categorize by conventional commit prefix
  if [[ "$line" =~ ^feat ]]; then
    # Strip prefix: feat: , feat(scope):
    msg="$(echo "$line" | sed 's/^feat[^:]*: //')"
    FEATS="${FEATS}- ${msg}\n"
  elif [[ "$line" =~ ^fix ]]; then
    msg="$(echo "$line" | sed 's/^fix[^:]*: //')"
    FIXES="${FIXES}- ${msg}\n"
  elif [[ "$line" =~ ^docs: ]]; then
    # Skip docs commits from release notes
    continue
  else
    OTHER="${OTHER}- ${line}\n"
  fi
done <<< "$(git log "$RANGE" --pretty=format:'%s' --no-merges)"

# Build release notes
NOTES="v${NEW_VERSION}\n"

if [[ -n "$FEATS" ]]; then
  NOTES="${NOTES}\n### Features\n${FEATS}"
fi
if [[ -n "$FIXES" ]]; then
  NOTES="${NOTES}\n### Fixes\n${FIXES}"
fi
if [[ -n "$OTHER" ]]; then
  NOTES="${NOTES}\n### Other\n${OTHER}"
fi

echo ""
echo "--- Release Notes ---"
echo -e "$NOTES"
echo "---------------------"
echo ""

# --- Commit and tag ---
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to ${NEW_VERSION}"
git tag -a "v${NEW_VERSION}" -m "$(echo -e "$NOTES")"

echo "Done! Created tag v${NEW_VERSION}"
echo "Run 'git push && git push --tags' to publish."

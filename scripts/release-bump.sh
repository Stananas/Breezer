#!/usr/bin/env bash
# release-bump.sh — fully automatic versioning + tagging (no manual PR).
#
# Called by version.yml on every push to main touching software/**. If there
# are Conventional-Commit triggers (feat/fix) since the last `breezer-v*` tag:
#   feat → minor, fix → patch
# it bumps the workspace version in the root Cargo.toml, commits, creates the
# annotated tag `breezer-vX.Y.Z` and pushes both. release.yml then builds the
# installers, and the updater picks the release up.
set -euo pipefail
cd "$(dirname "$0")/.."

# HEAD already produced by an auto-bump → nothing to do (avoid loops).
if git log -1 --format=%s | grep -qE '^chore\(release\)'; then
  echo "release commit — skip"
  exit 0
fi

CUR=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[[ -n "$CUR" ]] || { echo "cannot read version from Cargo.toml" >&2; exit 1; }

LAST=$(git tag -l 'breezer-v*' | sort -V | tail -1)
RANGE="HEAD"
[[ -n "$LAST" ]] && RANGE="$LAST..HEAD"
LOG=$(git log --format=%s "$RANGE")

has_minor_patch() { grep -qE '^feat(\(|:| )' <<<"$LOG"; return $?; }
has_patch()      { grep -qE '^fix(\(|:| )'  <<<"$LOG"; return $?; }

IFS=. read -r MAJ MIN PAT <<<"$CUR"
if [[ -z "$LAST" ]]; then
  # First release: from a bare workspace, start a fresh patch bump (0.x).
  NEW="$MAJ.$MIN.$((PAT + 1))"
elif has_minor_patch; then
  NEW="$MAJ.$((MIN + 1)).0"
elif has_patch; then
  NEW="$MAJ.$MIN.$((PAT + 1))"
else
  echo "no feat/fix since $LAST — nothing to release"
  exit 0
fi

sed -i "s/^version = \"$CUR\"$/version = \"$NEW\"/" Cargo.toml
git add Cargo.toml
git commit -q -m "chore(release): v$NEW — auto-bump"
git tag -a "breezer-v$NEW" -m "Breezer v$NEW (auto)"
git push origin "HEAD:main"
git push origin "breezer-v$NEW"
echo "released breezer-v$NEW"
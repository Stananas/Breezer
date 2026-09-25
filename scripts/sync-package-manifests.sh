#!/usr/bin/env bash
# sync-package-manifests.sh — keep the packaging manifests in sync with the
# Cargo workspace version (single source of truth: software/Cargo.toml).
# Mirrors the pattern used by OpenDeezer (scripts/update-package-manifests.sh).
#
#   scripts/sync-package-manifests.sh   # reads version, updates AUR etc.
set -euo pipefail
cd "$(dirname "$0")/.."

VER=$(sed -n 's/^version = "\(.*\)"$/\1/p' software/Cargo.toml | head -1)
[[ -n "$VER" ]] || { echo "cannot read version from software/Cargo.toml" >&2; exit 1; }
echo "Breezer version: $VER"

# --- AUR ---
sed -i "s/^pkgver=.*/pkgver=$VER/" packaging/aur/PKGBUILD
sed -i "s/^pkgver = .*/pkgver = $VER/" packaging/aur/.SRCINFO 2>/dev/null || true
echo "  packaging/aur/PKGBUILD (+ .SRCINFO) -> $VER"

echo "done. (Re-run 'makepkg --printsrcinfo' on Arch to refresh .SRCINFO hashes.)"
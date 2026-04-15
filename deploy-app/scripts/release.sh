#!/usr/bin/env bash
# End-to-end local release for Bishop.
#
# What it does:
#   1. Verifies working tree is clean and on main.
#   2. Bumps the version in package.json + Cargo.toml + tauri.conf.json.
#   3. Builds, signs, and notarizes Bishop.app (Tauri handles the .app).
#   4. Notarizes the DMG wrapper (Tauri skips this; Gatekeeper needs it).
#   5. Staples the notarization ticket to the DMG.
#   6. Verifies the DMG with spctl.
#   7. Commits the version bump, tags v<version>, pushes both.
#   8. Creates a GitHub Release and uploads the DMG.
#
# Prereqs (one-time):
#   - deploy-app/.env.signing filled in.
#   - `gh auth status` clean.
#   - Developer ID cert in login keychain.
#
# Usage:
#   ./scripts/release.sh 0.1.1             # arm64 only
#   ./scripts/release.sh 0.1.1 --intel     # arm64 + x86_64
#   ./scripts/release.sh 0.1.1 --draft     # create release as draft

set -euo pipefail

cd "$(dirname "$0")/.."

# -------- args --------
if [ $# -lt 1 ]; then
  echo "usage: $0 <version> [--intel] [--draft]" >&2
  exit 1
fi
VERSION="$1"; shift
WITH_INTEL=0
DRAFT_FLAG=""
for arg in "$@"; do
  case "$arg" in
    --intel) WITH_INTEL=1 ;;
    --draft) DRAFT_FLAG="--draft" ;;
    *) echo "unknown flag: $arg" >&2; exit 1 ;;
  esac
done
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "bad version '$VERSION' — expected semver like 0.1.1 or 0.2.0-rc1" >&2
  exit 1
fi
TAG="v$VERSION"

# -------- preflight --------
echo "→ preflight"
if [ ! -f .env.signing ]; then
  echo "error: .env.signing not found. Copy .env.signing.example first." >&2
  exit 1
fi
set -a; source .env.signing; set +a
for v in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID APPLE_PASSWORD; do
  if [ -z "${!v:-}" ]; then echo "error: $v not set in .env.signing" >&2; exit 1; fi
done

# Must be on main with a clean tree so the version-bump commit lands where we expect.
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
  echo "error: current branch is '$BRANCH', expected 'main'" >&2; exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is dirty — commit or stash first" >&2
  git status --short >&2; exit 1
fi
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "error: tag $TAG already exists" >&2; exit 1
fi

# -------- bump versions --------
echo "→ bumping versions to $VERSION"
# package.json
node -e "const p='./package.json';const f=require('fs');const j=JSON.parse(f.readFileSync(p));j.version='$VERSION';f.writeFileSync(p,JSON.stringify(j,null,2)+'\n');"
# tauri.conf.json
node -e "const p='./src-tauri/tauri.conf.json';const f=require('fs');const j=JSON.parse(f.readFileSync(p));j.version='$VERSION';f.writeFileSync(p,JSON.stringify(j,null,2)+'\n');"
# Cargo.toml — `version = "..."` under [package] only
python3 - <<PY
import re, pathlib
p = pathlib.Path("src-tauri/Cargo.toml")
t = p.read_text()
t2, n = re.subn(r'(\[package\][^\[]*?\nversion\s*=\s*")[^"]+(")', r'\g<1>$VERSION\g<2>', t, count=1, flags=re.DOTALL)
if n != 1:
    raise SystemExit("couldn't locate [package] version in Cargo.toml")
p.write_text(t2)
PY
# Cargo.lock — cargo refreshes it on next build; do a quick metadata touch so
# the commit includes the lockfile update in the same commit.
(cd src-tauri && cargo generate-lockfile >/dev/null 2>&1 || cargo metadata --format-version 1 >/dev/null)

# -------- build helper --------
build_one() {
  local target="$1"
  echo "→ building $target"
  if [ "$target" = "host" ]; then
    pnpm tauri build --bundles dmg
  else
    rustup target list --installed | grep -q "^$target\$" || rustup target add "$target"
    pnpm tauri build --target "$target" --bundles dmg
  fi
}

# Notarize the DMG wrapper (Tauri notarizes the .app but not the .dmg).
notarize_dmg() {
  local dmg="$1"
  echo "→ notarizing wrapper: $(basename "$dmg")"
  xcrun notarytool submit "$dmg" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_PASSWORD" \
    --wait
  xcrun stapler staple "$dmg"
  spctl -a -vv -t open --context context:primary-signature "$dmg"
}

# -------- build(s) --------
DMGS=()
build_one host
HOST_DMG=$(ls -1 src-tauri/target/release/bundle/dmg/Bishop_${VERSION}_*.dmg | head -1)
notarize_dmg "$HOST_DMG"
DMGS+=("$HOST_DMG")

if [ "$WITH_INTEL" = "1" ]; then
  build_one x86_64-apple-darwin
  INTEL_DMG=$(ls -1 src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/Bishop_${VERSION}_*.dmg | head -1)
  notarize_dmg "$INTEL_DMG"
  DMGS+=("$INTEL_DMG")
fi

# -------- commit + tag + release --------
echo "→ committing version bump"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: release $TAG"
git tag -a "$TAG" -m "Bishop $TAG"
git push origin main
git push origin "$TAG"

echo "→ creating GitHub Release"
gh release create "$TAG" \
  ${DRAFT_FLAG:-} \
  --title "Bishop $TAG" \
  --notes "Bishop $TAG — signed and notarized macOS build(s).

Download the DMG, double-click, drag Bishop into Applications. No Gatekeeper warnings.

See [CHANGELOG](../../compare/v0.1.0...$TAG) for what's new." \
  "${DMGS[@]}"

echo ""
echo "✓ release $TAG published."
for d in "${DMGS[@]}"; do echo "  • $(basename "$d")"; done

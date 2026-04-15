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
for v in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID APPLE_PASSWORD \
         TAURI_SIGNING_PRIVATE_KEY_PATH TAURI_SIGNING_PRIVATE_KEY_PASSWORD; do
  if [ -z "${!v:-}" ]; then echo "error: $v not set in .env.signing" >&2; exit 1; fi
done
# Tauri reads TAURI_SIGNING_PRIVATE_KEY (inline) OR ..._PATH. Resolve the path
# and expose the content; covers either usage.
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  KEY_PATH=$(eval echo "$TAURI_SIGNING_PRIVATE_KEY_PATH")
  if [ ! -f "$KEY_PATH" ]; then
    echo "error: updater private key not found at $KEY_PATH" >&2; exit 1
  fi
  export TAURI_SIGNING_PRIVATE_KEY=$(cat "$KEY_PATH")
fi

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
# Building with `--bundles app,dmg` produces both the user-facing DMG and
# the Tauri updater artifact (Bishop.app.tar.gz + .sig) that latest.json points
# at. Skipping the updater bundle here would silently break auto-update.
build_one() {
  local target="$1"
  echo "→ building $target"
  if [ "$target" = "host" ]; then
    pnpm tauri build --bundles app,dmg
  else
    rustup target list --installed | grep -q "^$target\$" || rustup target add "$target"
    pnpm tauri build --target "$target" --bundles app,dmg
  fi
}

# Map a Rust triple → Tauri updater platform key.
tauri_platform() {
  case "$1" in
    host|aarch64-apple-darwin) echo "darwin-aarch64" ;;
    x86_64-apple-darwin)       echo "darwin-x86_64" ;;
    *) echo "unknown-$1" ;;
  esac
}

# Locate the updater artifact for a given target.
updater_artifact_dir() {
  case "$1" in
    host) echo "src-tauri/target/release/bundle/macos" ;;
    *)    echo "src-tauri/target/$1/release/bundle/macos" ;;
  esac
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
# Each target emits: a DMG (user-facing install), and an Updater artifact pair
# (Bishop.app.tar.gz + Bishop.app.tar.gz.sig) that latest.json references.
DMGS=()
UPDATE_ASSETS=()
# Per-platform metadata for building latest.json after all builds finish.
UPDATE_PLATFORMS=()

collect_one() {
  local target="$1"
  local dmg_glob
  if [ "$target" = "host" ]; then
    dmg_glob="src-tauri/target/release/bundle/dmg/Bishop_${VERSION}_*.dmg"
  else
    dmg_glob="src-tauri/target/$target/release/bundle/dmg/Bishop_${VERSION}_*.dmg"
  fi
  local dmg
  dmg=$(ls -1 $dmg_glob | head -1)
  if [ -z "$dmg" ]; then echo "error: no DMG found for $target" >&2; exit 1; fi
  notarize_dmg "$dmg"
  DMGS+=("$dmg")

  local up_dir; up_dir=$(updater_artifact_dir "$target")
  local sig tar
  tar=$(ls -1 "$up_dir"/Bishop.app.tar.gz 2>/dev/null | head -1)
  sig=$(ls -1 "$up_dir"/Bishop.app.tar.gz.sig 2>/dev/null | head -1)
  if [ -z "$tar" ] || [ -z "$sig" ]; then
    echo "error: updater artifact missing for $target (expected Bishop.app.tar.gz[.sig] in $up_dir)" >&2
    exit 1
  fi

  # Rename to avoid collision when both arm64 + x86_64 ship in the same release.
  local plat; plat=$(tauri_platform "$target")
  local renamed_tar="$up_dir/Bishop_${VERSION}_${plat}.app.tar.gz"
  local renamed_sig="$renamed_tar.sig"
  mv "$tar" "$renamed_tar"
  mv "$sig" "$renamed_sig"

  UPDATE_ASSETS+=("$renamed_tar" "$renamed_sig")
  UPDATE_PLATFORMS+=("$plat|$(basename "$renamed_tar")|$(cat "$renamed_sig")")
}

build_one host
collect_one host

if [ "$WITH_INTEL" = "1" ]; then
  build_one x86_64-apple-darwin
  collect_one x86_64-apple-darwin
fi

# -------- latest.json --------
# Tauri's updater fetches this URL, compares versions, and downloads the
# matching platform's tarball. GitHub Releases hosts it as a static asset,
# reachable at https://github.com/<owner>/<repo>/releases/latest/download/latest.json.
MANIFEST=/tmp/bishop-latest-$VERSION.json
PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RELEASE_URL_BASE="https://github.com/panda-zw/bishop/releases/download/$TAG"

# Build the platforms map with jq so embedded signatures don't break JSON.
PLATFORMS_JSON='{}'
for entry in "${UPDATE_PLATFORMS[@]}"; do
  IFS='|' read -r plat fname sig_content <<< "$entry"
  PLATFORMS_JSON=$(jq --arg p "$plat" \
                     --arg url "$RELEASE_URL_BASE/$fname" \
                     --arg sig "$sig_content" \
                     '. + { ($p): { signature: $sig, url: $url } }' \
                     <<< "$PLATFORMS_JSON")
done
jq -n \
  --arg version "$VERSION" \
  --arg notes "Bishop $TAG" \
  --arg pub_date "$PUB_DATE" \
  --argjson platforms "$PLATFORMS_JSON" \
  '{ version: $version, notes: $notes, pub_date: $pub_date, platforms: $platforms }' \
  > "$MANIFEST"

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

Existing installs auto-update from this release." \
  "${DMGS[@]}" "${UPDATE_ASSETS[@]}" "$MANIFEST#latest.json"

rm -f "$MANIFEST"

echo ""
echo "✓ release $TAG published."
for d in "${DMGS[@]}"; do echo "  • $(basename "$d")"; done
for u in "${UPDATE_ASSETS[@]}"; do echo "  • $(basename "$u")"; done
echo "  • latest.json"

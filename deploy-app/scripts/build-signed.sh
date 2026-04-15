#!/usr/bin/env bash
# Build a signed + notarized Bishop DMG.
#
# Prereqs (one-time):
#   1. `security find-identity -v -p codesigning` shows a "Developer ID Application" cert.
#   2. Copy .env.signing.example → .env.signing and fill in APPLE_ID + APPLE_PASSWORD
#      (app-specific password from appleid.apple.com).
#
# Usage:
#   ./scripts/build-signed.sh            # dmg only
#   ./scripts/build-signed.sh app        # .app only
#   ./scripts/build-signed.sh all        # both

set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f .env.signing ]; then
  echo "error: .env.signing not found. Copy .env.signing.example and fill it in." >&2
  exit 1
fi
# shellcheck disable=SC1091
set -a; source .env.signing; set +a

for v in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID APPLE_PASSWORD; do
  if [ -z "${!v:-}" ]; then
    echo "error: $v is not set in .env.signing" >&2
    exit 1
  fi
done

TARGET="${1:-dmg}"
echo "→ building Bishop (target: $TARGET, identity: $APPLE_SIGNING_IDENTITY)"
pnpm tauri build --bundles "$TARGET"

echo "✓ done. DMG at: src-tauri/target/release/bundle/dmg/"

#!/usr/bin/env bash
# Assemble the user-facing `deploy` script from the numbered fragments in
# this directory. Numeric-prefix ordering (`00-header`, `01-colors`, …,
# `99-main`) makes the concatenation order obvious and lets additional
# chunks slot in without renumbering everyone.
#
# Builds to two places so the bundled Tauri resource and the repo-root
# `deploy` stay in sync:
#   - <repo>/deploy
#   - <repo>/deploy-app/src-tauri/resources/deploy
#
# Usage:
#   ./deploy-src/build.sh          # build both targets
#   ./deploy-src/build.sh --check  # verify sources concat-match current deploy

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/.." && pwd)"

# Deterministic concat — sort by filename so ordering is stable regardless of
# mtime / OS glob expansion.
FRAGMENTS=()
while IFS= read -r f; do FRAGMENTS+=("$f"); done < <(find "$HERE" -maxdepth 1 -name '[0-9][0-9]-*.sh' | sort)
if [ ${#FRAGMENTS[@]} -eq 0 ]; then
  echo "no fragments found in $HERE" >&2
  exit 1
fi

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

# shellcheck disable=SC2002
cat "${FRAGMENTS[@]}" > "$TMP"

MODE="${1:-write}"
case "$MODE" in
  --check)
    if ! diff -q "$TMP" "$REPO/deploy" >/dev/null; then
      echo "deploy is out of sync with deploy-src/" >&2
      diff "$REPO/deploy" "$TMP" | head -40 >&2 || true
      exit 1
    fi
    echo "✓ deploy matches deploy-src/"
    ;;
  write)
    install -m 0755 "$TMP" "$REPO/deploy"
    install -m 0755 "$TMP" "$REPO/deploy-app/src-tauri/resources/deploy"
    echo "✓ wrote $REPO/deploy"
    echo "✓ wrote $REPO/deploy-app/src-tauri/resources/deploy"
    ;;
  *) echo "unknown mode: $MODE" >&2; exit 1 ;;
esac

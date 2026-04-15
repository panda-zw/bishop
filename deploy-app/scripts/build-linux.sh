#!/usr/bin/env bash
# Build Linux DEB + AppImage bundles for Bishop inside a Docker container.
#
# Why Docker: Tauri's Linux bundling needs webkit2gtk + patchelf + gtk — not
# available on macOS. Spinning up Ubuntu 22.04 in a container is the cleanest
# way to get a reproducible build from a Mac dev environment.
#
# Artefacts land under src-tauri/target/release/bundle/{deb,appimage}/ on
# the host filesystem (the container mounts the repo read-write).
#
# Usage:
#   ./scripts/build-linux.sh                  # x86_64 only (default)
#   LINUX_ARCH=arm64 ./scripts/build-linux.sh # arm64 (runs natively on M-series)

set -euo pipefail

cd "$(dirname "$0")/.."

IMAGE_NAME="bishop-linux-builder:22.04"
ARCH="${LINUX_ARCH:-x86_64}"
case "$ARCH" in
  x86_64) PLATFORM=linux/amd64; RUST_TARGET=x86_64-unknown-linux-gnu ;;
  arm64)  PLATFORM=linux/arm64; RUST_TARGET=aarch64-unknown-linux-gnu ;;
  *) echo "unsupported LINUX_ARCH '$ARCH' (want x86_64 or arm64)" >&2; exit 1 ;;
esac

# Build the image if we don't already have it for this platform.
# `docker image inspect` short-circuits a rebuild when the image already exists;
# the actual cache lives in Docker's overlay filesystem as usual.
if ! docker image inspect "$IMAGE_NAME" --format '{{.Id}}' >/dev/null 2>&1; then
  echo "→ building $IMAGE_NAME ($PLATFORM)"
  docker buildx build \
    --platform "$PLATFORM" \
    -t "$IMAGE_NAME" \
    -f scripts/linux-builder.Dockerfile \
    --load \
    scripts
fi

# Forward the updater-signing creds into the container so the AppImage
# emits a valid .sig that matches the pubkey in tauri.conf.json.
if [ -f .env.signing ]; then
  set -a; source .env.signing; set +a
fi
UPDATER_KEY_CONTENT="${TAURI_SIGNING_PRIVATE_KEY:-}"
if [ -z "$UPDATER_KEY_CONTENT" ] && [ -n "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]; then
  KEY_PATH=$(eval echo "$TAURI_SIGNING_PRIVATE_KEY_PATH")
  if [ -f "$KEY_PATH" ]; then
    UPDATER_KEY_CONTENT=$(cat "$KEY_PATH")
  fi
fi

echo "→ building Linux bundles ($ARCH)"
docker run --rm \
  --platform "$PLATFORM" \
  -v "$(pwd):/src" \
  -v bishop-cargo-cache:/cargo/registry \
  -v bishop-linux-target:/src/src-tauri/target \
  -e TAURI_SIGNING_PRIVATE_KEY="$UPDATER_KEY_CONTENT" \
  -e TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
  "$IMAGE_NAME" \
  bash -euc "\
    cd /src && \
    rustup target add $RUST_TARGET && \
    pnpm install --frozen-lockfile && \
    pnpm tauri build --target $RUST_TARGET --bundles deb,appimage\
  "

# Copy the built artefacts from the volume into the normal host path so the
# rest of release.sh (and users inspecting src-tauri/target/) can find them.
TARGET_DIR="src-tauri/target/$RUST_TARGET/release/bundle"
mkdir -p "$TARGET_DIR/deb" "$TARGET_DIR/appimage"
docker run --rm \
  -v bishop-linux-target:/target \
  -v "$(pwd)/src-tauri/target/$RUST_TARGET/release/bundle:/out" \
  alpine sh -c "\
    cp -f /target/$RUST_TARGET/release/bundle/deb/*.deb /out/deb/ 2>/dev/null || true; \
    cp -f /target/$RUST_TARGET/release/bundle/appimage/*.AppImage* /out/appimage/ 2>/dev/null || true\
  "

echo "✓ Linux bundles at:"
ls -1 "$TARGET_DIR"/deb/ 2>/dev/null | sed 's/^/  deb\//'
ls -1 "$TARGET_DIR"/appimage/ 2>/dev/null | sed 's/^/  appimage\//'

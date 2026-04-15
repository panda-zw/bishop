# Base image for building Bishop's Linux DEB + AppImage bundles.
#
# Ubuntu 22.04 is chosen deliberately for GLIBC 2.35 — newer bases produce
# binaries that refuse to run on common LTS distros (Ubuntu 22.04, Debian 12,
# still-supported RHEL 9, etc.). Only move this forward when you've consciously
# decided to drop older user systems.

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/cargo \
    RUSTUP_HOME=/rustup \
    PATH=/cargo/bin:/usr/local/bin:/usr/bin:/bin

# ---- system deps -----------------------------------------------------------
# webkit2gtk-4.1 + gtk-3 + ayatana: Tauri 2's Linux UI stack
# librsvg: SVG icon rendering
# patchelf: required by tauri-bundler to produce a relocatable AppImage
# libxdo-dev: global hotkeys / keyboard (pulled in by some Tauri setups)
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      curl \
      file \
      git \
      libayatana-appindicator3-dev \
      libgtk-3-dev \
      librsvg2-dev \
      libssl-dev \
      libwebkit2gtk-4.1-dev \
      libxdo-dev \
      patchelf \
      pkg-config \
      wget \
      xz-utils \
    && rm -rf /var/lib/apt/lists/*

# ---- Rust ------------------------------------------------------------------
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path

# ---- Node + pnpm ----------------------------------------------------------
# Use the NodeSource v20 repo so the Node major matches what devs use locally.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g pnpm@10

# A workdir the caller can mount into.
WORKDIR /src

# Fall through to the caller's command.
CMD ["bash"]

# Bishop

Bishop is a Mac desktop app that lets you deploy web projects to servers and manage their
terminals with one click — no command-line knowledge needed. It's a visual companion to a
small opinionated `./deploy` bash CLI: the CLI does the heavy lifting on the server
(Docker, Traefik, a shared Postgres + Redis, TLS via Let's Encrypt), and Bishop gives you
projects, environments, live container status, deploy history, scaffolding, and a
multi-tab SSH + local terminal in one window.

## What's in this repo

- `deploy` — the bash CLI. Installs Docker + a shared infra stack on a fresh Linux box,
  then deploys any number of apps onto it.
- `deploy-app/` — the Tauri 2 (Rust) + Svelte 5 desktop app that wraps the CLI.

## Quick start (app)

Prereqs: [pnpm](https://pnpm.io/), [Rust](https://rustup.rs/), and the Tauri
[prerequisites for your OS](https://tauri.app/start/prerequisites/).

```sh
cd deploy-app
pnpm install
pnpm tauri dev         # develop with hot reload
pnpm tauri build --bundles dmg   # build a macOS DMG
```

The built DMG lands in `deploy-app/src-tauri/target/release/bundle/dmg/`.

## Cutting a release

Releases are built and signed locally (no CI). One-time setup: copy
`deploy-app/.env.signing.example` → `deploy-app/.env.signing` and fill in your Apple
Developer ID credentials (see the comments in the example file).

```sh
cd deploy-app
./scripts/release.sh 0.1.1            # arm64 only
./scripts/release.sh 0.1.1 --intel    # arm64 + x86_64
./scripts/release.sh 0.1.1 --draft    # publish as draft release
```

The script bumps the version in all manifests, builds + signs + notarizes the DMG
(including the outer DMG wrapper, which `tauri build` alone skips), tags the commit,
pushes, and creates the GitHub Release with the DMG attached.

## Features

- **Multi-tab terminals** — SSH (via your system `ssh` + `~/.ssh/config`, with ControlMaster
  multiplexing) and local shells, with split / grid layouts, persistent tabs across
  restarts, per-host scrollback replay, optional tmux wrapping for true cross-disconnect
  persistence, and live font-size controls.
- **Projects & environments** — one view per project, per-env status dots driven by
  `docker ps` health over SSH.
- **One-click deploy** — runs `./deploy <env>` with a streaming log viewer, deploy history
  (SQLite), and preflight checks that catch missing compose/Dockerfile/shared-infra files
  and known-stale template patterns before they blow up on the server.
- **Inline scaffolding** — generate `docker-compose.prod.yml`, `Dockerfile`,
  `.dockerignore`, and the shared Traefik / Postgres / Redis stack with sensible defaults,
  edit them in-app, then deploy.
- **System tray** — dynamic menu with quick access to projects, environments, saved hosts,
  and `~/.ssh/config` entries.
- **Theme-aware** — dark / light modes with xterm palettes that follow the app theme.

## CLI usage (standalone)

The `deploy` script can be used on its own without the app.

```sh
./deploy setup-server <env>   # installs Docker + shared infra on a fresh box
./deploy setup-app    <env>   # bootstraps this app's config on the server
./deploy <env>                # deploy the current project to <env>
./deploy check        <env>   # health-check the deployed app
```

See `./deploy help` for the full command list.

## Status

Early — usable today, but expect rough edges. PRs welcome.

## License

MIT — see [LICENSE](LICENSE) (once added).

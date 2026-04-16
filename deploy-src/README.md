# deploy-src/

Source fragments for the `deploy` bash CLI. [build.sh](build.sh) concatenates
them into the user-facing `deploy` script at the repo root (and into Bishop's
bundled resource copy under `deploy-app/src-tauri/resources/deploy`).

## Why split

A single 2,300-line bash script is a cliff. Phase 3 of the roadmap (rollback,
zero-downtime swaps, preview environments) piles several hundred more lines on
top. Splitting by concern keeps each chunk small enough to understand in one
sitting and makes new features land as new fragments rather than growing the
monolith.

## Ordering

Fragments are concatenated in filename order:

- `00-header.sh` — shebang, CLI usage block, `set -euo pipefail`
- `01-colors.sh` — ANSI colors + `info` / `warn` / `error` / `ask` helpers
- `02-paths.sh` — `SCRIPT_DIR` / `CONFIG_FILE` / project-config defaults
- `03-prompts.sh` — input cache + `prompt_*` / `confirm` / `pick_option` / `pick_environment`
- `04-remote.sh` — `parse_remote` + the `remote` / `remote_check` SSH wrappers
- `99-main.sh` — scaffolding generators, command implementations, arg parsing, dispatch

New extractions land between `04-remote.sh` and `99-main.sh` with prefixes like
`05-…`, `06-…`. Keep `99-main.sh` last so argument parsing runs after every
function has been defined.

## Workflow

```sh
# After editing any fragment:
./deploy-src/build.sh

# In CI (or as a pre-commit check):
./deploy-src/build.sh --check
```

`--check` fails if the committed `deploy` doesn't match a fresh concat — catch
anyone hand-editing the built file instead of a fragment.

## Don't

- **Don't edit `deploy` directly.** There's a "built artifact" header at the
  top reminding future-you of this.
- **Don't put a shebang in fragments other than `00-header.sh`.** The final
  `deploy` must have exactly one shebang and it must be the first line.

/**
 * Auto-updater store. Checks GitHub Releases for a newer version on app start
 * (and on demand), downloads + verifies the signed DMG in the background, and
 * surfaces the result as reactive state the banner component renders.
 *
 * Design:
 *   - Silent on no-update. Only speaks up when there's something the user can act on.
 *   - Download happens eagerly once an update is found — by the time the user clicks
 *     "Install and restart", there's nothing to wait for.
 *   - Respects "Later" for one app session; re-checks next launch.
 */

import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdaterState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; notes: string | null }
  | { kind: "downloading"; version: string; progress: number } // 0..1
  | { kind: "ready"; version: string; notes: string | null }
  | { kind: "error"; message: string };

function createUpdater() {
  const state = $state({
    status: { kind: "idle" } as UpdaterState,
    dismissed: false,
  });

  let pending: Update | null = null;

  async function check_() {
    if (state.dismissed) return;
    state.status = { kind: "checking" };
    try {
      const update = await check();
      if (!update) { state.status = { kind: "idle" }; return; }
      pending = update;
      state.status = { kind: "available", version: update.version, notes: update.body ?? null };
      // Kick off the download immediately — the user experience we want is
      // "click Install, app restarts," not "click Install, wait 2 minutes."
      await download_();
    } catch (e) {
      state.status = { kind: "error", message: String(e) };
    }
  }

  async function download_() {
    if (!pending) return;
    const version = pending.version;
    const notes = pending.body ?? null;
    let contentLength = 0;
    let downloaded = 0;
    state.status = { kind: "downloading", version, progress: 0 };
    try {
      await pending.download((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            state.status = {
              kind: "downloading",
              version,
              progress: contentLength > 0 ? Math.min(1, downloaded / contentLength) : 0,
            };
            break;
          case "Finished":
            state.status = { kind: "ready", version, notes };
            break;
        }
      });
    } catch (e) {
      state.status = { kind: "error", message: String(e) };
    }
  }

  async function installAndRestart() {
    if (!pending) return;
    try {
      await pending.install();
      await relaunch();
    } catch (e) {
      state.status = { kind: "error", message: String(e) };
    }
  }

  function dismiss() {
    state.dismissed = true;
    state.status = { kind: "idle" };
  }

  return {
    get status() { return state.status; },
    get dismissed() { return state.dismissed; },
    check: check_,
    installAndRestart,
    dismiss,
  };
}

export const updater = createUpdater();
export type UpdaterStore = ReturnType<typeof createUpdater>;

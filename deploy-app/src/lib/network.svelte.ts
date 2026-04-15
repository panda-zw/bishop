/**
 * Network connectivity state.
 *
 * Primary source: `navigator.onLine` + window `online`/`offline` events.
 * That only reflects the OS network interface (Wi-Fi, Ethernet) — it won't
 * catch every "no internet" case (e.g. captive portal), but it's reliable for
 * the common "Wi-Fi dropped" failure mode that produces the
 * "Network is unreachable" SSH error.
 *
 * Consumers (stores, components) subscribe to `online` via the getter;
 * on offline→online transitions we emit an event through a small listener
 * list so stores can re-probe their state.
 */

function createNetwork() {
  const state = $state({
    online: typeof navigator === "undefined" ? true : navigator.onLine,
    lastOfflineAt: null as number | null,
    lastOnlineAt: Date.now(),
  });

  type Listener = () => void;
  const reconnectListeners = new Set<Listener>();

  function setOnline(v: boolean) {
    if (state.online === v) return;
    state.online = v;
    const now = Date.now();
    if (v) {
      state.lastOnlineAt = now;
      // Fire reconnect handlers.
      for (const fn of reconnectListeners) { try { fn(); } catch {} }
    } else {
      state.lastOfflineAt = now;
    }
  }

  if (typeof window !== "undefined") {
    window.addEventListener("online",  () => setOnline(true));
    window.addEventListener("offline", () => setOnline(false));
  }

  return {
    get online() { return state.online; },
    get lastOfflineAt() { return state.lastOfflineAt; },
    get lastOnlineAt() { return state.lastOnlineAt; },

    /// Subscribe to offline→online transitions. Returns an unsubscribe fn.
    onReconnect(fn: Listener): () => void {
      reconnectListeners.add(fn);
      return () => reconnectListeners.delete(fn);
    },

    /// Manual override — useful for a "force reconnect" button that refreshes
    /// everything even when `navigator.onLine` says we're still online.
    markReconnect() {
      state.lastOnlineAt = Date.now();
      for (const fn of reconnectListeners) { try { fn(); } catch {} }
    },
  };
}

export const network = createNetwork();
export type NetworkStore = ReturnType<typeof createNetwork>;

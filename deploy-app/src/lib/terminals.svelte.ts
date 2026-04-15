/**
 * Multi-tab terminal workspace — factory store. See stores.svelte.ts for the
 * rationale behind using factories rather than classes with $state fields.
 *
 * Each tab owns a PTY stream. Tabs survive modal close/reopen because the
 * workspace component stays mounted. Closing a tab kills its PTY.
 */

import { api, onTermOut, onTermExit } from "./api";
import type { TerminalTarget } from "./types";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface TerminalTab {
  id: string;
  streamId: string | null;
  label: string;
  subtitle: string;
  target: TerminalTarget;
  hostKey: string;
  replayScrollback: boolean;
  status: "connecting" | "open" | "closed" | "error";
  errorMsg?: string;
  createdAt: number;
  onOut: ((bytes: Uint8Array) => void) | null;
  onExit: (() => void) | null;
  unlistenOut: UnlistenFn | null;
  unlistenExit: UnlistenFn | null;
}

export function hostKeyFor(target: TerminalTarget): string {
  if (target.local) return `local:${target.initial_cwd ?? "~"}`;
  if (target.alias) return `alias:${target.alias}`;
  const user = target.user ?? "";
  const host = target.host ?? "";
  const port = target.port ?? 22;
  return `${user}@${host}:${port}`;
}

export type PanelLayout = "single" | "rows" | "cols" | "grid";

const TABS_STORAGE_KEY = "bishop-terminal-tabs";
const FONT_SIZE_KEY = "bishop-terminal-font-size";
const DEFAULT_FONT_SIZE = 13;
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 28;

/// Subset of TerminalTab that's safe to persist across restarts. Runtime-only
/// fields (streamId, listeners, callbacks) are re-created on rehydrate.
interface PersistedTab {
  id: string;
  label: string;
  subtitle: string;
  target: TerminalTarget;
  hostKey: string;
}

function createTerminals() {
  const state = $state({
    tabs: [] as TerminalTab[],
    activeId: null as string | null,
    open: false,
    panelHeight: 360,
    maximized: false,
    layout: "single" as PanelLayout,
    fontSize: DEFAULT_FONT_SIZE,
  });

  // Non-reactive auxiliary state.
  let preMaxHeight = 360;
  let seq = 0;
  const replayedHosts = new Set<string>();

  try {
    const v = Number(localStorage.getItem("bishop-terminal-height"));
    if (v >= 160 && v <= 2000) state.panelHeight = v;
  } catch {}

  try {
    const v = Number(localStorage.getItem(FONT_SIZE_KEY));
    if (v >= MIN_FONT_SIZE && v <= MAX_FONT_SIZE) state.fontSize = v;
  } catch {}

  /// Rehydrate persisted tabs. PTYs are backend-only and don't survive a
  /// restart, so each tab starts with streamId=null and status=connecting —
  /// TerminalTab.init() will spin up a fresh PTY when it mounts. replayScrollback
  /// is true so the previous session's saved bytes get replayed into the pane.
  try {
    const raw = localStorage.getItem(TABS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as { tabs: PersistedTab[]; activeId: string | null; open: boolean; layout?: PanelLayout };
      if (Array.isArray(parsed.tabs) && parsed.tabs.length > 0) {
        for (const p of parsed.tabs) {
          const numId = Number(p.id.replace(/^tab-/, ""));
          if (Number.isFinite(numId) && numId > seq) seq = numId;
          state.tabs.push({
            id: p.id,
            streamId: null,
            label: p.label,
            subtitle: p.subtitle,
            target: p.target,
            hostKey: p.hostKey,
            replayScrollback: true,
            status: "connecting",
            createdAt: Date.now(),
            onOut: null, onExit: null,
            unlistenOut: null, unlistenExit: null,
          });
        }
        state.activeId = parsed.activeId && state.tabs.some(t => t.id === parsed.activeId)
          ? parsed.activeId
          : state.tabs[0].id;
        state.open = !!parsed.open;
        if (parsed.layout === "single" || parsed.layout === "rows" || parsed.layout === "cols" || parsed.layout === "grid") {
          state.layout = parsed.layout;
        }
      }
    }
  } catch {}

  function persistTabs() {
    try {
      const payload = {
        tabs: state.tabs.map<PersistedTab>(t => ({
          id: t.id, label: t.label, subtitle: t.subtitle, target: t.target, hostKey: t.hostKey,
        })),
        activeId: state.activeId,
        open: state.open,
        layout: state.layout,
      };
      localStorage.setItem(TABS_STORAGE_KEY, JSON.stringify(payload));
    } catch {}
  }

  function sameTarget(a: TerminalTarget, b: TerminalTarget): boolean {
    return hostKeyFor(a) === hostKeyFor(b);
  }

  const self = {
    // ---- reactive accessors ----
    get tabs() { return state.tabs; },
    get activeId() { return state.activeId; },
    get open() { return state.open; },
    get panelHeight() { return state.panelHeight; },
    get maximized() { return state.maximized; },
    get layout() { return state.layout; },
    get fontSize() { return state.fontSize; },

    setFontSize(px: number) {
      const clamped = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, Math.round(px)));
      state.fontSize = clamped;
      try { localStorage.setItem(FONT_SIZE_KEY, String(clamped)); } catch {}
    },
    bumpFontSize(delta: number) { self.setFontSize(state.fontSize + delta); },
    resetFontSize() { self.setFontSize(DEFAULT_FONT_SIZE); },

    get active(): TerminalTab | null {
      return state.tabs.find(t => t.id === state.activeId) ?? null;
    },

    byId(id: string): TerminalTab | undefined {
      return state.tabs.find(t => t.id === id);
    },

    // ---- visibility ----
    show() { state.open = true; persistTabs(); },
    hide() { state.open = false; persistTabs(); },
    toggle() { state.open = !state.open; persistTabs(); },

    // ---- resize / maximize ----
    setPanelHeight(px: number) {
      state.panelHeight = Math.max(160, Math.min(2000, px));
      try { localStorage.setItem("bishop-terminal-height", String(state.panelHeight)); } catch {}
    },

    maximize() {
      if (state.maximized) return;
      preMaxHeight = state.panelHeight;
      state.maximized = true;
    },
    restore() {
      if (!state.maximized) return;
      state.maximized = false;
    },
    toggleMaximize() { state.maximized ? self.restore() : self.maximize(); },

    setLayout(l: PanelLayout) { state.layout = l; persistTabs(); },

    // ---- tab lifecycle ----
    setActive(id: string) {
      if (state.tabs.some(t => t.id === id)) { state.activeId = id; persistTabs(); }
    },

    /// Rename a tab's display label. Empty/whitespace-only names are rejected.
    renameTab(id: string, label: string) {
      const clean = label.trim();
      if (!clean) return;
      state.tabs = state.tabs.map(t => t.id === id ? { ...t, label: clean } : t);
      persistTabs();
    },

    addOrFocusTab(target: TerminalTarget, label: string, subtitle: string): TerminalTab {
      const existing = state.tabs.find(t =>
        sameTarget(t.target, target) && t.status !== "closed" && t.status !== "error"
      );
      if (existing) {
        state.activeId = existing.id;
        state.open = true;
        persistTabs();
        return existing;
      }
      return self.addTab(target, label, subtitle);
    },

    addTab(target: TerminalTarget, label: string, subtitle: string): TerminalTab {
      const id = `tab-${++seq}`;
      const hostKey = hostKeyFor(target);
      const replayScrollback = !replayedHosts.has(hostKey);
      replayedHosts.add(hostKey);
      const tab: TerminalTab = {
        id,
        streamId: null,
        label,
        subtitle,
        target,
        hostKey,
        replayScrollback,
        status: "connecting",
        createdAt: Date.now(),
        onOut: null, onExit: null,
        unlistenOut: null, unlistenExit: null,
      };
      state.tabs = [...state.tabs, tab];
      state.activeId = id;
      state.open = true;
      persistTabs();
      return tab;
    },

    updateTab(id: string, patch: Partial<TerminalTab>) {
      state.tabs = state.tabs.map(t => t.id === id ? { ...t, ...patch } : t);
      // Persist only when a user-visible field changed — skip churn from
      // streamId/listener updates on every PTY event.
      if ("label" in patch || "subtitle" in patch || "target" in patch) persistTabs();
    },

    closeTab(id: string) {
      const tab = state.tabs.find(t => t.id === id);
      if (!tab) return;

      state.tabs = state.tabs.filter(t => t.id !== id);
      const stillUsed = state.tabs.some(t => t.hostKey === tab.hostKey);
      if (!stillUsed) replayedHosts.delete(tab.hostKey);
      if (state.activeId === id) {
        state.activeId = state.tabs[state.tabs.length - 1]?.id ?? null;
      }
      if (state.tabs.length === 0) state.open = false;
      persistTabs();

      try { tab.unlistenOut?.(); } catch {}
      try { tab.unlistenExit?.(); } catch {}
      if (tab.streamId) { api.termClose(tab.streamId).catch(() => {}); }
    },

    // ---- PTY session bridging ----
    async startSession(tab: TerminalTab, cols: number, rows: number): Promise<string> {
      const current = self.byId(tab.id);
      if (current?.streamId) return current.streamId;
      try {
        const streamId = await api.startTerminal(tab.target, cols, rows);
        self.updateTab(tab.id, { streamId, status: "open" });
        return streamId;
      } catch (e) {
        self.updateTab(tab.id, { status: "error", errorMsg: String(e) });
        throw e;
      }
    },

    async attachListeners(
      id: string,
      streamId: string,
      onOut: (bytes: Uint8Array) => void,
      onExit: () => void,
    ) {
      const prev = self.byId(id);
      if (prev?.unlistenOut) { try { prev.unlistenOut(); } catch {} }
      if (prev?.unlistenExit) { try { prev.unlistenExit(); } catch {} }

      const unlistenOut = await onTermOut(streamId, (b64) => {
        const bin = atob(b64);
        const arr = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
        onOut(arr);
      });
      const unlistenExit = await onTermExit(streamId, () => onExit());
      self.updateTab(id, { unlistenOut, unlistenExit, onOut, onExit });
    },
  };

  return self;
}

export const terminals = createTerminals();
export type TerminalsStore = ReturnType<typeof createTerminals>;

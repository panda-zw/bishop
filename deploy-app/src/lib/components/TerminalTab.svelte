<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { api } from "../api";
  import { terminals, type TerminalTab } from "../terminals.svelte";
  import { theme } from "../theme.svelte";

  interface Props { tab: TerminalTab; active: boolean }
  let { tab, active }: Props = $props();

  let container: HTMLDivElement | null = $state(null);
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let ro: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;

  /// Scrollback ring buffer (raw bytes). Sent to disk periodically and on close.
  const MAX_BYTES = 256 * 1024;
  let scrollbuf = new Uint8Array(0);
  let saveHandle: ReturnType<typeof setInterval> | null = null;
  let dirty = false;

  function appendToBuf(bytes: Uint8Array) {
    if (bytes.length === 0) return;
    const combined = new Uint8Array(scrollbuf.length + bytes.length);
    combined.set(scrollbuf, 0);
    combined.set(bytes, scrollbuf.length);
    scrollbuf = combined.length > MAX_BYTES
      ? combined.slice(combined.length - MAX_BYTES)
      : combined;
    dirty = true;
  }

  function bytesToB64(bytes: Uint8Array): string {
    let bin = "";
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin);
  }
  function b64ToBytes(b64: string): Uint8Array {
    if (!b64) return new Uint8Array(0);
    const bin = atob(b64);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
    return arr;
  }

  async function flushScrollback() {
    if (!dirty || scrollbuf.length === 0) return;
    try {
      await api.writeScrollback(tab.hostKey, bytesToB64(scrollbuf));
      dirty = false;
    } catch (e) {
      console.warn("scrollback save failed", e);
    }
  }

  function fmtRelative(ts: number): string {
    if (!ts) return "";
    const diff = Math.max(0, Math.floor(Date.now() / 1000 - ts));
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return new Date(ts * 1000).toLocaleString();
  }

  /// xterm palettes keyed to the app theme.
  const XTERM_THEMES = {
    dark: {
      background: "#0b0d10",
      foreground: "#e5e7eb",
      cursor: "#e5e7eb",
      cursorAccent: "#0b0d10",
      selectionBackground: "#3b82f655",
      selectionForeground: "#e5e7eb",
    },
    light: {
      background: "#ffffff",
      foreground: "#0c1220",
      cursor: "#0c1220",
      cursorAccent: "#ffffff",
      selectionBackground: "#2563eb33",
      selectionForeground: "#0c1220",
    },
  } as const;

  async function init() {
    if (!container) return;

    term = new Terminal({
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
      fontSize: terminals.fontSize,
      theme: XTERM_THEMES[theme.current],
      cursorBlink: true,
      convertEol: false,
      allowProposedApi: true,
      scrollback: 5000,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(container);
    fit.fit();

    // Replay saved scrollback on the very first mount for this tab.
    // Store then clears the replay flag so re-mounts don't stack banners.
    try {
      const current = terminals.byId(tab.id);
      if (current?.replayScrollback) {
        const prior = await api.readScrollback(tab.hostKey);
        terminals.updateTab(tab.id, { replayScrollback: false });
        if (prior.data) {
          const bytes = b64ToBytes(prior.data);
          const decoder = new TextDecoder("utf-8", { fatal: false });
          term.write(decoder.decode(bytes));
          const banner =
            `\r\n\x1b[90m── Previous session (${fmtRelative(prior.saved_at)}) ──\x1b[0m\r\n` +
            `\x1b[90m   Reconnecting… running processes from the previous session are not preserved.\x1b[0m\r\n` +
            (tab.target.use_tmux
              ? `\x1b[90m   This host uses tmux — attaching to the existing session if available.\x1b[0m\r\n`
              : "") +
            `\r\n`;
          term.write(banner);
        }
      }
    } catch { /* ignore */ }

    const { cols, rows } = term;
    let streamId: string;
    try {
      // Idempotent in the store: returns existing streamId if one is set.
      streamId = await terminals.startSession(tab, cols, rows);
    } catch { return; }

    const decoder = new TextDecoder("utf-8", { fatal: false });
    await terminals.attachListeners(
      tab.id,
      streamId,
      (bytes) => {
        term?.write(decoder.decode(bytes, { stream: true }));
        appendToBuf(bytes);
      },
      () => {
        terminals.updateTab(tab.id, { status: "closed" });
        term?.writeln("\r\n\x1b[90m[session ended]\x1b[0m");
        appendToBuf(new TextEncoder().encode("\r\n[session ended]\r\n"));
        flushScrollback();
      },
    );

    const encoder = new TextEncoder();
    term.onData((data) => {
      if (!streamId) return;
      const bytes = encoder.encode(data);
      let bin = ""; for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
      api.termWrite(streamId, btoa(bin)).catch((e) => console.error("write", e));
    });

    ro = new ResizeObserver(() => {
      if (!fit || !term || !streamId) return;
      try { fit.fit(); } catch {}
      scheduleResize(streamId);
    });
    ro.observe(container);

    saveHandle = setInterval(flushScrollback, 15000);
  }

  /// Debounced termResize RPC — ResizeObserver fires on every pixel during a
  /// drag, which would spam the backend with 60+ calls/sec.
  function scheduleResize(streamId: string) {
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      if (!term) return;
      api.termResize(streamId, term.cols, term.rows).catch(() => {});
    }, 60);
  }

  $effect(() => {
    if (active && term && fit) {
      queueMicrotask(() => { try { fit!.fit(); term!.focus(); } catch {} });
    }
  });

  // Re-fit on layout / size / tab-count changes. Removing or adding a tab
  // reshuffles the grid; each surviving pane must re-measure its cell.
  $effect(() => {
    terminals.layout; terminals.maximized; terminals.panelHeight; terminals.tabs.length;
    if (!term || !fit) return;
    queueMicrotask(() => {
      try {
        fit!.fit();
        if (tab.streamId) scheduleResize(tab.streamId);
      } catch {}
    });
  });

  // Swap xterm colors when the app theme flips so already-open terminals
  // don't stay stuck with the old palette.
  $effect(() => {
    const current = theme.current;
    if (!term) return;
    try { term.options.theme = XTERM_THEMES[current]; } catch {}
  });

  // Live font-size changes — re-fit to recompute cols/rows, then push the
  // new geometry to the PTY so wrapping matches what the user sees.
  $effect(() => {
    const size = terminals.fontSize;
    if (!term) return;
    try {
      term.options.fontSize = size;
      queueMicrotask(() => {
        try {
          fit?.fit();
          if (tab.streamId) scheduleResize(tab.streamId);
        } catch {}
      });
    } catch {}
  });

  onMount(() => { init(); });
  onDestroy(() => {
    if (ro) ro.disconnect();
    if (saveHandle) clearInterval(saveHandle);
    if (resizeTimer) clearTimeout(resizeTimer);
    // Detach Tauri listeners so they don't fire into a disposed xterm.
    // The backend PTY stays alive — only `terminals.closeTab` kills it.
    const current = terminals.byId(tab.id);
    try { current?.unlistenOut?.(); } catch {}
    try { current?.unlistenExit?.(); } catch {}
    terminals.updateTab(tab.id, { unlistenOut: null, unlistenExit: null });
    void flushScrollback();
    term?.dispose();
    term = null;
  });
</script>

<div class="h-full w-full modal-sunken p-2" class:hidden={!active}>
  <div bind:this={container} class="h-full w-full"></div>
</div>

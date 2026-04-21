<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { ImageAddon } from "@xterm/addon-image";
  import "@xterm/xterm/css/xterm.css";
  import { api } from "../api";
  import { terminals, type TerminalTab } from "../terminals.svelte";
  import { theme } from "../theme.svelte";
  import { toast } from "../toast.svelte";

  interface Props { tab: TerminalTab; active: boolean }
  let { tab, active }: Props = $props();

  let container: HTMLDivElement | null = $state(null);
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let ro: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let dropActive = $state(false);

  /// Inline preview cap — emitting an iTerm2 escape for a 50 MB image can
  /// stall xterm for seconds. The file still gets saved/uploaded at full size.
  const MAX_INLINE_PREVIEW_BYTES = 5 * 1024 * 1024;
  /// Drop/paste hard cap — matches the backend limit so we fail fast with a
  /// readable toast instead of waiting on a long IPC round-trip.
  const MAX_FILE_BYTES = 200 * 1024 * 1024;

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
    // Inline image rendering for pasted/dropped images (iTerm2 + Sixel + Kitty).
    // Dropping an image into the tab emits an iTerm2 escape after the file path
    // so the user gets a visual confirmation above their command line.
    try { term.loadAddon(new ImageAddon()); } catch {}
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

    // Paste handler has to run at capture phase so it fires before xterm's
    // internal textarea consumes the event. When the clipboard holds files,
    // we preventDefault + stopPropagation and upload ourselves; otherwise we
    // no-op and xterm's text-paste path runs as usual.
    container.addEventListener("paste", handlePaste, { capture: true });
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

  /// POSIX single-quote quoting — every path we write to the PTY goes through
  /// this so filenames with spaces/specials reach the shell as a single arg.
  function shellQuote(s: string): string {
    return `'${s.split("'").join("'\\''")}'`;
  }

  function extToMime(name: string): string {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    if (ext === "png") return "image/png";
    if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
    if (ext === "gif") return "image/gif";
    if (ext === "webp") return "image/webp";
    return "application/octet-stream";
  }

  /// Emit an iTerm2 inline-image escape into the LOCAL xterm (not the PTY).
  /// The remote/local shell never sees this — only xterm's ImageAddon renders
  /// it, giving the user a thumbnail preview above their typed command.
  function previewImageLocally(name: string, bytes: Uint8Array) {
    if (!term || bytes.length > MAX_INLINE_PREVIEW_BYTES) return;
    const b64Name = btoa(unescape(encodeURIComponent(name)));
    const b64Data = bytesToB64(bytes);
    const esc =
      `\x1b]1337;File=name=${b64Name};size=${bytes.length};` +
      `inline=1;height=auto;width=auto;preserveAspectRatio=1:${b64Data}\x07\r\n`;
    term.write(esc);
  }

  /// Types `paths` into the PTY as space-separated shell-quoted args. No
  /// trailing newline — the user reviews and presses Enter themselves.
  function typePathsIntoShell(paths: string[]) {
    const active = terminals.byId(tab.id);
    if (!active?.streamId) return;
    const encoder = new TextEncoder();
    const joined = paths.map(shellQuote).join(" ");
    const bytes = encoder.encode(joined);
    api.termWrite(active.streamId, bytesToB64(bytes)).catch((e) => console.error("type", e));
  }

  /// Core pipeline — invoked by both drop and paste. For each file: size
  /// check, backend upload, preview if image, finally type the resulting
  /// paths into the PTY.
  async function processFiles(files: File[]) {
    if (files.length === 0) return;
    const paths: string[] = [];
    for (const file of files) {
      if (file.size > MAX_FILE_BYTES) {
        toast.error(
          `${file.name || "file"} too large`,
          `${(file.size / 1_048_576).toFixed(1)} MB — limit is ${MAX_FILE_BYTES / 1_048_576} MB. scp it directly instead.`,
        );
        continue;
      }
      try {
        const buf = new Uint8Array(await file.arrayBuffer());
        const b64 = bytesToB64(buf);
        const filename = file.name || "pasted.bin";
        const path = tab.target.local
          ? await api.pasteFileLocal(b64, filename)
          : await api.pasteFileRemote(tab.target, b64, filename);
        paths.push(path);

        const mime = file.type || extToMime(filename);
        if (mime.startsWith("image/")) {
          previewImageLocally(filename, buf);
        }
        toast.success("Saved", path);
      } catch (e) {
        toast.error("Upload failed", String(e));
      }
    }
    if (paths.length > 0) typePathsIntoShell(paths);
  }

  function handleDragOver(e: DragEvent) {
    if (!e.dataTransfer) return;
    // Only claim the event when actual files are being dragged — plain
    // text/URL drags stay with xterm's default behavior.
    const hasFiles = Array.from(e.dataTransfer.items).some(i => i.kind === "file");
    if (!hasFiles) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    dropActive = true;
  }

  function handleDragLeave(e: DragEvent) {
    // Fires when the pointer leaves the entire pane, not child elements.
    if (e.currentTarget === e.target) dropActive = false;
  }

  async function handleDrop(e: DragEvent) {
    dropActive = false;
    if (!e.dataTransfer) return;
    const files = Array.from(e.dataTransfer.files);
    if (files.length === 0) return;
    e.preventDefault();
    await processFiles(files);
  }

  async function handlePaste(e: ClipboardEvent) {
    if (!e.clipboardData) return;
    const files: File[] = [];
    for (const item of Array.from(e.clipboardData.items)) {
      if (item.kind === "file") {
        const f = item.getAsFile();
        if (f) files.push(f);
      }
    }
    if (files.length === 0) return;
    // Only swallow the paste when files are present — regular text-paste
    // continues to flow into xterm normally.
    e.preventDefault();
    e.stopPropagation();
    await processFiles(files);
  }

  onMount(() => { init(); });
  onDestroy(() => {
    if (ro) ro.disconnect();
    if (saveHandle) clearInterval(saveHandle);
    if (resizeTimer) clearTimeout(resizeTimer);
    if (container) container.removeEventListener("paste", handlePaste, { capture: true } as EventListenerOptions);
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="relative h-full w-full modal-sunken p-2"
  class:hidden={!active}
  class:drop-active={dropActive}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
>
  <div bind:this={container} class="h-full w-full"></div>
  {#if dropActive}
    <div class="pointer-events-none absolute inset-2 rounded-md border-2 border-dashed border-accent bg-accent/10 flex items-center justify-center">
      <span class="text-sm text-accent font-medium">Drop to {tab.target.local ? "save locally" : "upload to remote"}</span>
    </div>
  {/if}
</div>

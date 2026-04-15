<script lang="ts">
  import { api, onLogLine } from "../api";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  interface Props {
    projectPath: string;
    env: string;
    service: string;
    accent?: string;
    onClose?: () => void;
  }
  let { projectPath, env, service, accent = "#3b82f6", onClose }: Props = $props();

  let lines = $state<string[]>([]);
  let connecting = $state(true);
  let paused = $state(false);
  let wrap = $state(false);
  let filter = $state("");
  let streamId: string | null = null;
  let unlisten: UnlistenFn | null = null;
  let viewport: HTMLDivElement | null = null;
  /// Monotonic token — any async callback tied to an older session is ignored.
  let epoch = 0;

  const MAX_LINES = 1000;

  // docker logs --timestamps prefixes each line with RFC3339, e.g.
  //   2026-04-14T19:42:15.123456789Z <rest>
  // Split that off so we can dim it.
  const tsRe = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)\s(.*)$/s;
  function split(line: string): { ts: string; body: string } {
    const m = tsRe.exec(line);
    if (!m) return { ts: "", body: line };
    // Shorten to HH:MM:SS — date isn't useful in a live stream.
    const short = m[1].slice(11, 19);
    return { ts: short, body: m[2] };
  }

  const shown = $derived.by(() => {
    if (!filter.trim()) return lines;
    const q = filter.toLowerCase();
    return lines.filter(l => l.toLowerCase().includes(q));
  });

  async function start() {
    await stop();
    const myEpoch = ++epoch;
    lines = [];
    connecting = true;
    try {
      const id = await api.startLogStream(projectPath, env, service);
      if (myEpoch !== epoch) { try { await api.stopLogStream(id); } catch {} return; }
      streamId = id;
      unlisten = await onLogLine(id, (line) => {
        if (myEpoch !== epoch || paused) return;
        if (connecting) connecting = false;
        lines.push(line);
        if (lines.length > MAX_LINES) lines.splice(0, lines.length - MAX_LINES);
        queueMicrotask(() => viewport?.scrollTo({ top: viewport.scrollHeight }));
      });
      // Even with no output yet, mark connected after a moment so the user sees the pane settle.
      setTimeout(() => { if (myEpoch === epoch) connecting = false; }, 1200);
    } catch (e) {
      if (myEpoch === epoch) {
        lines = [`[error starting stream: ${e}]`];
        connecting = false;
      }
    }
  }

  async function stop() {
    epoch++;
    if (unlisten) { unlisten(); unlisten = null; }
    if (streamId) { try { await api.stopLogStream(streamId); } catch {} streamId = null; }
  }

  function clear() { lines = []; }

  $effect(() => {
    projectPath; env; service;
    start();
    return () => { stop(); };
  });
</script>

<div class="border border-border rounded-lg overflow-hidden flex flex-col h-full border-l-[3px] bg-card" style="border-left-color: {accent}">
  <div class="flex items-center gap-2 px-3 h-9 border-b border-border bg-card">
    <span class="text-xs font-mono font-medium">{service}</span>
    <div class="flex-1 mx-2 relative">
      <svg viewBox="0 0 16 16" class="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-muted pointer-events-none" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
        <circle cx="7" cy="7" r="4.5" />
        <path stroke-linecap="round" d="M10.5 10.5L13.5 13.5" />
      </svg>
      <input
        class="w-full h-7 bg-secondary/40 border border-transparent rounded-md pl-7 pr-2 text-xs font-mono placeholder:text-muted hover:bg-secondary/60 focus:bg-background focus-visible:outline-none focus-visible:border-ring/60 focus-visible:ring-0 transition-colors"
        placeholder="Filter logs"
        aria-label="Filter {service} logs"
        bind:value={filter}
      />
    </div>
    <button
      class="text-[11px] h-6 px-2 rounded-md border border-border hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      onclick={() => (wrap = !wrap)}
      aria-label="Toggle line wrap (currently {wrap ? 'on' : 'off'})"
    >{wrap ? "no-wrap" : "wrap"}</button>
    <button
      class="text-[11px] h-6 px-2 rounded-md border border-border hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      onclick={() => (paused = !paused)}
      aria-label={paused ? "Resume log stream" : "Pause log stream"}
    >{paused ? "resume" : "pause"}</button>
    <button
      class="text-[11px] h-6 px-2 rounded-md border border-border hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      onclick={clear}
      aria-label="Clear log buffer"
    >clear</button>
    {#if onClose}
      <button
        class="text-[11px] h-6 w-6 rounded-md border border-border hover:bg-secondary inline-flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={onClose}
        aria-label="Close {service} log"
      >✕</button>
    {/if}
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    bind:this={viewport}
    class="flex-1 overflow-auto font-mono text-[11px] leading-[18px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-inset"
    tabindex="0"
    role="log"
    aria-label="{service} log output"
    aria-live="off"
  >
    {#if connecting && lines.length === 0}
      <div class="px-3 py-3 space-y-1.5" aria-hidden="true">
        {#each [70, 92, 60, 85, 48, 78] as w, i (i)}
          <div class="flex gap-3">
            <div class="animate-pulse bg-secondary/70 h-3 w-12 rounded-sm shrink-0"></div>
            <div class="animate-pulse bg-secondary/50 h-3 rounded-sm" style="width: {w}%"></div>
          </div>
        {/each}
      </div>
    {:else if shown.length === 0}
      <div class="px-3 py-4 text-muted text-xs">
        {filter ? "No lines match your filter." : "Waiting for log output…"}
      </div>
    {:else}
      {#each shown as line, i (i)}
        {@const parts = split(line)}
        <div class="flex gap-3 px-3 {i % 2 === 0 ? '' : 'bg-secondary/25'} {wrap ? '' : 'whitespace-nowrap'}">
          {#if parts.ts}
            <span class="text-muted shrink-0 select-none">{parts.ts}</span>
          {/if}
          <span class="{wrap ? 'whitespace-pre-wrap break-all' : 'whitespace-pre'}">{parts.body}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

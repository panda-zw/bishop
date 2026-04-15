<script lang="ts">
  import { hosts, type AggregatedHost } from "../hosts.svelte";
  import { terminals } from "../terminals.svelte";
  import { app } from "../stores.svelte";
  import type { TerminalTarget } from "../types";

  interface Props { onPick: () => void }
  let { onPick }: Props = $props();

  let query = $state("");
  let customInput = $state("");

  function openLocal(cwd?: string | null, label = "Local shell") {
    terminals.addOrFocusTab({ local: true, initial_cwd: cwd ?? null }, label, cwd ?? "~");
    onPick();
  }

  const groups = $derived.by(() => hosts.aggregated());

  function matches(h: AggregatedHost) {
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return h.label.toLowerCase().includes(q)
      || h.subtitle.toLowerCase().includes(q)
      || (h.meta ?? "").toLowerCase().includes(q);
  }

  function open(target: TerminalTarget, label: string, subtitle: string) {
    terminals.addOrFocusTab(target, label, subtitle);
    onPick();
  }

  function parseCustom(input: string): TerminalTarget | null {
    const s = input.trim();
    if (!s) return null;
    if (s.includes("@")) {
      const [user, host] = s.split("@", 2);
      if (!user || !host) return null;
      return { user, host };
    }
    return { alias: s };
  }

  function openCustom() {
    const target = parseCustom(customInput);
    if (!target) return;
    const label = target.alias ?? `${target.user}@${target.host}`;
    open(target, label, label);
    customInput = "";
  }

  /// Drop subtitle when it duplicates the label (e.g. ssh_config hosts
  /// where the alias IS the IP so both rows show the same string).
  function subtitleFor(label: string, subtitle: string): string {
    if (!subtitle) return "";
    if (subtitle === label) return "";
    // also dedupe if subtitle just adds "user@" to an IP label
    if (subtitle.endsWith(`@${label}`)) return subtitle;
    return subtitle;
  }

  $effect(() => { hosts.refresh(); });
</script>

<div class="flex flex-col h-full">
  <div class="px-3 py-2 border-b border-border">
    <input
      type="text"
      placeholder="Search saved, remotes, ssh config…"
      class="w-full h-8 rounded-md border border-input bg-background px-3 text-xs placeholder:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      bind:value={query}
    />
  </div>
  <div class="flex-1 overflow-auto p-1">
    <!-- Local — pinned on top, always visible -->
    {#if !query.trim() || /local|shell|here/i.test(query)}
      <div class="mb-1">
        <div class="px-3 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-muted">Local</div>
        <button
          type="button"
          class="w-full group flex items-center gap-3 px-3 py-2 rounded-md hover:bg-secondary text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={() => openLocal()}
        >
          <span class="w-7 h-7 shrink-0 inline-flex items-center justify-center rounded-md bg-secondary/60 group-hover:bg-background/50">
            <svg viewBox="0 0 16 16" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2 3h12v10H2zM4.5 6l2 1.5-2 1.5M8 9.5h4"/>
            </svg>
          </span>
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium">Local shell</div>
            <div class="text-[11px] text-muted truncate">opens in your home directory</div>
          </div>
        </button>
        {#if app.activeProject}
          <button
            type="button"
            class="w-full group flex items-center gap-3 px-3 py-2 rounded-md hover:bg-secondary text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => openLocal(app.activeProject!.path, `Local · ${app.activeProject!.name}`)}
          >
            <span class="w-7 h-7 shrink-0 inline-flex items-center justify-center rounded-md bg-secondary/60 group-hover:bg-background/50">
              <svg viewBox="0 0 16 16" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.5 1.5h4.5A1.5 1.5 0 0 1 14 6v5.5A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5v-7Z"/>
              </svg>
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium">Local shell in {app.activeProject.name}</div>
              <div class="text-[11px] text-muted truncate font-mono">{app.activeProject.path}</div>
            </div>
          </button>
        {/if}
      </div>
    {/if}

    <!-- Saved hosts -->
    {#if groups.saved.filter(matches).length > 0}
      <div class="mb-1">
        <div class="px-3 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-muted">Saved hosts</div>
        {#each groups.saved.filter(matches) as h (h.key)}
          {@const sub = subtitleFor(h.label, h.subtitle)}
          <button
            type="button"
            class="w-full group flex items-center gap-3 px-3 py-2 rounded-md hover:bg-secondary text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => open(h.target, h.label, h.subtitle)}
          >
            <span class="w-7 h-7 shrink-0 inline-flex items-center justify-center rounded-md bg-secondary/60 group-hover:bg-background/50">
              <svg viewBox="0 0 16 16" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" d="M2 4h12v4H2zM2 10h12M5 4v-.5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1V4"/>
              </svg>
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium truncate">{h.label}</div>
              {#if sub}<div class="text-[11px] text-muted truncate font-mono">{sub}</div>{/if}
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <!-- Project remotes -->
    {#if groups.remotes.filter(matches).length > 0}
      <div class="mb-1">
        <div class="px-3 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-muted">Project remotes</div>
        {#each groups.remotes.filter(matches) as h (h.key)}
          <button
            type="button"
            class="w-full group flex items-center gap-3 px-3 py-2 rounded-md hover:bg-secondary text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => open(h.target, `${h.meta ?? h.label} · ${h.label}`, h.subtitle)}
          >
            <span class="w-7 h-7 shrink-0 inline-flex items-center justify-center rounded-md bg-secondary/60 group-hover:bg-background/50">
              <svg viewBox="0 0 16 16" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" d="M2 4.5A1.5 1.5 0 0 1 3.5 3h9A1.5 1.5 0 0 1 14 4.5v7A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5zM5 6v4M8 6v4M11 6v4"/>
              </svg>
            </span>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 text-sm">
                <span class="font-medium truncate">{h.meta ?? h.label}</span>
                <span class="text-[10px] uppercase tracking-wider bg-secondary/80 text-muted px-1.5 py-px rounded">{h.label}</span>
              </div>
              <div class="text-[11px] text-muted truncate font-mono">{h.subtitle}</div>
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <!-- SSH config -->
    {#if groups.sshConfig.filter(matches).length > 0}
      <div class="mb-1">
        <div class="px-3 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-muted">SSH config</div>
        {#each groups.sshConfig.filter(matches) as h (h.key)}
          {@const sub = subtitleFor(h.label, h.subtitle)}
          <button
            type="button"
            class="w-full group flex items-center gap-3 px-3 py-2 rounded-md hover:bg-secondary text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => open(h.target, h.label, h.subtitle || h.label)}
          >
            <span class="w-7 h-7 shrink-0 inline-flex items-center justify-center rounded-md bg-secondary/60 group-hover:bg-background/50">
              <svg viewBox="0 0 16 16" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                <circle cx="8" cy="8" r="5.5"/>
                <path stroke-linecap="round" d="M2.5 8h11M8 2.5c2 2.5 2 8.5 0 11M8 2.5c-2 2.5-2 8.5 0 11"/>
              </svg>
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium truncate">{h.label}</div>
              {#if sub || h.meta}
                <div class="text-[11px] text-muted truncate font-mono">
                  {sub}{sub && h.meta ? " · " : ""}{h.meta ?? ""}
                </div>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <!-- Custom -->
    <div class="mt-2 px-3 pt-2 border-t border-border">
      <div class="pb-2 text-[10px] font-semibold uppercase tracking-widest text-muted">Custom</div>
      <form class="flex gap-2" onsubmit={(e) => { e.preventDefault(); openCustom(); }}>
        <input
          type="text"
          placeholder="user@host or ssh alias"
          class="flex-1 h-8 rounded-md border border-input bg-background px-3 text-xs font-mono placeholder:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          bind:value={customInput}
        />
        <button
          type="submit"
          class="h-8 px-3 rounded-md text-xs bg-primary text-primary-foreground disabled:opacity-50"
          disabled={!customInput.trim()}
        >Connect</button>
      </form>
    </div>
  </div>

  <!-- Footer: discover rename + enter hint -->
  <div class="px-3 py-2 border-t border-border text-[11px] text-muted flex items-center justify-between">
    <span>Tip: double-click a tab to rename it</span>
    <span class="font-mono text-[10px] text-muted/80">↵ open · esc close</span>
  </div>
</div>

<script lang="ts">
  import StatusDot from "./StatusDot.svelte";
  import Skeleton from "./ui/Skeleton.svelte";
  import type { Container } from "../types";

  interface Props {
    containers: Container[];
    selected: Set<string>;
    loading?: boolean;
    onToggle: (name: string) => void;
    onRestart?: (name: string) => void;
    restartingName?: string | null;
  }
  let {
    containers, selected, loading = false, onToggle, onRestart, restartingName = null,
  }: Props = $props();
</script>

<div class="border border-border rounded-lg overflow-hidden divide-y divide-border bg-card">
  {#if loading}
    {#each Array(3) as _, i (i)}
      <div class="flex items-center gap-3 px-4 h-10">
        <Skeleton class="w-3.5 h-3.5 rounded-[3px]" />
        <Skeleton class="w-2 h-2 rounded-full" />
        <Skeleton class="h-3 flex-1 max-w-[240px]" />
        <Skeleton class="h-3 w-14" />
      </div>
    {/each}
  {:else}
    {#each containers as c (c.name)}
      {@const isSelected = selected.has(c.name)}
      <div class="flex items-center gap-3 px-4 h-10 group transition-colors {isSelected ? 'bg-secondary/60' : 'hover:bg-secondary/40'}">
        <button
          class="flex items-center gap-3 flex-1 text-left min-w-0 focus-visible:outline-none"
          onclick={() => onToggle(c.name)}
          aria-pressed={isSelected}
          aria-label="{isSelected ? 'Deselect' : 'Select'} container {c.name} for log streaming"
        >
          <span
            aria-hidden="true"
            class="w-3.5 h-3.5 shrink-0 rounded-[3px] border flex items-center justify-center transition-colors
              {isSelected
                ? 'bg-accent border-accent text-accent-foreground'
                : 'border-border group-hover:border-muted'}"
          >
            {#if isSelected}
              <svg viewBox="0 0 12 12" class="w-2.5 h-2.5" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="2,6 5,9 10,3" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            {/if}
          </span>
          <StatusDot state={c.state} health={c.health} />
          <span class="flex-1 font-mono text-xs truncate">{c.name}</span>
          <span class="text-[11px] text-muted">{c.health ?? c.state}</span>
        </button>
        {#if onRestart}
          <button
            class="text-[11px] text-muted hover:text-foreground opacity-0 group-hover:opacity-100 focus-visible:opacity-100 disabled:opacity-50 transition
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 rounded px-1.5 py-0.5"
            disabled={restartingName === c.name}
            onclick={(e) => { e.stopPropagation(); onRestart(c.name); }}
            aria-label="Restart {c.name}"
          >{restartingName === c.name ? "…" : "restart"}</button>
        {/if}
      </div>
    {:else}
      <div class="px-4 py-6 text-center space-y-1">
        <div class="text-sm text-foreground">No containers found</div>
        <div class="text-xs text-muted">
          The app may not be deployed yet. Try <span class="font-mono text-foreground">Deploy</span> or <span class="font-mono text-foreground">Check</span>.
        </div>
      </div>
    {/each}
  {/if}
</div>

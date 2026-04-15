<script lang="ts">
  import StatusDot from "./StatusDot.svelte";
  import Skeleton from "./ui/Skeleton.svelte";
  import Button from "./ui/Button.svelte";
  import { dashActions } from "../stores.svelte";
  import { tasks } from "../tasks.svelte";
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

  // A deploy that's mid-flight changes the meaning of "empty" — not "broken",
  // just "containers will show up in ~60s". Check the task store directly.
  const deployInFlight = $derived(
    tasks.running.some(t => t.kind === "deploy")
  );
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
      {#if deployInFlight}
        <div class="flex flex-col items-center text-center px-4 py-8 gap-3">
          <div class="w-8 h-8 rounded-full border-2 border-border border-t-primary animate-spin" aria-hidden="true"></div>
          <div class="space-y-1">
            <div class="text-sm font-medium">Deploy in progress</div>
            <div class="text-xs text-muted max-w-[260px] leading-relaxed">
              Containers will appear here once the server reports them running.
              Watch the live log in the deploy modal.
            </div>
          </div>
        </div>
      {:else}
        <div class="flex flex-col items-center text-center px-4 py-8 gap-3">
          <div class="w-8 h-8 rounded-lg border border-border bg-background flex items-center justify-center">
            <svg viewBox="0 0 24 24" class="w-4 h-4 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20 7 12 3 4 7m16 0-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
            </svg>
          </div>
          <div class="space-y-1">
            <div class="text-sm font-medium">No deploys yet on this environment</div>
            <div class="text-xs text-muted max-w-[260px] leading-relaxed">
              Once you deploy, every container running on the server will show up here with live status + logs.
            </div>
          </div>
          <Button size="sm" onclick={() => dashActions.deploy()}>Deploy now</Button>
        </div>
      {/if}
    {/each}
  {/if}
</div>

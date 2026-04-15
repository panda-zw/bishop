<script lang="ts">
  /// Top-of-window banner that surfaces auto-update state. Silent on idle;
  /// appears when an update is found and stays visible until installed or
  /// dismissed. Download runs in the background while this is showing.

  import { updater } from "../updater.svelte";
  import Button from "./ui/Button.svelte";

  const s = $derived(updater.status);
</script>

{#if s.kind === "available" || s.kind === "downloading" || s.kind === "ready"}
  <div class="shrink-0 border-b border-border bg-primary/8 text-foreground px-4 py-2 flex items-center gap-3 text-xs">
    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-pulse shrink-0" aria-hidden="true"></span>
    <div class="flex-1 min-w-0 flex items-center gap-2 flex-wrap">
      {#if s.kind === "available"}
        <span class="font-medium">Update available — Bishop {s.version}</span>
        <span class="text-muted">· downloading…</span>
      {:else if s.kind === "downloading"}
        <span class="font-medium">Downloading {s.version}</span>
        <span class="text-muted">· {Math.round(s.progress * 100)}%</span>
        <div class="flex-1 min-w-[80px] max-w-[200px] h-1 rounded-full bg-border/70 overflow-hidden">
          <div class="h-full bg-primary transition-[width] duration-200" style="width: {Math.round(s.progress * 100)}%"></div>
        </div>
      {:else if s.kind === "ready"}
        <span class="font-medium">Bishop {s.version} is ready to install.</span>
        <span class="text-muted">Restart required.</span>
      {/if}
    </div>
    <div class="flex items-center gap-1.5 shrink-0">
      {#if s.kind === "ready"}
        <Button size="sm" onclick={() => updater.installAndRestart()}>Install & restart</Button>
      {/if}
      <button
        type="button"
        class="text-muted hover:text-foreground text-[11px] px-1.5 py-0.5"
        onclick={() => updater.dismiss()}
        aria-label="Dismiss update banner"
      >Later</button>
    </div>
  </div>
{/if}

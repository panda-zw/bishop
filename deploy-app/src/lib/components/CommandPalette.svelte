<script lang="ts">
  import { palette, score, type PaletteAction } from "../palette.svelte";

  interface Props { actions: PaletteAction[] }
  let { actions }: Props = $props();

  let query = $state("");
  let active = $state(0);
  let input: HTMLInputElement | null = $state(null);

  const ranked = $derived.by(() => {
    if (!query.trim()) return actions.map((a, i) => ({ a, s: -i }));
    return actions
      .map((a) => ({ a, s: score(query, `${a.label} ${a.hint ?? ""} ${a.group ?? ""}`) }))
      .filter((r) => r.s > -Infinity)
      .sort((a, b) => b.s - a.s);
  });

  $effect(() => { ranked; active = 0; });

  $effect(() => {
    if (palette.open) {
      query = "";
      active = 0;
      queueMicrotask(() => input?.focus());
    }
  });

  async function run(a: PaletteAction) {
    palette.hide();
    await a.run();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") { palette.hide(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); active = Math.min(active + 1, ranked.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); active = Math.max(active - 1, 0); }
    else if (e.key === "Enter") { e.preventDefault(); if (ranked[active]) run(ranked[active].a); }
  }
</script>

{#if palette.open}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] p-4"
    onclick={() => palette.hide()}
    role="presentation"
  >
    <div aria-hidden="true" class="absolute inset-0 bg-black/70 backdrop-blur-sm"></div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="bishop-modal relative border rounded-lg shadow-2xl shadow-black/60 w-full max-w-xl overflow-hidden"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
    >
      <div class="flex items-center border-b border-border px-3">
        <span class="text-muted text-sm">⌕</span>
        <input
          bind:this={input}
          bind:value={query}
          onkeydown={onKey}
          placeholder="Type a command…"
          class="flex-1 h-11 bg-transparent px-3 text-sm outline-none border-0 shadow-none focus-visible:ring-0"
        />
        <kbd class="text-[10px] text-muted font-mono border border-border rounded px-1.5 py-0.5">esc</kbd>
      </div>
      <div class="max-h-[50vh] overflow-auto py-1 px-1" role="listbox">
        {#each ranked as { a }, i (a.id)}
          <button
            class="w-full flex items-center gap-3 px-3 py-1.5 text-left rounded-md {i === active ? 'bg-secondary text-foreground' : 'text-foreground/90 hover:bg-secondary/60'}"
            role="option"
            aria-selected={i === active}
            onmouseenter={() => (active = i)}
            onclick={() => run(a)}
          >
            <span class="flex-1 text-sm truncate">{a.label}</span>
            {#if a.hint}<span class="text-xs text-muted font-mono">{a.hint}</span>{/if}
          </button>
        {:else}
          <div class="px-4 py-8 text-sm text-muted text-center">No matches</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

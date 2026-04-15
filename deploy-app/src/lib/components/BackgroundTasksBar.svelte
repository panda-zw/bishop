<script lang="ts">
  import { tasks, type Task } from "../tasks.svelte";

  // Show: all running tasks + finished tasks that haven't been dismissed yet,
  // but hide whichever task is currently open (that's already visible as a modal).
  const visible = $derived.by(() =>
    tasks.items.filter(t => t.id !== tasks.activeId)
  );

  function dotCls(t: Task) {
    if (t.status === "running")  return "bg-warn animate-pulse";
    if (t.status === "success")  return "bg-ok";
    if (t.status === "failed")   return "bg-err";
    return "bg-muted";
  }

  function elapsed(t: Task) {
    const end = t.finishedAt ?? Date.now();
    const s = Math.round((end - t.startedAt) / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60); const rem = s % 60;
    return `${m}m ${rem}s`;
  }
</script>

{#if visible.length > 0}
  <div
    class="fixed bottom-4 left-4 z-40 flex flex-col gap-2 max-w-sm"
    role="region"
    aria-label="Background tasks"
  >
    {#each visible as t (t.id)}
      <div class="bishop-modal border rounded-md shadow-lg shadow-black/30 px-3 py-2 flex items-center gap-2.5 min-w-[260px]">
        <span aria-hidden="true" class="w-1.5 h-1.5 rounded-full shrink-0 {dotCls(t)}"></span>
        <button
          class="flex-1 text-left min-w-0 focus-visible:outline-none"
          onclick={() => tasks.show(t.id)}
          aria-label="Open {t.title}"
        >
          <div class="text-xs font-medium truncate">{t.title}</div>
          <div class="text-[10px] text-muted font-mono truncate">
            {t.status === "running" ? "running" : t.status}
            · {elapsed(t)}
          </div>
        </button>
        {#if t.status === "running"}
          <button
            class="text-[11px] text-muted hover:text-err rounded px-1.5 py-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => tasks.cancel(t.id)}
            aria-label="Cancel {t.title}"
          >cancel</button>
        {:else}
          <button
            class="text-[11px] text-muted hover:text-foreground rounded px-1.5 py-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => tasks.dismiss(t.id)}
            aria-label="Dismiss {t.title}"
          >✕</button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

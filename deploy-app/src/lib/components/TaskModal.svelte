<script lang="ts">
  import { tasks, type Task } from "../tasks.svelte";
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";

  let viewport: HTMLDivElement | null = $state(null);
  let shouldAutoscroll = $state(true);
  const ansiRe = /\x1b\[[0-9;]*m/g;
  function clean(line: string) { return line.replace(ansiRe, ""); }

  function onScroll() {
    if (!viewport) return;
    const atBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight < 32;
    shouldAutoscroll = atBottom;
  }

  $effect(() => {
    const t = tasks.active;
    if (!t || !viewport) return;
    // Re-run whenever the line count changes.
    t.lines.length;
    if (shouldAutoscroll) {
      queueMicrotask(() => viewport?.scrollTo({ top: viewport.scrollHeight }));
    }
  });

  function badge(t: Task) {
    switch (t.status) {
      case "running":   return { text: "running", cls: "text-warn" };
      case "success":   return { text: "completed", cls: "text-ok" };
      case "failed":    return { text: `failed${t.exitCode !== null ? ` (${t.exitCode})` : ""}`, cls: "text-err" };
      case "cancelled": return { text: "cancelled", cls: "text-muted" };
    }
  }
</script>

{#if tasks.active}
  {@const t = tasks.active}
  {@const b = badge(t)}
  <Modal title={t.title} wide onClose={() => tasks.hide()}>
    <div class="flex items-center gap-3 px-5 py-2.5 border-b border-border text-xs">
      <span class="flex items-center gap-1.5 {b.cls}">
        <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
        {b.text}
      </span>
      <span class="text-muted font-mono">{t.commandLine}</span>
    </div>
    {#if t.description}
      <div class="px-5 pt-3 text-xs text-muted">{t.description}</div>
    {/if}
    <div
      bind:this={viewport}
      onscroll={onScroll}
      class="font-mono text-[11px] leading-[18px] px-5 py-3"
    >
      {#each t.lines as line, i (i)}
        <div class="whitespace-pre-wrap">{clean(line)}</div>
      {:else}
        <div class="text-muted">
          {t.status === "running" ? "starting…" : "no output"}
        </div>
      {/each}
    </div>

    {#snippet footer()}
      {#if t.status === "running"}
        <Button size="sm" variant="outline" onclick={() => tasks.hide()}>
          Minimize
        </Button>
        <Button size="sm" variant="destructive" onclick={() => tasks.cancel(t.id)}>
          Cancel
        </Button>
      {:else}
        <Button size="sm" variant="ghost" onclick={() => tasks.dismiss(t.id)}>
          Clear
        </Button>
        <Button size="sm" variant="outline" onclick={() => tasks.hide()}>
          Close
        </Button>
      {/if}
    {/snippet}
  </Modal>
{/if}

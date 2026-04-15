<script lang="ts">
  import { confirmDialog } from "../confirm.svelte";
  import Button from "./ui/Button.svelte";

  function onKey(e: KeyboardEvent) {
    if (!confirmDialog.pending) return;
    if (e.key === "Escape") { e.preventDefault(); confirmDialog.answer(false); }
    if (e.key === "Enter")  { e.preventDefault(); confirmDialog.answer(true); }
  }
</script>

<svelte:window on:keydown={onKey} />

{#if confirmDialog.pending}
  {@const p = confirmDialog.pending}
  <div class="fixed inset-0 z-[55] flex items-center justify-center p-6">
    <div aria-hidden="true" class="absolute inset-0 bg-black/70 backdrop-blur-sm"></div>
    <div
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      class="bishop-modal relative border rounded-lg shadow-2xl shadow-black/60 w-full max-w-sm p-5 space-y-4"
    >
      <div>
        <h2 id="confirm-title" class="text-sm font-semibold tracking-tight">{p.title}</h2>
        {#if p.message}
          <p class="text-xs text-muted mt-1.5 leading-relaxed">{p.message}</p>
        {/if}
      </div>
      <div class="flex justify-end gap-2">
        <Button size="sm" variant="outline" onclick={() => confirmDialog.answer(false)}>
          {p.cancelLabel ?? "Cancel"}
        </Button>
        <Button
          size="sm"
          variant={p.destructive ? "destructive" : "default"}
          onclick={() => confirmDialog.answer(true)}
        >{p.confirmLabel ?? "Confirm"}</Button>
      </div>
    </div>
  </div>
{/if}

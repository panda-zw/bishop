<script lang="ts">
  import { toast, type ToastKind } from "../toast.svelte";

  function kindCls(k: ToastKind) {
    switch (k) {
      case "success": return "border-ok/40 bg-ok/10 text-ok";
      case "error":   return "border-err/40 bg-err/10 text-err";
      case "warn":    return "border-warn/40 bg-warn/10 text-warn";
      default:        return "border-border bg-card text-foreground";
    }
  }
</script>

<div class="fixed bottom-4 right-4 z-[60] flex flex-col gap-2 w-[320px] pointer-events-none">
  {#each toast.items as t (t.id)}
    <div
      role="status"
      aria-live="polite"
      class="pointer-events-auto rounded-md border px-3 py-2 shadow-lg shadow-black/20 text-xs {kindCls(t.kind)}"
    >
      <div class="flex items-start gap-2">
        <div class="flex-1 min-w-0">
          <div class="font-medium text-[13px] truncate">{t.title}</div>
          {#if t.body}
            <div class="text-muted mt-0.5 break-words">{t.body}</div>
          {/if}
        </div>
        <button
          class="text-muted hover:text-foreground -mr-1 -mt-0.5 rounded-sm w-5 h-5 inline-flex items-center justify-center"
          onclick={() => toast.dismiss(t.id)}
          aria-label="Dismiss notification"
        >✕</button>
      </div>
    </div>
  {/each}
</div>

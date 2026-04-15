<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
    wide?: boolean;
  }
  let { title, onClose, children, footer, wide = false }: Props = $props();

  // Unique id for aria-labelledby. crypto.randomUUID is fine — Tauri ships a modern webview.
  const titleId = `modal-title-${crypto.randomUUID().slice(0, 8)}`;

  let panel: HTMLDivElement | null = $state(null);
  let previouslyFocused: HTMLElement | null = null;

  function focusables(root: HTMLElement): HTMLElement[] {
    return Array.from(
      root.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter(el => el.offsetParent !== null || el.getClientRects().length > 0);
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") { onClose(); return; }
    if (e.key !== "Tab" || !panel) return;

    const list = focusables(panel);
    if (list.length === 0) { e.preventDefault(); return; }

    const first = list[0];
    const last = list[list.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (e.shiftKey) {
      if (active === first || !panel.contains(active)) { e.preventDefault(); last.focus(); }
    } else {
      if (active === last)  { e.preventDefault(); first.focus(); }
    }
  }

  $effect(() => {
    previouslyFocused = document.activeElement as HTMLElement | null;
    queueMicrotask(() => {
      if (!panel) return;
      const list = focusables(panel);
      // First focusable inside the body (skip the close button as the initial focus target).
      const target = list.find(el => el.getAttribute("aria-label") !== "Close") ?? list[0];
      target?.focus();
    });
    return () => {
      previouslyFocused?.focus?.();
    };
  });
</script>

<svelte:window on:keydown={handleKey} />

<div class="fixed inset-0 z-50 flex items-center justify-center p-6">
  <div aria-hidden="true" class="absolute inset-0 bg-black/70 backdrop-blur-sm"></div>
  <div
    bind:this={panel}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    class="bishop-modal relative border rounded-lg shadow-2xl shadow-black/60 w-full {wide ? 'max-w-4xl' : 'max-w-xl'} max-h-[85vh] flex flex-col"
  >
    <div class="flex items-center justify-between px-5 py-4 border-b border-border">
      <h2 id={titleId} class="text-base font-semibold tracking-tight">{title}</h2>
      <button
        class="text-muted hover:text-foreground rounded-sm w-6 h-6 inline-flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        aria-label="Close"
        onclick={onClose}
      >✕</button>
    </div>
    <div class="flex-1 overflow-auto">
      {@render children()}
    </div>
    {#if footer}
      <div class="px-5 py-3 border-t border-border flex justify-end gap-2">
        {@render footer()}
      </div>
    {/if}
  </div>
</div>

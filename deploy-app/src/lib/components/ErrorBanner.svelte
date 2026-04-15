<script lang="ts">
  /// Structured error banner — renders a BishopError above the raw log.
  /// Frontend is responsible for dispatching the action (open_scaffold,
  /// run_step, etc.) since only the surrounding page knows context like
  /// the current project + env.

  import type { BishopError } from "../types";
  import Button from "./ui/Button.svelte";
  import { dashActions } from "../stores.svelte";

  interface Props {
    error: BishopError;
    /// Called when the user clicks the action button. Override per-surface;
    /// defaults here cover the common deploy flow.
    onAction?: (err: BishopError) => void;
  }

  let { error, onAction }: Props = $props();

  let showRaw = $state(false);

  function defaultAction(err: BishopError) {
    const a = err.action;
    if (!a) return;
    switch (a.kind) {
      case "open_url":
        if (a.payload) window.open(a.payload, "_blank");
        break;
      case "open_scaffold":
        dashActions.openScaffold((a.payload ?? "compose") as any);
        break;
      case "run_step":
        if (a.payload && (a.payload === "setup-server" || a.payload === "setup-app")) {
          dashActions.runStep(a.payload);
        }
        break;
      case "copy":
        if (a.payload) navigator.clipboard.writeText(a.payload).catch(() => {});
        break;
    }
  }

  function handle() {
    (onAction ?? defaultAction)(error);
  }
</script>

<div class="mx-5 mt-3 mb-1 rounded-md border border-err/40 bg-err/5 overflow-hidden">
  <div class="flex items-start gap-3 px-4 py-3">
    <span class="mt-0.5 shrink-0 w-5 h-5 inline-flex items-center justify-center rounded-full bg-err/15 text-err text-xs font-semibold" aria-hidden="true">!</span>
    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-3">
        <div class="text-sm font-medium text-foreground">{error.message}</div>
        <code class="shrink-0 text-[10px] font-mono text-muted/80 mt-0.5">{error.code}</code>
      </div>
      {#if error.hint}
        <div class="mt-1.5 text-xs text-muted leading-relaxed">{error.hint}</div>
      {/if}
      <div class="mt-2.5 flex items-center gap-2 flex-wrap">
        {#if error.action}
          <Button size="sm" onclick={handle}>{error.action.label}</Button>
        {/if}
        {#if error.docs_url}
          <Button size="sm" variant="outline" onclick={() => window.open(error.docs_url!, "_blank")}>
            Read the docs
          </Button>
        {/if}
        {#if error.raw}
          <button
            type="button"
            class="text-[11px] text-muted hover:text-foreground underline-offset-2 hover:underline"
            onclick={() => (showRaw = !showRaw)}
          >
            {showRaw ? "Hide" : "Show"} raw output
          </button>
        {/if}
      </div>
      {#if showRaw && error.raw}
        <pre class="mt-2 max-h-40 overflow-auto rounded bg-background/60 border border-border/60 p-2 text-[10px] leading-[15px] font-mono text-muted whitespace-pre-wrap">{error.raw}</pre>
      {/if}
    </div>
  </div>
</div>

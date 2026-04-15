<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    optional?: boolean;
    hint?: string;
    children: Snippet;
  }
  let { label, optional = false, hint, children }: Props = $props();
</script>

<label class="block space-y-1.5">
  <div class="flex items-baseline gap-1.5">
    <span class="text-xs font-medium">{label}</span>
    {#if optional}
      <span class="text-[10px] text-muted font-normal uppercase tracking-wider">optional</span>
    {/if}
  </div>
  {@render children()}
  {#if hint}
    <div class="text-[11px] text-muted leading-snug">{hint}</div>
  {/if}
</label>

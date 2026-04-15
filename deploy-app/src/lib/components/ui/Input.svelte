<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  interface Props extends HTMLInputAttributes {
    class?: string;
  }
  let { class: klass = "", value = $bindable(), ...rest }: Props = $props();
</script>

<input
  class="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm placeholder:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:border-ring disabled:opacity-50 {klass}"
  bind:value
  {...rest}
/>

<script lang="ts">
  interface Props { class?: string }
  let { class: klass = "" }: Props = $props();
</script>

<div
  role="status"
  aria-label="Loading"
  class="animate-pulse rounded-md bg-secondary/70 {klass}"
></div>

<script lang="ts">
  interface Props { state: string; health?: string | null }
  let { state, health = null }: Props = $props();

  const color = $derived.by(() => {
    if (state === "running" && health !== "unhealthy") return "bg-ok";
    if (state === "restarting" || health === "starting") return "bg-warn";
    if (state === "exited" || health === "unhealthy") return "bg-err";
    return "bg-muted";
  });

  const label = $derived(health ?? state);
</script>

<span
  class="inline-block w-2 h-2 rounded-full {color}"
  role="img"
  aria-label={label}
  title={label}
></span>

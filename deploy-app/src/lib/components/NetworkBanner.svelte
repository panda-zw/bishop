<script lang="ts">
  import { network } from "../network.svelte";
  import { toast } from "../toast.svelte";

  /// Keep a tiny ticker so the "offline for Xs" label updates.
  let now = $state(Date.now());
  let interval: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (!network.online) {
      interval = setInterval(() => (now = Date.now()), 1000);
      return () => { if (interval) clearInterval(interval); interval = null; };
    }
  });

  function elapsed() {
    if (!network.lastOfflineAt) return "";
    const s = Math.max(0, Math.floor((now - network.lastOfflineAt) / 1000));
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60); const rem = s % 60;
    return `${m}m ${rem}s`;
  }

  function tryReconnect() {
    // navigator.onLine can be stale — nudge it by firing our reconnect path.
    // The actual check happens in subscribers (app.probeAll, hosts.refresh, etc.).
    network.markReconnect();
    toast.info("Reconnecting…", "Refreshing server status.");
  }
</script>

{#if !network.online}
  <div
    role="status"
    aria-live="polite"
    class="fixed top-0 inset-x-0 z-[70] flex items-center justify-center gap-3 px-4 py-2 bg-err text-destructive-foreground text-xs font-medium shadow-md"
  >
    <span class="inline-block w-1.5 h-1.5 rounded-full bg-current animate-pulse"></span>
    <span>You're offline — checks and deploys won't reach any server</span>
    <span class="opacity-70 font-mono">({elapsed()})</span>
    <button
      type="button"
      class="ml-2 px-2 py-0.5 rounded-sm bg-background/20 hover:bg-background/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-background/50"
      onclick={tryReconnect}
    >Retry now</button>
  </div>
{/if}

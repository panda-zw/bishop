<script lang="ts">
  import { api, onMetrics } from "../api";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  interface Props {
    projectPath: string;
    env: string;
    appName: string;
  }
  let { projectPath, env, appName }: Props = $props();

  interface Row {
    name: string;
    cpu: string;   // "0.12%"
    mem: string;   // "123MiB / 2GiB"
    memPct: string;
    net: string;
    block: string;
  }

  let rows = $state<Record<string, Row>>({});
  let error = $state<string | null>(null);
  let streamId: string | null = null;
  let unlisten: UnlistenFn | null = null;

  const displayed = $derived.by(() => {
    const prefix = `${appName}-`;
    return Object.values(rows)
      .filter(r => r.name === appName || r.name.startsWith(prefix))
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  async function start() {
    try {
      streamId = await api.startMetrics(projectPath, env);
      unlisten = await onMetrics(streamId, (raw) => {
        try {
          const j: Record<string, string> = JSON.parse(raw);
          const name = j.Name ?? j.Container ?? "";
          if (!name) return;
          rows = {
            ...rows,
            [name]: {
              name,
              cpu: j.CPUPerc ?? "",
              mem: j.MemUsage ?? "",
              memPct: j.MemPerc ?? "",
              net: j.NetIO ?? "",
              block: j.BlockIO ?? "",
            }
          };
        } catch {}
      });
    } catch (e) {
      error = String(e);
    }
  }

  async function stop() {
    if (unlisten) unlisten();
    if (streamId) { try { await api.stopMetrics(streamId); } catch {} streamId = null; }
  }

  $effect(() => {
    projectPath; env;
    rows = {};
    start();
    return () => { stop(); };
  });
</script>

{#if error}
  <div class="text-xs text-err font-mono border border-err/30 bg-err/10 rounded-md px-3 py-2">{error}</div>
{:else if displayed.length === 0}
  <div class="text-xs text-muted">waiting for stats…</div>
{:else}
  <div class="border border-border rounded-lg overflow-hidden bg-card">
    <table class="w-full text-xs font-mono">
      <thead>
        <tr class="text-[10px] uppercase tracking-wider text-muted">
          <th class="text-left px-4 h-8 font-medium">container</th>
          <th class="text-right px-4 h-8 font-medium w-20">cpu</th>
          <th class="text-right px-4 h-8 font-medium w-20">mem%</th>
          <th class="text-right px-4 h-8 font-medium">memory</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border">
        {#each displayed as r (r.name)}
          <tr class="hover:bg-secondary/40 transition-colors">
            <td class="px-4 py-1.5 truncate">{r.name}</td>
            <td class="text-right px-4 py-1.5 tabular-nums">{r.cpu}</td>
            <td class="text-right px-4 py-1.5 tabular-nums">{r.memPct}</td>
            <td class="text-right px-4 py-1.5 tabular-nums">{r.mem}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

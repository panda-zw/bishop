<script lang="ts">
  import { api } from "../api";
  import type { DeployRow } from "../types";
  import Modal from "./Modal.svelte";
  import Skeleton from "./ui/Skeleton.svelte";
  import Button from "./ui/Button.svelte";
  import { dashActions } from "../stores.svelte";

  interface Props {
    projectPath: string;
    env: string;
    onClose: () => void;
  }
  let { projectPath, env, onClose }: Props = $props();

  let rows = $state<DeployRow[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedId = $state<number | null>(null);
  let selectedLog = $state<string | null>(null);
  let logLoading = $state(false);

  async function load() {
    loading = true;
    try {
      rows = await api.listDeploys(projectPath, env);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function viewLog(id: number) {
    selectedId = id;
    selectedLog = null;
    logLoading = true;
    try {
      selectedLog = (await api.getDeployLog(id)) ?? "(empty)";
    } catch (e) {
      selectedLog = `[error: ${e}]`;
    } finally {
      logLoading = false;
    }
  }

  function fmtTime(iso: string) {
    const d = new Date(iso);
    return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  }

  function duration(row: DeployRow) {
    if (!row.finished_at) return "—";
    const s = (new Date(row.finished_at).getTime() - new Date(row.started_at).getTime()) / 1000;
    if (s < 60) return `${s.toFixed(0)}s`;
    const m = Math.floor(s / 60); const rem = Math.round(s % 60);
    return `${m}m ${rem}s`;
  }

  function badgeCls(status: string) {
    if (status === "success") return "text-ok";
    if (status === "failed") return "text-err";
    if (status === "running") return "text-warn";
    return "text-muted";
  }

  // Strip ANSI escapes from historical logs, same as DeployModal.
  const ansiRe = /\x1b\[[0-9;]*m/g;
  function clean(s: string | null) { return (s ?? "").replace(ansiRe, ""); }

  $effect(() => { load(); });
</script>

<Modal title="Deploy history — {env}" wide {onClose}>
  <div class="flex h-[65vh]">
    <div class="w-1/2 overflow-auto border-r border-border">
      {#if loading}
        <div class="divide-y divide-border" aria-busy="true" aria-label="Loading deploy history">
          {#each Array(4) as _, i (i)}
            <div class="px-4 py-3 space-y-2">
              <div class="flex items-center justify-between gap-2">
                <Skeleton class="h-3.5 w-36" />
                <Skeleton class="h-3 w-14" />
              </div>
              <div class="flex gap-3">
                <Skeleton class="h-3 w-10" />
              </div>
            </div>
          {/each}
        </div>
      {:else if error}
        <div class="p-5 text-xs text-err font-mono">{error}</div>
      {:else if rows.length === 0}
        <div class="flex flex-col items-center text-center p-8 gap-3">
          <div class="w-10 h-10 rounded-lg border border-border bg-card flex items-center justify-center">
            <svg viewBox="0 0 24 24" class="w-5 h-5 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 8v4l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"/>
            </svg>
          </div>
          <div class="space-y-1">
            <div class="text-sm font-medium">No deploys yet</div>
            <p class="text-xs text-muted leading-relaxed max-w-[240px]">
              Once you deploy this environment, every run will appear here with its logs.
            </p>
          </div>
          <Button size="sm" onclick={() => { onClose(); dashActions.deploy(); }}>
            Deploy {env}
          </Button>
        </div>
      {:else}
        {#each rows as row (row.id)}
          <button
            class="w-full text-left px-4 py-3 border-b border-border transition-colors {selectedId === row.id ? 'bg-secondary' : 'hover:bg-secondary/50'}"
            onclick={() => viewLog(row.id)}
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm font-medium">{fmtTime(row.started_at)}</span>
              <span class="text-[11px] flex items-center gap-1.5 {badgeCls(row.status)}">
                <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
                {row.status}
              </span>
            </div>
            <div class="text-[11px] text-muted flex gap-3 mt-1">
              <span>{duration(row)}</span>
              {#if row.exit_code !== null && row.exit_code !== 0}
                <span class="text-err">exit {row.exit_code}</span>
              {/if}
            </div>
          </button>
        {/each}
      {/if}
    </div>
    <div class="w-1/2 overflow-auto">
      {#if selectedId === null}
        <div class="p-5 text-xs text-muted">Select a deploy to view its log.</div>
      {:else if logLoading}
        <div class="p-5 text-xs text-muted">Loading log…</div>
      {:else}
        <pre class="font-mono text-[11px] leading-[18px] px-4 py-3 whitespace-pre-wrap break-words">{clean(selectedLog)}</pre>
      {/if}
    </div>
  </div>
</Modal>

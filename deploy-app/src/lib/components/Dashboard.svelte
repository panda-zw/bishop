<script lang="ts">
  import { app, dashActions } from "../stores.svelte";
  import { api } from "../api";
  import type { Container } from "../types";
  import ContainerList from "./ContainerList.svelte";
  import LogViewer from "./LogViewer.svelte";
  import EnvVarsModal from "./EnvVarsModal.svelte";
  import HistoryModal from "./HistoryModal.svelte";
  import MetricsPanel from "./MetricsPanel.svelte";
  import EditRemoteModal from "./EditRemoteModal.svelte";
  import ScaffoldModal from "./ScaffoldModal.svelte";
  import Button from "./ui/Button.svelte";
  import { confirmDialog } from "../confirm.svelte";
  import { toast } from "../toast.svelte";

  const PALETTE = ["#3b82f6", "#22c55e", "#f59e0b", "#ef4444", "#a855f7", "#06b6d4", "#ec4899"];

  let containers = $state<Container[]>([]);
  let selected = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let loading = $state(false);
  let showMetrics = $state(false);
  let restartingName = $state<string | null>(null);
  let setupOpen = $state(false);

  /// Close a dropdown when clicking outside its root.
  function closeOnClickOutside(node: HTMLElement, cb: () => void) {
    const handler = (e: MouseEvent) => {
      if (!node.contains(e.target as Node)) cb();
    };
    document.addEventListener("mousedown", handler);
    return { destroy() { document.removeEventListener("mousedown", handler); } };
  }
  /// Monotonic token so in-flight refreshes tied to a stale (project, env) combo get ignored.
  let epoch = 0;

  async function restartOne(name: string) {
    const proj = app.activeProject; const env = app.activeEnv;
    if (!proj || !env) return;
    const ok = await confirmDialog.ask({
      title: `Restart ${name}?`,
      message: "The container will stop and restart. In-flight requests may be dropped.",
      confirmLabel: "Restart",
      destructive: true,
    });
    if (!ok) return;
    restartingName = name;
    try {
      await api.restartService(proj.path, env.name, name);
      await refresh(proj.path, env.name, epoch);
      toast.success(`Restarted ${name}`);
    } catch (e) {
      toast.error("Restart failed", String(e));
    } finally {
      restartingName = null;
    }
  }
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  const selectedList = $derived(Array.from(selected));

  function accentFor(name: string) {
    const idx = selectedList.indexOf(name);
    return PALETTE[idx >= 0 ? idx % PALETTE.length : 0];
  }

  function toggleSelected(name: string) {
    const next = new Set(selected);
    if (next.has(name)) next.delete(name); else next.add(name);
    selected = next;
  }

  async function refresh(projectPath: string, envName: string, myEpoch: number) {
    try {
      const data = await api.getContainers(projectPath, envName);
      if (myEpoch !== epoch) return;  // stale response — discard
      containers = data;
      error = null;
      if (selected.size === 0 && data.length > 0) {
        selected = new Set([data[0].name]);
      }
    } catch (e) {
      if (myEpoch !== epoch) return;
      error = String(e);
    } finally {
      if (myEpoch === epoch) loading = false;
    }
  }

  $effect(() => {
    const proj = app.activeProject;
    const env = app.activeEnv;
    // Invalidate any pending responses from a previous (project, env).
    const myEpoch = ++epoch;
    if (pollHandle) { clearInterval(pollHandle); pollHandle = null; }
    containers = [];
    selected = new Set();
    error = null;
    if (!proj || !env) { loading = false; return; }
    loading = true;
    refresh(proj.path, env.name, myEpoch);
    pollHandle = setInterval(() => refresh(proj.path, env.name, myEpoch), 5000);
    return () => { if (pollHandle) clearInterval(pollHandle); };
  });
</script>

{#if !app.activeProject || !app.activeEnv}
  <div class="flex-1 flex items-center justify-center p-10">
    <div class="max-w-sm text-center space-y-5">
      <div class="mx-auto w-12 h-12 rounded-lg border border-border bg-card flex items-center justify-center">
        <svg viewBox="0 0 24 24" class="w-6 h-6 text-muted" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3 7.5l9-4.5 9 4.5M3 7.5l9 4.5 9-4.5M3 7.5v9l9 4.5M21 7.5v9l-9 4.5M12 12v9"/>
        </svg>
      </div>
      <div class="space-y-1.5">
        <h2 class="text-base font-semibold">No project selected</h2>
        <p class="text-sm text-muted leading-relaxed">
          Add a project folder containing <span class="font-mono text-foreground">.deploy/</span>,
          or set one up from scratch.
        </p>
      </div>
      <div class="flex justify-center gap-2">
        <Button size="sm" onclick={() => dashActions.openInit()}>Setup new project</Button>
        <Button size="sm" variant="outline" onclick={() => dashActions.openSettings()}>Open settings</Button>
      </div>
    </div>
  </div>
{:else}
  <div class="flex-1 flex flex-col p-6 gap-6 overflow-hidden">
    <header class="flex items-center justify-between gap-4">
      <div class="min-w-0">
        <h1 class="text-lg font-semibold tracking-tight truncate">
          {app.activeProject.name}
          <span class="text-muted font-normal">/</span>
          {app.activeEnv.name}
        </h1>
        <div class="text-xs text-muted font-mono flex items-center gap-2 mt-0.5 flex-wrap">
          <span>{app.activeEnv.ssh_user}@{app.activeEnv.ssh_host}</span>
          <span class="text-muted/60">·</span>
          <button
            class="hover:text-foreground"
            onclick={() => {
              // Open a terminal tab directly in the app's remote folder.
              import("../terminals.svelte").then(({ terminals }) => {
                const env = app.activeEnv!;
                terminals.addOrFocusTab(
                  { user: env.ssh_user, host: env.ssh_host, initial_cwd: env.app_dir },
                  `${app.activeProject!.name} · ${env.name}`,
                  `${env.ssh_user}@${env.ssh_host}`,
                );
              });
            }}
            title="Open a terminal in this folder on the server"
          >{app.activeEnv.app_dir}</button>
          <span class="text-muted/60">·</span>
          <button class="hover:text-foreground underline underline-offset-4" onclick={() => (dashActions.showEditRemote = true)}>edit</button>
        </div>
      </div>
      <div class="flex items-center gap-1.5 shrink-0">
        <Button size="sm" onclick={() => dashActions.deploy()} title="Build and release the latest code to this environment">Deploy</Button>
        <Button size="sm" variant="outline" onclick={() => dashActions.restart()} disabled={dashActions.restarting} title="Restart the main app container without re-deploying">
          {dashActions.restarting ? "Restarting…" : "Restart"}
        </Button>
        <Button size="sm" variant="outline" onclick={() => dashActions.openEnv()} title="View and edit environment variables on the server">Env vars</Button>
        <Button size="sm" variant="outline" onclick={() => dashActions.openHistory()} title="See past deploys and their logs">History</Button>
        <Button size="sm" variant="outline" onclick={() => dashActions.runCheck()} title="Run a health check against the server">Check</Button>
        <span class="w-px h-5 bg-border mx-0.5" aria-hidden="true"></span>
        <Button size="sm" variant="ghost" onclick={() => dashActions.openTerminal()} title="Open an SSH terminal to this server">Terminal</Button>
        <div class="relative" use:closeOnClickOutside={() => (setupOpen = false)}>
          <Button
            size="sm"
            variant="ghost"
            aria-haspopup="menu"
            aria-expanded={setupOpen}
            onclick={() => (setupOpen = !setupOpen)}
          >
            Setup
            <svg viewBox="0 0 10 10" class="w-2.5 h-2.5 text-muted" fill="currentColor" aria-hidden="true">
              <path d="M2 3.5l3 3 3-3H2z"/>
            </svg>
          </Button>
          {#if setupOpen}
            <div
              role="menu"
              class="absolute right-0 top-full mt-1 w-52 bishop-modal border rounded-md shadow-lg py-1 z-10"
            >
              <button
                role="menuitem"
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-secondary"
                onclick={() => { setupOpen = false; dashActions.runStep('setup-server'); }}
              >
                <div>Prepare this server</div>
                <div class="text-[10px] text-muted">Installs Docker, Traefik, Postgres, Redis on the host. Safe to re-run.</div>
              </button>
              <button
                role="menuitem"
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-secondary"
                onclick={() => { setupOpen = false; dashActions.runStep('setup-app'); }}
              >
                <div>Prepare this app on the server</div>
                <div class="text-[10px] text-muted">Creates the app folder and writes its .env on the server.</div>
              </button>
              <div class="border-t border-border my-1"></div>
              <button
                role="menuitem"
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-secondary"
                onclick={() => { setupOpen = false; dashActions.openScaffold('compose'); }}
              >
                <div>Scaffold deployment files</div>
                <div class="text-[10px] text-muted">Create or edit docker-compose.prod.yml and Dockerfile.</div>
              </button>
            </div>
          {/if}
        </div>
      </div>
    </header>

    {#if error}
      <div class="text-xs text-err border border-err/30 bg-err/10 rounded-md px-3 py-2 font-mono">{error}</div>
    {/if}

    <section>
      <div class="flex items-baseline justify-between mb-2.5">
        <div class="text-[10px] font-medium uppercase tracking-wider text-muted">Containers</div>
        <div class="text-xs text-muted flex items-center gap-3">
          <span>{selected.size} selected</span>
          <span class="text-muted/60">·</span>
          <button class="hover:text-foreground underline underline-offset-4" onclick={() => (showMetrics = !showMetrics)}>
            {showMetrics ? "hide metrics" : "show metrics"}
          </button>
        </div>
      </div>
      <ContainerList
        {containers}
        {selected}
        {loading}
        onToggle={toggleSelected}
        onRestart={restartOne}
        {restartingName}
      />
    </section>

    {#if showMetrics}
      <section>
        <div class="text-[10px] font-medium uppercase tracking-wider text-muted mb-2.5">Metrics</div>
        <MetricsPanel
          projectPath={app.activeProject.path}
          env={app.activeEnv.name}
          appName={app.activeEnv.app_name}
        />
      </section>
    {/if}

    {#if selectedList.length > 0}
      <section class="flex-1 min-h-0 grid gap-3" style="grid-template-rows: repeat({selectedList.length}, minmax(0, 1fr));">
        {#each selectedList as name (name)}
          <LogViewer
            projectPath={app.activeProject.path}
            env={app.activeEnv.name}
            service={name}
            accent={accentFor(name)}
            onClose={() => toggleSelected(name)}
          />
        {/each}
      </section>
    {/if}
  </div>
{/if}

{#if dashActions.showEnv && app.activeProject && app.activeEnv}
  <EnvVarsModal
    projectPath={app.activeProject.path}
    env={app.activeEnv.name}
    onClose={() => (dashActions.showEnv = false)}
  />
{/if}

{#if dashActions.showHistory && app.activeProject && app.activeEnv}
  <HistoryModal
    projectPath={app.activeProject.path}
    env={app.activeEnv.name}
    onClose={() => (dashActions.showHistory = false)}
  />
{/if}


{#if dashActions.showEditRemote && app.activeProject && app.activeEnv}
  <EditRemoteModal
    project={app.activeProject}
    env={app.activeEnv}
    onClose={() => (dashActions.showEditRemote = false)}
  />
{/if}

{#if dashActions.showScaffold && app.activeProject && app.activeEnv}
  <ScaffoldModal
    projectPath={app.activeProject.path}
    appName={app.activeEnv.app_name}
    envName={app.activeEnv.name}
    gitRepo={app.activeProject.git_repo}
    domain={app.activeEnv.domain}
    appPort={3000}
    extraContainers={""}
    initialTab={dashActions.scaffoldInitialTab}
    onClose={() => (dashActions.showScaffold = false)}
  />
{/if}

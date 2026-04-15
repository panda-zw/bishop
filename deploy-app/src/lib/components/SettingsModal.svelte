<script lang="ts">
  import { app } from "../stores.svelte";
  import { api } from "../api";
  import { theme } from "../theme.svelte";
  import { confirmDialog } from "../confirm.svelte";
  import { toast } from "../toast.svelte";
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";

  interface Props { onClose: () => void }
  let { onClose }: Props = $props();

  let opStatus = $state<{ installed: boolean; signed_in: boolean } | null>(null);

  async function remove(path: string) {
    const ok = await confirmDialog.ask({
      title: "Remove project from Bishop?",
      message: "Bishop will stop tracking this project. Files on disk are untouched.",
      confirmLabel: "Remove",
      destructive: true,
    });
    if (!ok) return;
    try {
      await api.removeProject(path);
      app.projects = app.projects.filter(p => p.path !== path);
      api.refreshTray().catch(() => {});
      if (app.activeProject?.path === path) {
        app.selectProject(app.projects[0] ?? { path: "", name: "", git_repo: null, remotes: [] });
        if (!app.projects[0]) { app.activeProject = null; app.activeEnv = null; }
      }
      toast.success("Project removed");
    } catch (e) { toast.error("Remove failed", String(e)); }
  }

  async function refreshOp() {
    try { opStatus = await api.opStatus(); } catch { opStatus = { installed: false, signed_in: false }; }
  }

  let scriptStatus = $state<Record<string, boolean>>({});
  async function refreshScriptStatus() {
    const next: Record<string, boolean> = {};
    for (const p of app.projects) {
      try { next[p.path] = await api.hasLocalDeployScript(p.path); }
      catch { next[p.path] = false; }
    }
    scriptStatus = next;
  }

  async function installScript(path: string) {
    try {
      await api.installDeployScript(path);
      toast.success("Deploy script installed");
      await refreshScriptStatus();
    } catch (e) {
      toast.error("Install failed", String(e));
    }
  }

  $effect(() => { refreshOp(); refreshScriptStatus(); });
</script>

<Modal title="Settings" {onClose}>
  <div class="p-5 space-y-6">
    <section class="space-y-2.5">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-muted">Projects</h3>
      {#if app.projects.length === 0}
        <div class="text-xs text-muted">No projects yet. Add one from the sidebar.</div>
      {:else}
        <div class="space-y-1.5">
          {#each app.projects as p (p.path)}
            <div class="flex items-center gap-3 px-3 py-2 border border-border rounded-md bg-card">
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium">{p.name}</div>
                <div class="text-xs text-muted font-mono truncate mt-0.5">{p.path}</div>
                <div class="text-[11px] mt-1 flex items-center gap-1.5">
                  <span class="inline-block w-1.5 h-1.5 rounded-full {scriptStatus[p.path] ? 'bg-ok' : 'bg-warn'}"></span>
                  <span class="text-muted">
                    {scriptStatus[p.path] ? "deploy script installed" : "using bundled deploy script"}
                  </span>
                </div>
              </div>
              <Button size="sm" variant="outline" onclick={() => installScript(p.path)}>
                {scriptStatus[p.path] ? "Reinstall" : "Install"}
              </Button>
              <Button size="sm" variant="ghost" class="text-err hover:text-err hover:bg-err/10" onclick={() => remove(p.path)}>Remove</Button>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="space-y-2.5">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-muted">1Password</h3>
      {#if !opStatus}
        <div class="text-xs text-muted">Checking…</div>
      {:else if !opStatus.installed}
        <div class="text-xs">
          <span class="text-err font-medium">Not installed.</span>
          <span class="text-muted">Install the 1Password CLI (<span class="font-mono text-foreground">op</span>) to sync secrets into env vars.</span>
        </div>
      {:else if !opStatus.signed_in}
        <div class="text-xs">
          <span class="text-warn font-medium">Installed, not signed in.</span>
          <span class="text-muted">Run <span class="font-mono text-foreground">op signin</span> in a terminal.</span>
        </div>
      {:else}
        <div class="text-xs text-ok">
          Signed in. Pull values via <span class="font-mono">op://</span> references in the env var editor.
        </div>
      {/if}
      <Button size="sm" variant="ghost" class="text-muted" onclick={refreshOp}>Re-check</Button>
    </section>

    <section class="space-y-2.5">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-muted">Appearance</h3>
      <div class="flex gap-2">
        <Button size="sm" variant={theme.current === 'dark' ? 'default' : 'outline'} onclick={() => theme.set('dark')}>Dark</Button>
        <Button size="sm" variant={theme.current === 'light' ? 'default' : 'outline'} onclick={() => theme.set('light')}>Light</Button>
      </div>
    </section>

    <section class="space-y-2.5">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-muted">Keyboard shortcuts</h3>
      <div class="text-xs grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5">
        <kbd class="font-mono text-muted">⌘K</kbd><span>Command palette</span>
        <kbd class="font-mono text-muted">⌘D</kbd><span>Deploy</span>
        <kbd class="font-mono text-muted">⌘R</kbd><span>Restart</span>
        <kbd class="font-mono text-muted">⌘E</kbd><span>Env vars</span>
        <kbd class="font-mono text-muted">⌘T</kbd><span>Open terminal for this env</span>
        <kbd class="font-mono text-muted">⌘B</kbd><span>Collapse / expand sidebar</span>
        <kbd class="font-mono text-muted">⌘J</kbd><span>Toggle terminal panel</span>
        <kbd class="font-mono text-muted">⌘\</kbd><span>Maximize / restore terminal</span>
        <kbd class="font-mono text-muted">⌘1–9</kbd><span>Switch terminal tab (or env)</span>
      </div>
    </section>

    <section class="space-y-2.5">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-muted">About</h3>
      <div class="text-xs text-muted">
        Bishop v0.1 — desktop companion for the <span class="font-mono text-foreground">./deploy</span> CLI.
      </div>
    </section>
  </div>
</Modal>

<script lang="ts">
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Dashboard from "./lib/components/Dashboard.svelte";
  import CommandPalette from "./lib/components/CommandPalette.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import InitWizard from "./lib/components/InitWizard.svelte";
  import OnboardingWizard from "./lib/components/OnboardingWizard.svelte";
  import Toaster from "./lib/components/Toaster.svelte";
  import ConfirmDialog from "./lib/components/ConfirmDialog.svelte";
  import TaskModal from "./lib/components/TaskModal.svelte";
  import BackgroundTasksBar from "./lib/components/BackgroundTasksBar.svelte";
  import TerminalPanel from "./lib/components/TerminalPanel.svelte";
  import NetworkBanner from "./lib/components/NetworkBanner.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import { network } from "./lib/network.svelte";
  import { updater } from "./lib/updater.svelte";
  import { app } from "./lib/stores.svelte";
  import { dashActions } from "./lib/stores.svelte";
  import { palette, type PaletteAction } from "./lib/palette.svelte";
  import { api } from "./lib/api";
  import { theme } from "./lib/theme.svelte";
  import { hosts } from "./lib/hosts.svelte";
  import { terminals } from "./lib/terminals.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { toast } from "./lib/toast.svelte";
  import { tasks } from "./lib/tasks.svelte";
  import { onMount } from "svelte";

  let probeHandle: ReturnType<typeof setInterval> | null = null;

  /// Push a one-line status summary to the tray tooltip whenever the underlying
  /// state changes. The tooltip is visible when the user hovers the tray icon.
  $effect(() => {
    const running = tasks.running.length;
    const projCount = app.projects.length;
    let summary = "Bishop";
    if (running > 0) summary += `  ·  ${running} running`;
    if (projCount > 0) summary += `  ·  ${projCount} project${projCount === 1 ? "" : "s"}`;
    api.setTrayTooltip(summary).catch(() => {});
  });

  onMount(() => {
    theme.apply();
    const unlisteners: Array<() => void> = [];
    (async () => {
      try {
        app.projects = await api.listProjects();
        if (app.projects[0]) app.selectProject(app.projects[0]);
        // First-run: no projects ever configured → open onboarding. The user
        // can always dismiss via "Skip for now" and reopen from Settings.
        if (app.projects.length === 0) dashActions.showOnboarding = true;
      } catch (e) { console.error(e); }
      app.probeAll();
      probeHandle = setInterval(() => app.probeAll(), 30000);
      await hosts.refresh();
      // Seed the tray with the current state.
      api.refreshTray().catch(() => {});

      // Fire-and-forget update check — the banner appears only if one's available.
      // A delay keeps us out of the launch critical path on slow networks.
      setTimeout(() => { void updater.check(); }, 3000);

      // On network reconnect, re-run every status probe immediately. The 30s
      // background poll continues as a safety net but this makes the UI
      // responsive to a wifi drop/recover cycle.
      unlisteners.push(
        network.onReconnect(() => {
          toast.success("Back online", "Refreshing server status…");
          void app.probeAll();
          void hosts.refresh();
        }),
      );

      // --- Tray event plumbing ---
      unlisteners.push(await listen<string>("tray:open-project", (e) => {
        const p = app.projects.find(x => x.path === e.payload);
        if (p) app.selectProject(p);
      }));
      unlisteners.push(await listen<{ project_path: string; env: string }>("tray:open-env", (e) => {
        const p = app.projects.find(x => x.path === e.payload.project_path);
        if (!p) return;
        app.selectProject(p);
        const env = p.remotes.find(x => x.name === e.payload.env);
        if (env) app.selectEnv(env);
      }));
      unlisteners.push(await listen<string>("tray:connect-host", (e) => {
        const saved = hosts.saved.find(h => h.id === e.payload);
        if (!saved) { toast.error("Host not found"); return; }
        terminals.addOrFocusTab(
          { user: saved.user, host: saved.host, port: saved.port ?? null,
            identity: saved.identity ?? null, proxy_jump: saved.proxy_jump ?? null,
            initial_cwd: saved.initial_cwd ?? null,
            use_tmux: saved.use_tmux ?? false, tmux_session: saved.tmux_session ?? saved.label },
          saved.label,
          `${saved.user}@${saved.host}`,
        );
      }));
      unlisteners.push(await listen<string>("tray:connect-alias", (e) => {
        terminals.addOrFocusTab({ alias: e.payload }, e.payload, e.payload);
      }));
    })();
    return () => {
      if (probeHandle) clearInterval(probeHandle);
      for (const fn of unlisteners) { try { fn(); } catch {} }
    };
  });

  function onKey(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;

    const k = e.key.toLowerCase();

    // App-control shortcuts — always fire, even when focus is inside a terminal
    // or an input. These manipulate panels/sidebar/palette, not text.
    if (k === "k") { e.preventDefault(); palette.toggle(); return; }
    if (k === "j") { e.preventDefault(); terminals.toggle(); return; }
    if (k === "b") { e.preventDefault(); app.toggleSidebar(); return; }
    if (k === "\\") { e.preventDefault(); terminals.toggleMaximize(); return; }

    // Everything below manipulates a modal/action tied to dashboard focus —
    // suppress when the user is typing so we don't hijack text input.
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;

    if (k === "d") { e.preventDefault(); dashActions.deploy(); return; }
    if (k === "r") { e.preventDefault(); dashActions.restart(); return; }
    if (k === "e") { e.preventDefault(); dashActions.openEnv(); return; }
    if (k === "t") { e.preventDefault(); dashActions.openTerminal(); return; }

    // ⌘1…⌘9 — when terminal panel is open, switch its tab. Otherwise switch env.
    if (/^[1-9]$/.test(k)) {
      if (terminals.open && terminals.tabs.length > 0) {
        const tab = terminals.tabs[Number(k) - 1];
        if (tab) { e.preventDefault(); terminals.setActive(tab.id); return; }
      }
    }

    // ⌘1-9 — switch environment by index within the active project.
    if (/^[1-9]$/.test(k) && app.activeProject) {
      const idx = Number(k) - 1;
      const env = app.activeProject.remotes[idx];
      if (env) { e.preventDefault(); app.selectEnv(env); }
    }
  }

  const paletteActions = $derived.by<PaletteAction[]>(() => {
    const out: PaletteAction[] = [];

    if (app.activeProject && app.activeEnv) {
      out.push(
        { id: "deploy",  label: `Deploy → ${app.activeEnv.name}`, hint: "⌘D", group: "actions", run: () => dashActions.deploy() },
        { id: "restart", label: `Restart app on ${app.activeEnv.name}`, hint: "⌘R", group: "actions", run: () => dashActions.restart() },
        { id: "env",     label: `Env vars — ${app.activeEnv.name}`, hint: "⌘E", group: "actions", run: () => dashActions.openEnv() },
        { id: "history", label: `Deploy history — ${app.activeEnv.name}`, group: "actions", run: () => dashActions.openHistory() },
        { id: "check",   label: `Health check — ${app.activeEnv.name}`, group: "actions", run: () => dashActions.runCheck() },
        { id: "term",    label: `Terminal — ${app.activeEnv.name}`, hint: "⌘T", group: "actions", run: () => dashActions.openTerminal() },
        { id: "setup-server", label: `Set up server — ${app.activeEnv.name}`, group: "setup", run: () => dashActions.runStep("setup-server") },
        { id: "setup-app",    label: `Set up app — ${app.activeEnv.name}`,    group: "setup", run: () => dashActions.runStep("setup-app") },
        { id: "scaffold",     label: `Scaffold deployment files — ${app.activeProject.name}`, group: "setup", run: () => dashActions.openScaffold("compose") },
      );
    }

    if (app.activeProject) {
      for (const env of app.activeProject.remotes) {
        if (env.name === app.activeEnv?.name) continue;
        out.push({
          id: `env:${env.name}`,
          label: `Switch to ${env.name}`,
          hint: env.ssh_host,
          group: "environments",
          run: () => app.selectEnv(env),
        });
      }
    }

    out.push(
      { id: "settings", label: "Settings", group: "app", run: () => dashActions.openSettings() },
      { id: "init", label: "Setup new project", group: "app", run: () => dashActions.openInit() },
      { id: "onboarding", label: "Take the onboarding tour", group: "app", run: () => { dashActions.showOnboarding = true; } },
    );

    // Local terminal actions — always first in the terminals group.
    out.push({
      id: "local-shell",
      label: "New local shell",
      hint: "~",
      group: "terminals",
      run: () => { terminals.addOrFocusTab({ local: true }, "Local shell", "~"); },
    });
    if (app.activeProject) {
      out.push({
        id: "local-shell-here",
        label: `Local shell in ${app.activeProject.name}`,
        hint: app.activeProject.path,
        group: "terminals",
        run: () => {
          terminals.addOrFocusTab(
            { local: true, initial_cwd: app.activeProject!.path },
            `Local · ${app.activeProject!.name}`,
            app.activeProject!.path,
          );
        },
      });
    }

    // One-click terminal entries for every saved host + ssh_config host.
    const hostGroups = hosts.aggregated();
    for (const h of [...hostGroups.saved, ...hostGroups.sshConfig]) {
      out.push({
        id: `host:${h.key}`,
        label: `Terminal → ${h.label}`,
        hint: h.subtitle,
        group: "terminals",
        run: () => { terminals.addOrFocusTab(h.target, h.label, h.subtitle); },
      });
    }

    for (const p of app.projects) {
      if (p.path === app.activeProject?.path) continue;
      out.push({
        id: `proj:${p.path}`,
        label: `Open project: ${p.name}`,
        hint: p.path,
        group: "projects",
        run: () => app.selectProject(p),
      });
    }

    return out;
  });
</script>

<svelte:window onkeydown={onKey} />

<UpdateBanner />

<div class="h-full flex border-t border-border">
  <Sidebar />
  <main class="flex-1 flex flex-col overflow-hidden min-w-0">
    <div class="flex-1 flex flex-col overflow-hidden min-h-0 {terminals.maximized ? 'hidden' : ''}">
      <Dashboard />
    </div>
    <TerminalPanel />
  </main>
</div>

<CommandPalette actions={paletteActions} />
<NetworkBanner />
<Toaster />
<ConfirmDialog />
<TaskModal />
<BackgroundTasksBar />

{#if dashActions.showSettings}
  <SettingsModal onClose={() => (dashActions.showSettings = false)} />
{/if}

{#if dashActions.showInit}
  <InitWizard onClose={() => (dashActions.showInit = false)} />
{/if}

{#if dashActions.showOnboarding}
  <OnboardingWizard onClose={() => (dashActions.showOnboarding = false)} />
{/if}

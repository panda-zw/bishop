/**
 * Factory-style Svelte 5 stores.
 *
 * We deliberately avoid classes with `$state` class fields — that pattern
 * fails under Vite HMR because each module reload compiles a fresh class
 * definition whose `$state` signals live in a new runtime, leaving older
 * component subscriptions pointing at the previous runtime's signals.
 *
 * A factory closure keeps state in plain closure-captured variables proxied
 * through a single `$state({...})` object. The exported object's getters and
 * setters forward to that one proxy. Templates always read through the proxy
 * getter, so reactivity never fractures across compilations.
 */

import type { Project, Environment } from "./types";
import { api } from "./api";
import { confirmDialog } from "./confirm.svelte";
import { toast } from "./toast.svelte";
import { tasks } from "./tasks.svelte";

/// Environment health states shown by the sidebar dots.
///   unknown  — haven't checked yet (initial load, or checking in progress)
///   up       — app container is running and healthy
///   starting — app container is restarting or still in "starting" health
///   missing  — SSH works but no containers with the expected name exist
///   down     — SSH unreachable, or app container exited
type EnvStatus = "unknown" | "up" | "starting" | "missing" | "down";

function createAppState() {
  const state = $state({
    projects: [] as Project[],
    activeProject: null as Project | null,
    activeEnv: null as Environment | null,
    /// Keyed by `${projectPath}::${envName}`.
    envStatuses: {} as Record<string, EnvStatus>,
    sidebarCollapsed: false,
  });

  // Hydrate from localStorage — the reactive proxy is already installed.
  try {
    state.sidebarCollapsed = localStorage.getItem("bishop-sidebar-collapsed") === "1";
  } catch {}


  function reachKey(projectPath: string, envName: string) {
    return `${projectPath}::${envName}`;
  }

  const self = {
    // ---- reactive accessors ----
    get projects() { return state.projects; },
    set projects(v: Project[]) { state.projects = v; },

    get activeProject() { return state.activeProject; },
    set activeProject(v: Project | null) { state.activeProject = v; },

    get activeEnv() { return state.activeEnv; },
    set activeEnv(v: Environment | null) { state.activeEnv = v; },

    get envStatuses() { return state.envStatuses; },

    get sidebarCollapsed() { return state.sidebarCollapsed; },

    // ---- methods ----
    selectProject(p: Project) {
      state.activeProject = p;
      state.activeEnv = p.remotes[0] ?? null;
    },

    selectEnv(e: Environment) {
      state.activeEnv = e;
    },

    reachKey,

    /// Back-compat alias — earlier callers used reachFor(); now it returns
    /// the derived EnvStatus.
    reachFor(projectPath: string, envName: string): EnvStatus {
      return self.statusFor(projectPath, envName);
    },

    statusFor(projectPath: string, envName: string): EnvStatus {
      return state.envStatuses[reachKey(projectPath, envName)] ?? "unknown";
    },

    async probeAll() {
      for (const proj of state.projects) {
        for (const env of proj.remotes) {
          void self.probeOne(proj.path, env.name);
        }
      }
    },

    /// Probe one (project, env) pair. Uses `docker ps` over SSH so the result
    /// doubles as a reachability check AND an application-health signal.
    async probeOne(projectPath: string, envName: string) {
      const key = reachKey(projectPath, envName);
      const project = state.projects.find(p => p.path === projectPath);
      const env = project?.remotes.find(e => e.name === envName);
      if (!env) return;

      try {
        const containers = await api.getContainers(projectPath, envName);
        const appContainer =
          containers.find(c => c.name === `${env.app_name}-app`)
          ?? containers.find(c => c.name.endsWith("-app"))
          ?? null;

        let status: EnvStatus;
        if (containers.length === 0 || !appContainer) {
          status = "missing";
        } else if (appContainer.state === "restarting" || appContainer.health === "starting") {
          status = "starting";
        } else if (appContainer.state === "running"
          && (appContainer.health === "healthy" || appContainer.health === null)) {
          status = "up";
        } else {
          status = "down";
        }

        state.envStatuses = { ...state.envStatuses, [key]: status };
      } catch {
        // SSH unreachable or docker ps errored — surface as down.
        state.envStatuses = { ...state.envStatuses, [key]: "down" };
      }
    },

    setSidebarCollapsed(v: boolean) {
      state.sidebarCollapsed = v;
      try { localStorage.setItem("bishop-sidebar-collapsed", v ? "1" : "0"); } catch {}
    },

    toggleSidebar() { self.setSidebarCollapsed(!state.sidebarCollapsed); },
  };

  return self;
}

export const app = createAppState();
export type AppStore = ReturnType<typeof createAppState>;

/// UI flags for modals + imperative actions the command palette can trigger.
function createDashActions() {
  const state = $state({
    showEnv: false,
    showHistory: false,
    showSettings: false,
    showInit: false,
    showEditRemote: false,
    showScaffold: false,
    scaffoldInitialTab: "compose" as
      | "compose" | "dockerfile" | "dockerignore"
      | "sharedCompose" | "sharedTraefik" | "sharedInitDb",
    restarting: false,
  });

  const self = {
    get showEnv() { return state.showEnv; },
    set showEnv(v: boolean) { state.showEnv = v; },

    get showHistory() { return state.showHistory; },
    set showHistory(v: boolean) { state.showHistory = v; },

    get showSettings() { return state.showSettings; },
    set showSettings(v: boolean) { state.showSettings = v; },

    get showInit() { return state.showInit; },
    set showInit(v: boolean) { state.showInit = v; },

    get showEditRemote() { return state.showEditRemote; },
    set showEditRemote(v: boolean) { state.showEditRemote = v; },

    get showScaffold() { return state.showScaffold; },
    set showScaffold(v: boolean) { state.showScaffold = v; },
    get scaffoldInitialTab() { return state.scaffoldInitialTab; },

    openScaffold(
      initialTab: typeof state.scaffoldInitialTab = "compose",
    ) {
      if (!app.activeProject) return;
      state.scaffoldInitialTab = initialTab;
      state.showScaffold = true;
    },

    get restarting() { return state.restarting; },

    async deploy() {
      if (!app.activeProject || !app.activeEnv) return;
      const env = app.activeEnv;

      // Prerequisite check — the CLI scp's several files to the server during
      // every deploy. Missing any of them produces a cryptic scp error; detect
      // up front and route the user to the scaffold flow with a clear message.
      try {
        const [hasCompose, shared] = await Promise.all([
          api.hasComposeFile(app.activeProject.path),
          api.sharedFileStatus(app.activeProject.path),
        ]);
        const missing: { key: typeof state.scaffoldInitialTab; label: string }[] = [];
        if (!hasCompose)       missing.push({ key: "compose",       label: "docker-compose.prod.yml" });
        if (!shared.compose)   missing.push({ key: "sharedCompose", label: "shared/docker-compose.yml" });
        if (!shared.traefik)   missing.push({ key: "sharedTraefik", label: "shared/traefik/traefik.yml" });
        if (!shared.init_db)   missing.push({ key: "sharedInitDb",  label: "shared/init-databases.sh" });

        if (missing.length > 0) {
          const list = missing.map(m => `• ${m.label}`).join("\n");
          const open = await confirmDialog.ask({
            title: `${missing.length} deployment file${missing.length === 1 ? "" : "s"} missing`,
            message: `The CLI needs these before it can scp them to the server:\n\n${list}\n\nBishop can scaffold them with sensible defaults — review and edit before saving.`,
            confirmLabel: "Open scaffold",
            cancelLabel: "Cancel deploy",
          });
          if (!open) return;
          state.scaffoldInitialTab = missing[0].key;
          state.showScaffold = true;
          return;
        }

        // Known-stale-template migrations. Older scaffolds of
        // docker-compose.prod.yml used `env_file: ../../.env` — docker compose
        // runs from the app dir on the server, so that path resolves to
        // /opt/.env (parent of the app dir) and the deploy fails. Offer a
        // targeted one-line fix that preserves all other edits.
        const migrations = await detectComposeMigrations(app.activeProject.path);
        if (migrations.length > 0) {
          const fixList = migrations.map(m => `• ${m.summary}`).join("\n");
          const ok = await confirmDialog.ask({
            title: "Update docker-compose.prod.yml before deploying?",
            message: `Found known stale patterns in .deploy/infra/docker-compose.prod.yml that will fail on the server:\n\n${fixList}\n\nBishop can patch them in place. Every other line in your compose file stays untouched.`,
            confirmLabel: "Patch and deploy",
            cancelLabel: "Cancel",
          });
          if (!ok) return;
          try {
            await applyComposeMigrations(app.activeProject.path, migrations);
            toast.success("Patched docker-compose.prod.yml");
          } catch (e) {
            toast.error("Patch failed", String(e));
            return;
          }
        }
      } catch { /* non-fatal — fall through and let the CLI's own error surface */ }

      const isProd = env.name === "production" || env.name === "prod" || env.name === "main";
      const ok = await confirmDialog.ask({
        title: `Deploy to ${env.name}?`,
        message: isProd
          ? `This runs ./deploy ${env.name} against ${env.ssh_host} — a production environment. Proceed?`
          : `This runs ./deploy ${env.name} against ${env.ssh_host}. The current app container will be replaced.`,
        confirmLabel: "Deploy",
        destructive: isProd,
      });
      if (!ok) return;
      try { await tasks.startDeploy(app.activeProject.path, env.name); }
      catch (e) { toast.error("Failed to start deploy", String(e)); }
    },

    openEnv() { if (app.activeProject && app.activeEnv) state.showEnv = true; },
    openHistory() { if (app.activeProject && app.activeEnv) state.showHistory = true; },

    openTerminal() {
      // Dynamic import to avoid the circular stores ↔ terminals dependency.
      import("./terminals.svelte").then(({ terminals }) => {
        if (app.activeProject && app.activeEnv) {
          const env = app.activeEnv;
          terminals.addOrFocusTab(
            { user: env.ssh_user, host: env.ssh_host, initial_cwd: env.app_dir },
            `${app.activeProject!.name} · ${env.name}`,
            `${env.ssh_user}@${env.ssh_host}`,
          );
        } else {
          terminals.show();
        }
      });
    },

    openSettings() { state.showSettings = true; },
    openInit() { state.showInit = true; },

    async runCheck() {
      if (!app.activeProject || !app.activeEnv) return;
      try { await tasks.startHealthCheck(app.activeProject.path, app.activeEnv.name); }
      catch (e) { toast.error("Failed to start check", String(e)); }
    },

    async runStep(sub: "setup-server" | "setup-app") {
      if (!app.activeProject || !app.activeEnv) return;
      const env = app.activeEnv;
      const ok = await confirmDialog.ask({
        title: `Run ${sub} on ${env.name}?`,
        message: sub === "setup-server"
          ? `Installs Docker, Traefik, shared Postgres and Redis on ${env.ssh_host}. Safe to re-run — takes ~3–5 minutes.`
          : `Bootstraps this app's config and .env on ${env.ssh_host}. Safe to re-run.`,
        confirmLabel: "Run",
      });
      if (!ok) return;
      try {
        await tasks.startCliStep(app.activeProject.path, sub, env.name, {
          title: sub === "setup-server" ? `Set up server — ${env.name}` : `Set up app — ${env.name}`,
          description: sub === "setup-server"
            ? `Installs Docker, Traefik, shared Postgres/Redis on ${env.ssh_host}.`
            : `Bootstraps this app's config and .env on ${env.ssh_host}.`,
        });
      } catch (e) { toast.error(`Failed to start ${sub}`, String(e)); }
    },

    async restart() {
      const proj = app.activeProject; const env = app.activeEnv;
      if (!proj || !env) return;
      const ok = await confirmDialog.ask({
        title: `Restart app on ${env.name}?`,
        message: `This stops and starts the "app" container on ${env.ssh_host}. In-flight requests may be dropped.`,
        confirmLabel: "Restart",
        destructive: true,
      });
      if (!ok) return;
      state.restarting = true;
      try {
        await api.restartService(proj.path, env.name, "app");
        toast.success(`Restarted app on ${env.name}`);
      } catch (e) {
        toast.error("Restart failed", String(e));
      } finally {
        state.restarting = false;
      }
    },
  };

  return self;
}

export const dashActions = createDashActions();
export type DashActionsStore = ReturnType<typeof createDashActions>;

/// -------------------------------------------------------------------------
/// Compose file migrations — targeted fixes for known-bad patterns left over
/// from older template versions. Each migration names itself for the confirm
/// dialog and carries an idempotent transformer.
/// -------------------------------------------------------------------------

interface ComposeMigration {
  id: string;
  summary: string;
  pattern: RegExp;
  replacement: string;
}

const COMPOSE_MIGRATIONS: ComposeMigration[] = [
  {
    id: "env-file-parent-parent",
    summary: "Rewrite env_file: ../../.env → .env (path was wrong for the server layout).",
    pattern: /^(\s*-?\s*)(\.\.\/\.\.\/\.env)\s*$/gm,
    replacement: "$1.env",
  },
  {
    id: "ghcr-triple-slash",
    summary: "Fix broken GHCR image reference (ghcr.io///github.com/... → ghcr.io/...).",
    // Produced by an earlier git-URL parser bug on HTTPS remotes.
    pattern: /ghcr\.io\/+(?:github|gitlab|bitbucket)\.(?:com|org)\//g,
    replacement: "ghcr.io/",
  },
  {
    id: "network-name-traefik",
    summary: "Rename app network 'traefik' → 'proxy' (matches the shared stack + CLI network creation).",
    // Only rewrite within the networks list / network declarations, not any
    // text that happens to contain 'traefik'.
    pattern: /(^\s*-\s*)traefik(\s*$)/gm,
    replacement: "$1proxy$2",
  },
  {
    id: "network-name-traefik-decl",
    summary: "Rename top-level network declaration 'traefik:' → 'proxy:'.",
    pattern: /(^networks:\s*\n(?:.*\n)*?\s+)traefik:/gm,
    replacement: "$1proxy:",
  },
  {
    id: "network-name-shared",
    summary: "Rename app network 'shared' → 'shared-db' (matches the shared stack).",
    pattern: /(^\s*-\s*)shared(\s*$)/gm,
    replacement: "$1shared-db$2",
  },
  {
    id: "network-name-shared-decl",
    summary: "Rename top-level network declaration 'shared:' → 'shared-db:'.",
    pattern: /(^networks:\s*\n(?:.*\n)*?\s+)shared:(?!-db)/gm,
    replacement: "$1shared-db:",
  },
];

async function detectComposeMigrations(projectPath: string): Promise<ComposeMigration[]> {
  const content = await api.readComposeFile(projectPath);
  if (!content) return [];
  return COMPOSE_MIGRATIONS.filter(m => {
    // `test` with a stateful /g RegExp is fine here because we create fresh
    // copies when actually applying — but reset lastIndex to be safe.
    m.pattern.lastIndex = 0;
    return m.pattern.test(content);
  });
}

async function applyComposeMigrations(projectPath: string, migrations: ComposeMigration[]): Promise<void> {
  let content = await api.readComposeFile(projectPath);
  if (!content) return;
  for (const m of migrations) {
    m.pattern.lastIndex = 0;
    content = content.replace(m.pattern, m.replacement);
  }
  await api.writeComposeFile(projectPath, content);
}

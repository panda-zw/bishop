<script lang="ts">
  import { app } from "../stores.svelte";
  import { api } from "../api";
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";
  import Input from "./ui/Input.svelte";
  import Field from "./ui/Field.svelte";
  import { dashActions } from "../stores.svelte";
  import { tasks } from "../tasks.svelte";
  import { toast } from "../toast.svelte";

  interface Props { onClose: () => void }
  let { onClose }: Props = $props();

  let step = $state(0);

  let projectPath = $state("");
  let envName = $state("staging");
  let sshUser = $state("root");
  let sshHost = $state("");
  let appName = $state("");
  let domain = $state("");
  let appPort = $state<number>(3000);
  let healthString = $state("ok");
  let extraContainers = $state("");
  let dataDirs = $state("");
  let autoSecrets = $state("JWT_SECRET");
  let domainTemplates = $state("");

  let busy = $state(false);
  let error = $state<string | null>(null);
  let created = $state(false);

  type StepState = "pending" | "running" | "done" | "failed";
  let serverTaskId = $state<string | null>(null);
  let appTaskId = $state<string | null>(null);

  /// Derive step state from the task store so we stay in sync even if the user
  /// minimized/cancelled the task from the background bar.
  const serverState = $derived.by<StepState>(() => stateFromTask(serverTaskId));
  const appState = $derived.by<StepState>(() => stateFromTask(appTaskId));

  function stateFromTask(id: string | null): StepState {
    if (!id) return "pending";
    const t = tasks.byId(id);
    if (!t) return "pending";
    if (t.status === "running") return "running";
    if (t.status === "success") return "done";
    return "failed";
  }

  async function runStep(sub: "setup-server" | "setup-app") {
    try {
      const id = await tasks.startCliStep(projectPath, sub, envName, {
        title: sub === "setup-server"
          ? `Set up server — ${envName}`
          : `Set up app — ${envName}`,
        description: sub === "setup-server"
          ? `Installs Docker, Traefik, shared Postgres/Redis on ${sshUser}@${sshHost}.`
          : `Bootstraps this app's config + .env on ${sshUser}@${sshHost}.`,
      });
      if (sub === "setup-server") serverTaskId = id; else appTaskId = id;
    } catch (e) {
      toast.error(`Failed to start ${sub}`, String(e));
    }
  }

  async function pickDir() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ directory: true, multiple: false });
    if (typeof p === "string") {
      projectPath = p;
      if (!appName) {
        const base = p.split("/").pop()!.toLowerCase().replace(/_/g, "-");
        appName = envName === "production" ? base : `${base}-${envName}`;
      }
    }
  }

  async function submit() {
    busy = true; error = null;
    try {
      const proj = await api.initProject({
        project_path: projectPath,
        env_name: envName.trim(),
        ssh_user: sshUser.trim(),
        ssh_host: sshHost.trim(),
        app_name: appName.trim(),
        domain: domain.trim() || null,
        app_port: appPort,
        health_string: healthString,
        extra_containers: extraContainers,
        data_dirs: dataDirs,
        auto_secrets: autoSecrets,
        domain_templates: domainTemplates,
      });

      // Register project in Bishop's stored list (writes settings.json).
      try {
        await api.addProject(projectPath);
      } catch {}

      // Install a project-local copy of the deploy script so subsequent commands
      // find it without falling back to the bundled one. Non-blocking.
      try {
        const hasLocal = await api.hasLocalDeployScript(projectPath);
        if (!hasLocal) await api.installDeployScript(projectPath);
      } catch (e) {
        // Non-fatal — the bundled copy still works.
        console.warn("install_deploy_script:", e);
      }

      // Refresh list + select the new one.
      app.projects = await api.listProjects();
      const picked = app.projects.find(p => p.path === proj.path);
      if (picked) app.selectProject(picked);
      api.refreshTray().catch(() => {});
      created = true;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function field(label: string, value: string) { return { label, value }; }
</script>

<Modal title="Setup new project" wide {onClose}>
  <div class="p-5 space-y-5">
    {#if created}
      {@const canDeploy = serverState === "done" && appState === "done"}
      <div class="space-y-4">
        <div class="text-sm text-ok flex items-center gap-2">
          <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
          Project created. Bishop is tracking it.
        </div>
        <div class="text-sm font-medium">Next steps</div>
        <div class="text-xs text-muted -mt-2">
          Run these against <span class="font-mono text-foreground">{sshUser}@{sshHost}</span>.
          Each step streams its output live — you can cancel at any time.
        </div>

        <ol class="space-y-2">
          <!-- Step 1: setup-server -->
          <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md bg-card">
            <span class="w-6 h-6 shrink-0 rounded-full border flex items-center justify-center text-[11px] font-semibold
              {serverState === 'done' ? 'bg-ok text-background border-ok' :
               serverState === 'running' ? 'border-warn text-warn' :
               serverState === 'failed' ? 'border-err text-err' :
               'border-border text-muted'}">
              {serverState === 'done' ? '✓' : serverState === 'failed' ? '!' : '1'}
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium">Set up the server</div>
              <div class="text-xs text-muted mt-0.5">Installs Docker, Traefik, Postgres, and Redis on the remote host. Takes ~3–5 minutes.</div>
              <div class="text-[11px] text-muted font-mono mt-1.5">./deploy setup-server {envName}</div>
            </div>
            <div class="flex items-center gap-2">
              {#if serverState === 'running' && serverTaskId}
                <button class="text-xs text-muted hover:text-foreground underline" onclick={() => tasks.show(serverTaskId!)}>view</button>
              {/if}
              <Button
                size="sm"
                variant={serverState === 'done' ? 'outline' : 'default'}
                disabled={serverState === 'running'}
                onclick={() => runStep('setup-server')}
              >
                {serverState === 'done' ? 'Re-run' : serverState === 'failed' ? 'Retry' : serverState === 'running' ? 'Running…' : 'Run'}
              </Button>
            </div>
          </li>

          <!-- Step 2: setup-app -->
          <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md bg-card">
            <span class="w-6 h-6 shrink-0 rounded-full border flex items-center justify-center text-[11px] font-semibold
              {appState === 'done' ? 'bg-ok text-background border-ok' :
               appState === 'running' ? 'border-warn text-warn' :
               appState === 'failed' ? 'border-err text-err' :
               'border-border text-muted'}">
              {appState === 'done' ? '✓' : appState === 'failed' ? '!' : '2'}
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium">Set up the app</div>
              <div class="text-xs text-muted mt-0.5">Bootstraps this app's config and <span class="font-mono">.env</span> on the server.</div>
              <div class="text-[11px] text-muted font-mono mt-1.5">./deploy setup-app {envName}</div>
            </div>
            <div class="flex items-center gap-2">
              {#if appState === 'running' && appTaskId}
                <button class="text-xs text-muted hover:text-foreground underline" onclick={() => tasks.show(appTaskId!)}>view</button>
              {/if}
              <Button
                size="sm"
                variant={appState === 'done' ? 'outline' : 'default'}
                disabled={appState === 'running' || serverState !== 'done'}
                onclick={() => runStep('setup-app')}
              >
                {appState === 'done' ? 'Re-run' : appState === 'failed' ? 'Retry' : appState === 'running' ? 'Running…' : 'Run'}
              </Button>
            </div>
          </li>

          <!-- Step 3: first deploy -->
          <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md bg-card">
            <span class="w-6 h-6 shrink-0 rounded-full border border-border text-muted flex items-center justify-center text-[11px] font-semibold">3</span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium">First deploy</div>
              <div class="text-xs text-muted mt-0.5">Builds, pushes, and starts the app. You can do this any time from the dashboard.</div>
            </div>
            <Button
              size="sm"
              variant={canDeploy ? 'default' : 'outline'}
              disabled={!canDeploy}
              onclick={() => { onClose(); dashActions.deploy(); }}
            >
              Deploy
            </Button>
          </li>
        </ol>

        <div class="flex justify-end pt-1">
          <Button size="sm" variant="outline" onclick={onClose}>Close wizard</Button>
        </div>
      </div>
    {:else}
      <div class="space-y-4">
        <div class="text-xs text-muted">
          Writes <span class="font-mono text-foreground">.deploy/config</span>
          and <span class="font-mono text-foreground">.deploy/remotes/{envName || "&lt;env&gt;"}</span>
          in the selected folder.
        </div>

        <Field label="Project folder" hint="The folder on your computer containing the app's code.">
          <div class="flex gap-2">
            <Input class="font-mono" placeholder="/path/to/project" bind:value={projectPath} />
            <Button size="md" variant="outline" onclick={pickDir}>Choose…</Button>
          </div>
        </Field>

        <div class="grid grid-cols-2 gap-4">
          <Field label="Environment name" hint="A short name for this deployment, e.g. staging, production.">
            <Input placeholder="staging" bind:value={envName} />
          </Field>
          <Field label="App name on server" hint="The unique name this app uses for its containers and folders on the server.">
            <Input placeholder="my-app-staging" bind:value={appName} />
          </Field>
          <Field label="SSH user" hint="The login user on the server. Usually root or a deploy user.">
            <Input bind:value={sshUser} />
          </Field>
          <Field label="Server address" hint="The server's IP address or domain name.">
            <Input placeholder="203.0.113.10" bind:value={sshHost} />
          </Field>
          <div class="col-span-2">
            <Field
              label="Public domain"
              optional
              hint="The domain your app will be reachable at. Make sure DNS is already pointing at the server. Leave blank if you're using IP-based access."
            >
              <Input placeholder="staging.example.com" bind:value={domain} />
            </Field>
          </div>
        </div>

        <details class="text-sm">
          <summary class="cursor-pointer text-xs font-medium text-muted hover:text-foreground select-none rounded-sm px-1 py-0.5 -mx-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50">
            Advanced config <span class="text-muted/70 font-normal">— only change these if you know you need to</span>
          </summary>
          <div class="grid grid-cols-2 gap-4 mt-3 pt-3 border-t border-border">
            <Field label="App port" hint="The port your app listens on inside its container. Most web apps use 3000 or 8000.">
              <Input type="number" bind:value={appPort} />
            </Field>
            <Field label="Health check text" hint="A word the app returns when it's healthy. Deploy waits for this before declaring success.">
              <Input bind:value={healthString} />
            </Field>
            <div class="col-span-2">
              <Field
                label="Extra containers"
                optional
                hint="Other services your app needs besides the main one — separated by spaces. For example: redis-worker minio"
              >
                <Input class="font-mono" placeholder="redis-worker minio" bind:value={extraContainers} />
              </Field>
            </div>
            <div class="col-span-2">
              <Field
                label="Folders to keep between deploys"
                optional
                hint="Folders that hold data you don't want wiped on each deploy (uploads, caches, etc). Space-separated."
              >
                <Input class="font-mono" placeholder="uploads minio" bind:value={dataDirs} />
              </Field>
            </div>
            <div class="col-span-2">
              <Field
                label="Secrets to auto-generate"
                optional
                hint="Names of env vars the server should fill with random values the first time you set up (like JWT_SECRET). Space-separated."
              >
                <Input class="font-mono" bind:value={autoSecrets} />
              </Field>
            </div>
            <div class="col-span-2">
              <Field
                label="Domain-based env vars"
                optional
                hint={"Env vars whose value depends on your domain. Use ${DOMAIN} as the placeholder. Space-separated KEY=template pairs."}
              >
                <Input class="font-mono" placeholder="CALLBACK_URL=https://${'${DOMAIN}'}/cb" bind:value={domainTemplates} />
              </Field>
            </div>
          </div>
        </details>

        {#if error}
          <div class="text-xs text-err border border-err/30 bg-err/10 rounded-md px-3 py-2 font-mono">{error}</div>
        {/if}
      </div>
    {/if}
  </div>
  {#snippet footer()}
    {#if !created}
      <Button size="sm" variant="outline" onclick={onClose} disabled={busy}>Cancel</Button>
      <Button size="sm"
        disabled={busy || !projectPath || !envName || !sshHost || !appName}
        onclick={submit}
      >{busy ? "Creating…" : "Create"}</Button>
    {/if}
  {/snippet}
</Modal>


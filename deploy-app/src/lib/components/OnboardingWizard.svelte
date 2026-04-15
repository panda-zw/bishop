<script lang="ts">
  /// First-run onboarding — five-step modal that takes a brand-new user from
  /// a cold install to a deployed app in ≤15 minutes. Auto-opens on launch
  /// when the user has no projects.
  ///
  /// Each step scopes itself tightly:
  ///   1. Welcome    — context + expectations
  ///   2. Project    — pick a folder (sample-app path coming in #13)
  ///   3. Server     — SSH creds with pre-flight test (catches bad keys cheaply)
  ///   4. Configure  — app identity + optional domain
  ///   5. Provision  — create project, run setup-server + setup-app, first deploy
  ///
  /// Failures in steps 3 and 5 route through <ErrorBanner> so users see
  /// catalog entries ("Permission denied — run `ssh-add`") instead of raw
  /// stderr traces.

  import { app, dashActions } from "../stores.svelte";
  import { api, type SshTestResult } from "../api";
  import { tasks } from "../tasks.svelte";
  import { toast } from "../toast.svelte";
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";
  import Input from "./ui/Input.svelte";
  import Field from "./ui/Field.svelte";
  import ErrorBanner from "./ErrorBanner.svelte";
  import type { BishopError } from "../types";

  interface Props { onClose: () => void }
  let { onClose }: Props = $props();

  // ---------- step state ----------
  type Step = 1 | 2 | 3 | 4 | 5;
  let step = $state<Step>(1);

  // Step 2 — project. Users either pick an existing folder or clone our starter.
  let projectPath = $state("");
  let cloning = $state(false);
  let cloneError = $state<string | null>(null);
  let usedStarter = $state(false);

  // Step 3 — server + SSH test
  let sshUser = $state("root");
  let sshHost = $state("");
  let testing = $state(false);
  let testResult = $state<SshTestResult | null>(null);
  const sshOk = $derived(testResult?.ok === true);

  // Step 4 — configure
  let envName = $state("staging");
  let appName = $state("");
  let domain = $state("");
  let appPort = $state<number>(3000);

  // Step 5 — provision
  let creating = $state(false);
  let createError = $state<BishopError | null>(null);
  let projectCreated = $state(false);
  let serverTaskId = $state<string | null>(null);
  let appTaskId = $state<string | null>(null);

  type TaskState = "pending" | "running" | "done" | "failed";
  function stateFromTask(id: string | null): TaskState {
    if (!id) return "pending";
    const t = tasks.byId(id);
    if (!t) return "pending";
    if (t.status === "running") return "running";
    if (t.status === "success") return "done";
    return "failed";
  }
  const serverState = $derived.by(() => stateFromTask(serverTaskId));
  const appState = $derived.by(() => stateFromTask(appTaskId));

  // ---------- derivations ----------
  /// Auto-fill appName from the project folder + env name on the way into step 4.
  function deriveAppName() {
    if (appName) return;
    const base = projectPath.split("/").pop()?.toLowerCase().replace(/_/g, "-") ?? "app";
    appName = envName === "production" ? base : `${base}-${envName}`;
  }

  // ---------- step 2 ----------
  async function pickDir() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ directory: true, multiple: false });
    if (typeof p === "string") { projectPath = p; usedStarter = false; }
  }

  /// Clone the Bishop starter into a user-picked parent folder. Defaults the
  /// folder name so users who just want to try it out don't have to think.
  async function useStarter() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const parent = await open({
      directory: true,
      multiple: false,
      title: "Where should Bishop put the starter project?",
    });
    if (typeof parent !== "string") return;

    cloning = true;
    cloneError = null;
    try {
      projectPath = await api.cloneSample(parent, "bishop-starter-nextjs");
      usedStarter = true;
      // Opinionated defaults for the starter so the rest of the wizard
      // is "press Continue four times" when trying it.
      if (!appName) appName = "bishop-starter";
      appPort = 3000;
    } catch (e) {
      cloneError = String(e);
    } finally {
      cloning = false;
    }
  }

  // ---------- step 3 ----------
  async function runSshTest() {
    if (!sshUser.trim() || !sshHost.trim()) return;
    testing = true;
    testResult = null;
    try {
      testResult = await api.testSsh(sshUser.trim(), sshHost.trim());
    } catch (e) {
      testResult = { ok: false, message: null, error: null, raw: String(e) };
    } finally {
      testing = false;
    }
  }

  // ---------- step 5 ----------
  async function createProject() {
    creating = true;
    createError = null;
    try {
      const proj = await api.initProject({
        project_path: projectPath,
        env_name: envName.trim(),
        ssh_user: sshUser.trim(),
        ssh_host: sshHost.trim(),
        app_name: appName.trim(),
        domain: domain.trim() || null,
        app_port: appPort,
        health_string: "ok",
        extra_containers: "",
        data_dirs: "",
        auto_secrets: "JWT_SECRET",
        domain_templates: "",
      });
      try { await api.addProject(projectPath); } catch {}
      try {
        const hasLocal = await api.hasLocalDeployScript(projectPath);
        if (!hasLocal) await api.installDeployScript(projectPath);
      } catch (e) { console.warn("install_deploy_script:", e); }

      app.projects = await api.listProjects();
      const picked = app.projects.find(p => p.path === proj.path);
      if (picked) app.selectProject(picked);
      api.refreshTray().catch(() => {});
      projectCreated = true;
    } catch (e) {
      createError = {
        code: "INIT_PROJECT_FAILED",
        message: "Failed to create project.",
        hint: "Check that the folder is writable and the form values are valid.",
        raw: String(e),
      };
    } finally {
      creating = false;
    }
  }

  async function runStep(sub: "setup-server" | "setup-app") {
    try {
      const id = await tasks.startCliStep(projectPath, sub, envName, {
        title: sub === "setup-server" ? `Set up server — ${envName}` : `Set up app — ${envName}`,
        description: sub === "setup-server"
          ? `Installs Docker, Traefik, Postgres, Redis on ${sshUser}@${sshHost}.`
          : `Bootstraps the app's config + .env on ${sshUser}@${sshHost}.`,
      });
      if (sub === "setup-server") serverTaskId = id; else appTaskId = id;
    } catch (e) {
      toast.error(`Failed to start ${sub}`, String(e));
    }
  }

  function deployNow() {
    onClose();
    dashActions.deploy();
  }

  // ---------- step guards ----------
  const canAdvance = $derived.by(() => {
    switch (step) {
      case 1: return true;
      case 2: return projectPath.trim().length > 0;
      case 3: return sshOk;
      case 4: return envName.trim() && appName.trim();
      case 5: return true;
      default: return false;
    }
  });

  function next() {
    if (!canAdvance) return;
    if (step === 3) deriveAppName();
    if (step < 5) step = (step + 1) as Step;
  }

  function prev() {
    if (step > 1) step = (step - 1) as Step;
  }

  // ---------- step headings ----------
  const titles: Record<Step, string> = {
    1: "Welcome to Bishop",
    2: "Pick your project",
    3: "Connect to a server",
    4: "Name your app",
    5: "Provision and deploy",
  };
</script>

<Modal title={titles[step]} wide onClose={onClose}>
  <div class="px-5 py-4">
    <!-- Stepper -->
    <div class="flex items-center gap-1 mb-5" role="navigation" aria-label="Onboarding steps">
      {#each [1, 2, 3, 4, 5] as i}
        {@const done = i < step}
        {@const current = i === step}
        <div class="flex items-center gap-1 flex-1 min-w-0">
          <span
            class="shrink-0 w-6 h-6 rounded-full border flex items-center justify-center text-[11px] font-semibold transition-colors
              {done ? 'bg-ok text-background border-ok' : current ? 'border-primary text-primary bg-primary/10' : 'border-border text-muted'}"
            aria-current={current ? "step" : undefined}
          >
            {done ? "✓" : i}
          </span>
          {#if i < 5}
            <div class="flex-1 h-px {done ? 'bg-ok' : 'bg-border'} transition-colors"></div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Step 1: Welcome -->
    {#if step === 1}
      <div class="space-y-4 text-sm leading-relaxed">
        <p>
          Bishop deploys your apps to servers you own. You'll point it at a Linux VPS,
          it'll install Docker, Traefik (with auto-TLS), shared Postgres and Redis, and
          then it'll ship your app onto that stack in about 15 minutes.
        </p>
        <div class="rounded-md border border-border bg-secondary/40 px-4 py-3 space-y-2">
          <div class="text-xs font-semibold uppercase tracking-wider text-muted">Before you start</div>
          <ul class="text-xs text-muted space-y-1.5 list-disc list-inside">
            <li>A fresh Linux server (Ubuntu 22.04+ recommended) that you can SSH into as root.</li>
            <li>Your app's source code checked out locally.</li>
            <li>(Optional) a domain with its A record pointing at the server — for auto-TLS.</li>
          </ul>
        </div>
        <p class="text-xs text-muted">
          No server yet? Cheapest path: a $5/mo <a class="underline hover:text-foreground" href="https://hetzner.com/cloud" target="_blank" rel="noreferrer">Hetzner Cloud</a>
          CX22, or a DigitalOcean droplet. Come back when you have SSH access.
        </p>
      </div>
    {/if}

    <!-- Step 2: Project -->
    {#if step === 2}
      <div class="space-y-4 text-sm">
        <p class="text-muted">
          Point Bishop at a folder with your app's source — or clone our Next.js starter
          if you just want to try the flow end-to-end.
        </p>

        <Field label="Project folder">
          <div class="flex gap-2">
            <Input class="font-mono" placeholder="/path/to/your/project" bind:value={projectPath} />
            <Button size="md" variant="outline" onclick={pickDir} disabled={cloning}>Choose…</Button>
          </div>
        </Field>

        {#if usedStarter && projectPath}
          <div class="text-xs text-ok flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
            Starter cloned into <span class="font-mono text-foreground">{projectPath}</span>
          </div>
        {/if}

        <div class="rounded-md border border-border border-dashed px-4 py-3 flex items-start gap-3">
          <div class="flex-1">
            <div class="text-xs font-semibold uppercase tracking-wider text-muted mb-1">Don't have a project yet?</div>
            <div class="text-xs text-muted">
              Clone the Next.js starter — a tiny app with a working Dockerfile and a
              <span class="font-mono text-foreground">/api/health</span> endpoint pre-wired for Bishop.
            </div>
          </div>
          <Button size="sm" variant="outline" onclick={useStarter} disabled={cloning}>
            {cloning ? "Cloning…" : "Use the starter"}
          </Button>
        </div>

        {#if cloneError}
          <div class="rounded-md border border-err/40 bg-err/5 px-4 py-3 text-xs">
            <div class="font-medium text-foreground mb-1">Couldn't clone the starter</div>
            <pre class="font-mono text-muted whitespace-pre-wrap">{cloneError}</pre>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Step 3: Server -->
    {#if step === 3}
      <div class="space-y-4 text-sm">
        <p class="text-muted">
          We'll run a quick SSH probe to confirm your key works before spending time on the
          real setup.
        </p>
        <div class="grid grid-cols-2 gap-4">
          <Field label="SSH user" hint="Usually root on a fresh VPS.">
            <Input bind:value={sshUser} />
          </Field>
          <Field label="Server address" hint="IP or hostname.">
            <Input placeholder="203.0.113.10" bind:value={sshHost} />
          </Field>
        </div>
        <div class="flex items-center gap-3">
          <Button
            size="sm"
            variant="outline"
            onclick={runSshTest}
            disabled={!sshUser.trim() || !sshHost.trim() || testing}
          >
            {testing ? "Testing…" : "Test connection"}
          </Button>
          {#if testResult && testResult.ok}
            <span class="text-xs text-ok flex items-center gap-1.5">
              <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
              Connected — {testResult.message}
            </span>
          {/if}
        </div>
        {#if testResult && !testResult.ok}
          {#if testResult.error}
            <ErrorBanner error={testResult.error} onAction={() => runSshTest()} />
          {:else}
            <div class="rounded-md border border-err/40 bg-err/5 px-4 py-3 text-xs text-foreground">
              <div class="font-medium mb-1">Connection failed</div>
              <pre class="font-mono text-muted whitespace-pre-wrap">{testResult.raw}</pre>
            </div>
          {/if}
        {/if}
      </div>
    {/if}

    <!-- Step 4: Configure -->
    {#if step === 4}
      <div class="space-y-4 text-sm">
        <p class="text-muted">
          Name this deployment. You'll be able to add more environments (production, preview, etc.) later.
        </p>
        <div class="grid grid-cols-2 gap-4">
          <Field label="Environment" hint="Short name used in server paths. Common: staging, production.">
            <Input bind:value={envName} />
          </Field>
          <Field label="App name on server" hint="Used for container names + /opt folder.">
            <Input bind:value={appName} />
          </Field>
          <Field label="App port" hint="Port your app listens on inside its container.">
            <Input type="number" bind:value={appPort} />
          </Field>
          <Field label="Public domain" optional hint="Leave blank to deploy without TLS.">
            <Input placeholder="staging.example.com" bind:value={domain} />
          </Field>
        </div>
      </div>
    {/if}

    <!-- Step 5: Provision -->
    {#if step === 5}
      <div class="space-y-4 text-sm">
        {#if !projectCreated}
          <p class="text-muted">
            Ready to go. Bishop will create <span class="font-mono text-foreground">.deploy/</span>
            in your project folder, then install infrastructure and bootstrap the app on
            <span class="font-mono text-foreground">{sshUser}@{sshHost}</span>.
          </p>
          {#if createError}
            <ErrorBanner error={createError} />
          {/if}
          <Button size="md" disabled={creating} onclick={createProject}>
            {creating ? "Creating…" : "Create project and continue"}
          </Button>
        {:else}
          <ol class="space-y-2">
            <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md">
              <span class="w-6 h-6 shrink-0 rounded-full border flex items-center justify-center text-[11px] font-semibold
                {serverState === 'done' ? 'bg-ok text-background border-ok' :
                 serverState === 'running' ? 'border-warn text-warn' :
                 serverState === 'failed' ? 'border-err text-err' : 'border-border text-muted'}">
                {serverState === 'done' ? '✓' : serverState === 'failed' ? '!' : '1'}
              </span>
              <div class="flex-1">
                <div class="font-medium">Set up the server</div>
                <div class="text-xs text-muted mt-0.5">Docker + Traefik + Postgres + Redis. ~3–5 min.</div>
              </div>
              <div class="flex items-center gap-2">
                {#if serverState === 'running' && serverTaskId}
                  <button class="text-xs text-muted hover:text-foreground underline" onclick={() => tasks.show(serverTaskId!)}>view</button>
                {/if}
                <Button size="sm" variant={serverState === 'done' ? 'outline' : 'default'}
                        disabled={serverState === 'running'} onclick={() => runStep('setup-server')}>
                  {serverState === 'done' ? 'Re-run' : serverState === 'failed' ? 'Retry' : serverState === 'running' ? 'Running…' : 'Run'}
                </Button>
              </div>
            </li>
            <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md">
              <span class="w-6 h-6 shrink-0 rounded-full border flex items-center justify-center text-[11px] font-semibold
                {appState === 'done' ? 'bg-ok text-background border-ok' :
                 appState === 'running' ? 'border-warn text-warn' :
                 appState === 'failed' ? 'border-err text-err' : 'border-border text-muted'}">
                {appState === 'done' ? '✓' : appState === 'failed' ? '!' : '2'}
              </span>
              <div class="flex-1">
                <div class="font-medium">Set up this app</div>
                <div class="text-xs text-muted mt-0.5">Uploads .env + app-specific config.</div>
              </div>
              <div class="flex items-center gap-2">
                {#if appState === 'running' && appTaskId}
                  <button class="text-xs text-muted hover:text-foreground underline" onclick={() => tasks.show(appTaskId!)}>view</button>
                {/if}
                <Button size="sm" variant={appState === 'done' ? 'outline' : 'default'}
                        disabled={appState === 'running' || serverState !== 'done'} onclick={() => runStep('setup-app')}>
                  {appState === 'done' ? 'Re-run' : appState === 'failed' ? 'Retry' : appState === 'running' ? 'Running…' : 'Run'}
                </Button>
              </div>
            </li>
            <li class="flex items-start gap-3 px-3 py-3 border border-border rounded-md">
              <span class="w-6 h-6 shrink-0 rounded-full border border-border text-muted flex items-center justify-center text-[11px] font-semibold">3</span>
              <div class="flex-1">
                <div class="font-medium">First deploy</div>
                <div class="text-xs text-muted mt-0.5">Builds + ships your app. Do this any time.</div>
              </div>
              <Button size="sm" disabled={serverState !== 'done' || appState !== 'done'} onclick={deployNow}>
                Deploy
              </Button>
            </li>
          </ol>
        {/if}
      </div>
    {/if}
  </div>

  {#snippet footer()}
    {#if step === 1}
      <Button size="sm" variant="ghost" onclick={onClose}>Skip for now</Button>
      <Button size="sm" onclick={next}>Let's go</Button>
    {:else if step < 5}
      <Button size="sm" variant="ghost" onclick={prev}>Back</Button>
      <Button size="sm" onclick={next} disabled={!canAdvance}>Continue</Button>
    {:else}
      <Button size="sm" variant="ghost" onclick={prev} disabled={creating}>Back</Button>
      {#if !projectCreated}
        <Button size="sm" variant="outline" onclick={onClose}>Close</Button>
      {:else}
        <Button size="sm" variant="outline" onclick={onClose}>Close wizard</Button>
      {/if}
    {/if}
  {/snippet}
</Modal>

<script lang="ts">
  import { app, dashActions } from "../stores.svelte";
  import { api } from "../api";
  import { toast } from "../toast.svelte";
  import { hosts } from "../hosts.svelte";
  import { terminals } from "../terminals.svelte";
  import Button from "./ui/Button.svelte";
  import { onMount } from "svelte";

  let hostsExpanded = $state(true);
  let editingHostId = $state<string | null>(null);
  let showHostEditor = $state<"new" | "edit" | null>(null);

  onMount(() => { hosts.refresh(); });

  function openHost(target: import("../types").TerminalTarget, label: string, subtitle: string) {
    const tab = terminals.addOrFocusTab(target, label, subtitle);
    // Make the panel surface itself even if the user's previous action was "Hide".
    terminals.show();
    toast.info(`Opened ${tab.label}`, subtitle);
  }

  async function addProject() {
    const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
    const path = await openDialog({ directory: true, multiple: false });
    if (typeof path === "string") {
      try {
        const p = await api.addProject(path);
        app.projects = [...app.projects, p];
        app.selectProject(p);
        app.probeOne(p.path, app.activeEnv?.name ?? (p.remotes[0]?.name ?? ""));
        api.refreshTray().catch(() => {});
        toast.success(`Added ${p.name}`);
      } catch (e) {
        toast.error("Failed to add project", String(e));
      }
    }
  }

  // Shorten a path for display: keep the folder name and ~3 parent segments at most.
  function shortPath(p: string) {
    const home = "/Users/";
    if (p.startsWith(home)) {
      const rest = p.slice(home.length);
      const parts = rest.split("/");
      if (parts.length > 3) return "~/…/" + parts.slice(-2).join("/");
      return "~/" + parts.slice(1).join("/");
    }
    return p;
  }
</script>

{#if app.sidebarCollapsed}
  <!-- Icon-tray mode (~48px). Everything is a single-column, tooltip-rich rail. -->
  <aside class="w-12 shrink-0 bg-card border-r border-border flex flex-col">
    <div class="h-12 flex items-center justify-center border-b border-border">
      <button
        type="button"
        class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={() => app.setSidebarCollapsed(false)}
        title="Expand sidebar (⌘B)"
        aria-label="Expand sidebar"
      >
        <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 4l4 4-4 4"/>
        </svg>
      </button>
    </div>

    <nav class="flex-1 overflow-auto py-2 flex flex-col items-center gap-1">
      {#each app.projects as project (project.path)}
        {@const isActive = app.activeProject?.path === project.path}
        <button
          class="w-8 h-8 inline-flex items-center justify-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50
            {isActive ? 'bg-secondary text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}"
          onclick={() => app.selectProject(project)}
          title={project.name}
          aria-label={project.name}
        >
          <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.5 1.5h4.5A1.5 1.5 0 0 1 14 6v5.5A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5v-7Z"/>
          </svg>
        </button>
      {/each}

      <div class="w-6 h-px bg-border my-1"></div>

      <!-- Local shell -->
      <button
        class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={() => terminals.addOrFocusTab({ local: true }, "Local shell", "~")}
        title="Open local shell"
        aria-label="Open local shell"
      >
        <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" d="M2 3.5h12v9H2zM4.5 7l2 1.5-2 1.5M8 10.5h4"/>
        </svg>
      </button>

      <!-- Saved hosts (up to 6 icons; overflow collapses) -->
      {#each hosts.saved.slice(0, 6) as h (h.id)}
        <button
          class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={() => openHost({ user: h.user, host: h.host, port: h.port ?? null, identity: h.identity ?? null, proxy_jump: h.proxy_jump ?? null, initial_cwd: h.initial_cwd ?? null, use_tmux: h.use_tmux ?? false, tmux_session: h.tmux_session ?? h.label }, h.label, `${h.user}@${h.host}`)}
          title={`${h.label} — ${h.user}@${h.host}`}
          aria-label={h.label}
        >
          <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3 4h10v4H3zM3 10.5h10M5 4v-.5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1V4"/>
          </svg>
        </button>
      {/each}
    </nav>

    <div class="py-2 flex flex-col items-center gap-1 border-t border-border">
      <button
        class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={addProject}
        title="Add project"
        aria-label="Add project"
      >+</button>
      <button
        class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={() => dashActions.openInit()}
        title="Setup new project"
        aria-label="Setup new project"
      >
        <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path stroke-linecap="round" d="M8 3v10M3 8h10"/>
          <circle cx="8" cy="8" r="6"/>
        </svg>
      </button>
      <button
        class="w-8 h-8 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        onclick={() => dashActions.openSettings()}
        title="Settings"
        aria-label="Settings"
      >
        <svg viewBox="0 0 16 16" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <circle cx="8" cy="8" r="2.2" />
          <path stroke-linecap="round" d="M8 1.5v1.8M8 12.7v1.8M14.5 8h-1.8M3.3 8H1.5M12.6 3.4l-1.3 1.3M4.7 11.3l-1.3 1.3M12.6 12.6l-1.3-1.3M4.7 4.7L3.4 3.4" />
        </svg>
      </button>
    </div>
  </aside>
{:else}
<aside class="w-64 shrink-0 bg-card border-r border-border flex flex-col">
  <div class="px-4 h-12 flex items-center justify-between border-b border-border">
    <span class="text-[13px] font-semibold tracking-tight">Bishop</span>
    <button
      class="w-6 h-6 inline-flex items-center justify-center rounded-sm text-muted hover:text-foreground hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      onclick={() => app.setSidebarCollapsed(true)}
      title="Collapse sidebar (⌘B)"
      aria-label="Collapse sidebar"
    >
      <svg viewBox="0 0 16 16" class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
        <path stroke-linecap="round" stroke-linejoin="round" d="M10 4l-4 4 4 4"/>
      </svg>
    </button>
  </div>

  <div class="px-3 pt-4 pb-2">
    <div class="text-[10px] font-semibold uppercase tracking-widest text-muted">Projects</div>
  </div>

  <nav class="flex-1 overflow-auto px-2 pb-2">
    <!-- Projects -->
    {#each app.projects as project (project.path)}
      {@const isActiveProject = app.activeProject?.path === project.path}
      <div class="mb-1">
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded-md transition-colors text-left
            {isActiveProject ? 'bg-secondary' : 'hover:bg-secondary/60'}
            focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={() => app.selectProject(project)}
          aria-expanded={isActiveProject}
        >
          <svg
            viewBox="0 0 10 10"
            class="w-2.5 h-2.5 text-muted shrink-0 transition-transform {isActiveProject ? 'rotate-90' : ''}"
            fill="currentColor"
            aria-hidden="true"
          >
            <path d="M3 1.5l4 3.5-4 3.5V1.5z" />
          </svg>
          <svg viewBox="0 0 16 16" class="w-3.5 h-3.5 text-muted shrink-0" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.5 1.5h4.5A1.5 1.5 0 0 1 14 6v5.5A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5v-7Z"/>
          </svg>
          <div class="flex-1 min-w-0">
            <div class="text-[13px] font-semibold leading-tight truncate {isActiveProject ? 'text-foreground' : 'text-foreground/90'}">{project.name}</div>
            <div class="text-[10px] text-muted font-mono truncate leading-tight mt-0.5">{shortPath(project.path)}</div>
          </div>
        </button>

        {#if isActiveProject && project.remotes.length > 0}
          <ul class="mt-1 ml-[14px] border-l border-border/70 pl-2 space-y-px py-0.5">
            {#each project.remotes as env (env.name)}
              {@const status = app.statusFor(project.path, env.name)}
              {@const active = app.activeEnv?.name === env.name}
              {@const tip =
                status === 'up' ? 'running'
                : status === 'starting' ? 'starting up'
                : status === 'missing' ? 'no app container found'
                : status === 'down' ? 'unreachable or stopped'
                : 'checking…'}
              <li>
                <button
                  class="relative w-full flex items-center gap-2 pl-2 pr-2 py-1 text-xs rounded-md transition-colors
                    {active ? 'bg-accent/10 text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}
                    focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                  onclick={() => app.selectEnv(env)}
                  title={`${env.ssh_user}@${env.ssh_host} — ${tip}`}
                >
                  {#if active}
                    <span class="absolute -left-[10px] top-1.5 bottom-1.5 w-[2px] rounded bg-accent" aria-hidden="true"></span>
                  {/if}
                  <span
                    class="inline-block w-1.5 h-1.5 rounded-full shrink-0
                      {status === 'up'
                        ? 'bg-ok'
                        : status === 'starting'
                          ? 'bg-warn animate-pulse'
                          : status === 'missing'
                            ? 'bg-muted/60 border border-border'
                            : status === 'down'
                              ? 'bg-err'
                              : 'bg-muted/40 border border-border'}"
                    aria-label={tip}
                  ></span>
                  <span class="font-medium {active ? 'text-foreground' : ''}">{env.name}</span>
                  <span class="ml-auto font-mono text-[10px] text-muted/80 truncate">{env.ssh_host}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/each}

    <!-- Hosts -->
    <div class="mt-4 pb-1 flex items-center justify-between px-2">
      <button
        class="text-[10px] font-semibold uppercase tracking-widest text-muted hover:text-foreground"
        onclick={() => (hostsExpanded = !hostsExpanded)}
        aria-expanded={hostsExpanded}
      >
        {hostsExpanded ? "▾" : "▸"} Hosts
      </button>
      <button
        class="text-muted hover:text-foreground w-5 h-5 inline-flex items-center justify-center rounded-sm"
        onclick={() => { editingHostId = null; showHostEditor = "new"; }}
        aria-label="Add host"
        title="Add host"
      >+</button>
    </div>

    {#if hostsExpanded}
      {@const groups = hosts.aggregated()}
      <!-- Local shell — always available, doesn't need a saved host -->
      <div class="px-2 pt-1 text-[9px] font-medium uppercase tracking-widest text-muted/70">Local</div>
      <div class="space-y-px mb-2">
        <button
          class="w-full flex items-center gap-2 px-2 py-1 text-xs rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={() => terminals.addOrFocusTab({ local: true }, "Local shell", "~")}
          title="Open a local shell on this machine"
        >
          <span class="text-[11px] w-3 text-center">⌂</span>
          <span class="truncate">Local shell</span>
          <span class="ml-auto text-[10px] text-muted/70">home</span>
        </button>
        {#if app.activeProject}
          <button
            class="w-full flex items-center gap-2 px-2 py-1 text-xs rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            onclick={() => terminals.addOrFocusTab({ local: true, initial_cwd: app.activeProject!.path }, `Local · ${app.activeProject!.name}`, app.activeProject!.path)}
            title="Open a local shell in this project's folder"
          >
            <span class="text-[11px] w-3 text-center">▸</span>
            <span class="truncate">In {app.activeProject.name}</span>
          </button>
        {/if}
      </div>

      {#if groups.saved.length > 0}
        <div class="px-2 pt-1 text-[9px] font-medium uppercase tracking-widest text-muted/70">Saved</div>
        <div class="space-y-px mb-2">
          {#each groups.saved as h (h.key)}
            <div class="group flex items-center">
              <button
                class="flex-1 flex items-center gap-2 px-2 py-1 text-xs rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 min-w-0"
                onclick={() => openHost(h.target, h.label, h.subtitle)}
                title={h.subtitle}
              >
                <span class="w-1.5 h-1.5 rounded-full bg-muted/50 shrink-0"></span>
                <span class="truncate">{h.label}</span>
              </button>
              <button
                class="text-muted hover:text-foreground w-5 h-5 inline-flex items-center justify-center rounded opacity-0 group-hover:opacity-100 focus-visible:opacity-100 text-[11px]"
                onclick={() => { editingHostId = (h.original as any).id; showHostEditor = "edit"; }}
                aria-label="Edit {h.label}"
              >✎</button>
            </div>
          {/each}
        </div>
      {/if}

      {#if groups.sshConfig.length > 0}
        <div class="px-2 pt-1 text-[9px] font-medium uppercase tracking-widest text-muted/70">SSH config</div>
        <div class="space-y-px mb-2">
          {#each groups.sshConfig as h (h.key)}
            <button
              class="w-full flex items-center gap-2 px-2 py-1 text-xs rounded-md text-muted hover:text-foreground hover:bg-secondary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              onclick={() => openHost(h.target, h.label, h.subtitle)}
              title={h.subtitle || h.label}
            >
              <span class="w-1.5 h-1.5 rounded-full bg-muted/40 shrink-0"></span>
              <span class="truncate">{h.label}</span>
              {#if h.meta}<span class="ml-auto text-[10px] text-muted/70 truncate">{h.meta}</span>{/if}
            </button>
          {/each}
        </div>
      {/if}

      {#if groups.saved.length === 0 && groups.sshConfig.length === 0}
        <div class="px-2 py-2 text-xs text-muted">
          Add a host to get a quick-connect terminal.
        </div>
      {/if}
    {/if}
  </nav>

  <div class="border-t border-border p-2 space-y-1">
    <Button variant="outline" size="sm" class="w-full justify-start" onclick={addProject}>
      <span class="text-muted">+</span> Add project
    </Button>
    <Button variant="outline" size="sm" class="w-full justify-start" onclick={() => dashActions.openInit()}>
      Setup new project
    </Button>
    <div class="border-t border-border/60 pt-1 mt-1">
      <Button
        variant="ghost"
        size="sm"
        class="w-full justify-start text-muted"
        onclick={() => dashActions.openSettings()}
        aria-label="Open settings"
      >
        <svg viewBox="0 0 16 16" class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <circle cx="8" cy="8" r="2.2" />
          <path stroke-linecap="round" d="M8 1.5v1.8M8 12.7v1.8M14.5 8h-1.8M3.3 8H1.5M12.6 3.4l-1.3 1.3M4.7 11.3l-1.3 1.3M12.6 12.6l-1.3-1.3M4.7 4.7L3.4 3.4" />
        </svg>
        Settings
      </Button>
    </div>
  </div>
</aside>
{/if}

{#if showHostEditor}
  {@const existing = editingHostId ? hosts.saved.find(h => h.id === editingHostId) ?? null : null}
  {#await import("./HostEditorModal.svelte") then M}
    <M.default
      existing={existing}
      onClose={() => { showHostEditor = null; editingHostId = null; }}
    />
  {/await}
{/if}

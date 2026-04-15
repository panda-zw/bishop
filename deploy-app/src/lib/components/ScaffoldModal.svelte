<script lang="ts">
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";
  import { api } from "../api";
  import { toast } from "../toast.svelte";
  import {
    templates, generateCompose, generateDockerfile, generateDockerignore,
    generateSharedCompose, generateSharedTraefik, generateSharedInitDb,
    type TemplateKind, type ScaffoldContext,
  } from "../templates";

  interface Props {
    projectPath: string;
    appName: string;
    envName: string;
    gitRepo: string | null;
    domain: string | null;
    appPort: number;
    extraContainers: string;
    initialTab?: FileKey;
    onClose: () => void;
    onSaved?: () => void;
  }
  let {
    projectPath, appName, envName, gitRepo, domain, appPort, extraContainers,
    initialTab = "compose", onClose, onSaved,
  }: Props = $props();

  type FileKey = "compose" | "dockerfile" | "dockerignore" | "sharedCompose" | "sharedTraefik" | "sharedInitDb";

  interface FileDef {
    key: FileKey;
    label: string;
    path: string;
    group: "app" | "shared";
  }

  const files: FileDef[] = [
    { key: "compose",       label: "docker-compose.prod.yml", path: ".deploy/infra/docker-compose.prod.yml", group: "app" },
    { key: "dockerfile",    label: "Dockerfile",              path: "Dockerfile",                            group: "app" },
    { key: "dockerignore",  label: ".dockerignore",           path: ".dockerignore",                         group: "app" },
    { key: "sharedCompose", label: "docker-compose.yml",      path: ".deploy/infra/shared/docker-compose.yml",        group: "shared" },
    { key: "sharedTraefik", label: "traefik.yml",             path: ".deploy/infra/shared/traefik/traefik.yml",       group: "shared" },
    { key: "sharedInitDb",  label: "init-databases.sh",       path: ".deploy/infra/shared/init-databases.sh",         group: "shared" },
  ];

  /* svelte-ignore state_referenced_locally */
  let tab = $state<FileKey>(initialTab);
  let kind = $state<TemplateKind>("node-api");
  let content = $state<Record<FileKey, string>>({
    compose: "", dockerfile: "", dockerignore: "",
    sharedCompose: "", sharedTraefik: "", sharedInitDb: "",
  });
  let exists = $state<Record<FileKey, boolean>>({
    compose: false, dockerfile: false, dockerignore: false,
    sharedCompose: false, sharedTraefik: false, sharedInitDb: false,
  });
  let touched = $state<Record<FileKey, boolean>>({
    compose: false, dockerfile: false, dockerignore: false,
    sharedCompose: false, sharedTraefik: false, sharedInitDb: false,
  });
  let loaded = $state(false);
  let busy = $state(false);

  const ctx = $derived<ScaffoldContext>({
    appName, envName, gitRepo, domain, appPort, extraContainers,
  });

  function defaultFor(k: FileKey, templateKind: TemplateKind): string {
    switch (k) {
      case "compose":       return generateCompose(templateKind, ctx);
      case "dockerfile":    return generateDockerfile(templateKind, ctx);
      case "dockerignore":  return generateDockerignore(templateKind);
      case "sharedCompose": return generateSharedCompose();
      case "sharedTraefik": return generateSharedTraefik();
      case "sharedInitDb":  return generateSharedInitDb();
    }
  }

  async function readExisting(k: FileKey): Promise<string | null> {
    switch (k) {
      case "compose":       return await api.readComposeFile(projectPath);
      case "dockerfile":    return await api.readDockerfile(projectPath);
      case "dockerignore":  return await api.readDockerignore(projectPath);
      case "sharedCompose": return await api.readSharedCompose(projectPath);
      case "sharedTraefik": return await api.readSharedTraefik(projectPath);
      case "sharedInitDb":  return await api.readSharedInitDb(projectPath);
    }
  }

  async function hasFile(k: FileKey): Promise<boolean> {
    switch (k) {
      case "compose":       return await api.hasComposeFile(projectPath);
      case "dockerfile":    return await api.hasDockerfile(projectPath);
      case "dockerignore":  return await api.hasDockerignore(projectPath);
      case "sharedCompose": return (await api.sharedFileStatus(projectPath)).compose;
      case "sharedTraefik": return (await api.sharedFileStatus(projectPath)).traefik;
      case "sharedInitDb":  return (await api.sharedFileStatus(projectPath)).init_db;
    }
  }

  async function writeFile(k: FileKey, body: string): Promise<void> {
    switch (k) {
      case "compose":       return await api.writeComposeFile(projectPath, body);
      case "dockerfile":    return await api.writeDockerfile(projectPath, body);
      case "dockerignore":  return await api.writeDockerignore(projectPath, body);
      case "sharedCompose": return await api.writeSharedCompose(projectPath, body);
      case "sharedTraefik": return await api.writeSharedTraefik(projectPath, body);
      case "sharedInitDb":  return await api.writeSharedInitDb(projectPath, body);
    }
  }

  async function load() {
    try {
      for (const f of files) {
        const has = await hasFile(f.key);
        exists[f.key] = has;
        if (has) {
          content[f.key] = (await readExisting(f.key)) ?? "";
          touched[f.key] = true;
        } else {
          content[f.key] = defaultFor(f.key, kind);
        }
      }
      loaded = true;
    } catch (e) {
      toast.error("Failed to load files", String(e));
    }
  }

  function changeTemplate(k: TemplateKind) {
    kind = k;
    // Only regenerate app-level files that haven't been edited; shared files
    // are template-agnostic.
    if (!touched.compose)      content.compose = generateCompose(k, ctx);
    if (!touched.dockerfile)   content.dockerfile = generateDockerfile(k, ctx);
    if (!touched.dockerignore) content.dockerignore = generateDockerignore(k);
  }

  async function saveCurrent() {
    busy = true;
    try {
      await writeFile(tab, content[tab]);
      exists[tab] = true;
      toast.success(`Saved ${files.find(f => f.key === tab)!.label}`);
      onSaved?.();
    } catch (e) {
      toast.error("Save failed", String(e));
    } finally {
      busy = false;
    }
  }

  async function saveAllMissing() {
    busy = true;
    let saved = 0;
    try {
      for (const f of files) {
        if (!exists[f.key]) {
          await writeFile(f.key, content[f.key]);
          exists[f.key] = true;
          saved++;
        }
      }
      toast.success(`Created ${saved} file${saved === 1 ? "" : "s"}`);
      onSaved?.();
      onClose();
    } catch (e) {
      toast.error("Save failed", String(e));
    } finally {
      busy = false;
    }
  }

  function resetToTemplate() {
    content[tab] = defaultFor(tab, kind);
    touched[tab] = false;
  }

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLTextAreaElement;
    if (e.key === "Enter") {
      const { value, selectionStart: start } = target;
      const lineStart = value.lastIndexOf("\n", start - 1) + 1;
      const match = value.slice(lineStart, start).match(/^\s*/);
      const indent = match ? match[0] : "";
      if (indent) {
        e.preventDefault();
        const before = value.slice(0, start);
        const after = value.slice(target.selectionEnd);
        content[tab] = before + "\n" + indent + after;
        queueMicrotask(() => {
          target.selectionStart = target.selectionEnd = start + 1 + indent.length;
        });
      }
    } else if (e.key === "Tab") {
      e.preventDefault();
      const { value, selectionStart: start, selectionEnd: end } = target;
      content[tab] = value.slice(0, start) + "  " + value.slice(end);
      queueMicrotask(() => {
        target.selectionStart = target.selectionEnd = start + 2;
      });
    }
  }

  const missingCount = $derived(files.filter(f => !exists[f.key]).length);
  const appFiles = $derived(files.filter(f => f.group === "app"));
  const sharedFiles = $derived(files.filter(f => f.group === "shared"));

  $effect(() => { load(); });
</script>

<Modal title="Scaffold deployment files — {appName}" wide {onClose}>
  <div class="flex h-[70vh]">
    <!-- Left column: template picker + file list -->
    <div class="w-64 shrink-0 border-r border-border bg-card flex flex-col overflow-auto">
      <div class="px-4 pt-4 pb-2 text-[10px] font-semibold uppercase tracking-widest text-muted">App stack</div>
      <div class="px-2 space-y-0.5">
        {#each templates as t (t.kind)}
          <button
            type="button"
            class="w-full text-left px-2 py-1.5 rounded-md text-sm transition-colors
              {kind === t.kind ? 'bg-secondary text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}"
            onclick={() => changeTemplate(t.kind)}
          >
            <div class="font-medium">{t.label}</div>
            <div class="text-[11px] text-muted leading-snug">{t.languageHint}</div>
          </button>
        {/each}
      </div>

      <div class="px-4 pt-5 pb-2 text-[10px] font-semibold uppercase tracking-widest text-muted">App files</div>
      <div class="px-2 space-y-0.5">
        {#each appFiles as f (f.key)}
          <button
            type="button"
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors
              {tab === f.key ? 'bg-secondary text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}"
            onclick={() => (tab = f.key)}
          >
            <span class="w-1.5 h-1.5 rounded-full {exists[f.key] ? 'bg-ok' : 'bg-warn'}"></span>
            <span class="flex-1 truncate">{f.label}</span>
            <span class="text-[10px] text-muted">{exists[f.key] ? "exists" : "new"}</span>
          </button>
        {/each}
      </div>

      <div class="px-4 pt-5 pb-2 text-[10px] font-semibold uppercase tracking-widest text-muted">
        Shared infrastructure
        <span class="font-normal normal-case tracking-normal text-muted/70">· same across apps</span>
      </div>
      <div class="px-2 space-y-0.5">
        {#each sharedFiles as f (f.key)}
          <button
            type="button"
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors
              {tab === f.key ? 'bg-secondary text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}"
            onclick={() => (tab = f.key)}
          >
            <span class="w-1.5 h-1.5 rounded-full {exists[f.key] ? 'bg-ok' : 'bg-warn'}"></span>
            <span class="flex-1 truncate">{f.label}</span>
            <span class="text-[10px] text-muted">{exists[f.key] ? "exists" : "new"}</span>
          </button>
        {/each}
      </div>

      <div class="mt-auto p-3 border-t border-border">
        <div class="text-[11px] text-muted leading-snug">
          Files marked <span class="text-warn">new</span> don't exist yet. Save individually, or
          use "Create all missing" below to write them all with defaults.
        </div>
      </div>
    </div>

    <!-- Right column: editor -->
    <div class="flex-1 flex flex-col min-w-0">
      <div class="flex items-center justify-between px-4 h-10 border-b border-border">
        <div class="text-xs font-mono text-muted">
          {files.find(f => f.key === tab)?.path}
        </div>
        <button
          type="button"
          class="text-[11px] text-muted hover:text-foreground underline underline-offset-4"
          onclick={resetToTemplate}
          disabled={busy}
          title="Discard edits and regenerate from the default template"
        >reset to template</button>
      </div>

      {#if !loaded}
        <div class="flex-1 flex items-center justify-center text-xs text-muted">Loading…</div>
      {:else}
        <textarea
          class="flex-1 w-full resize-none bg-background text-foreground font-mono text-[12px] leading-[18px] px-4 py-3 outline-none focus:outline-none border-0"
          bind:value={content[tab]}
          oninput={() => (touched[tab] = true)}
          onkeydown={onKeydown}
          spellcheck="false"
          aria-label={files.find(f => f.key === tab)?.label}
        ></textarea>
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <Button size="sm" variant="outline" onclick={onClose} disabled={busy}>Cancel</Button>
    <Button size="sm" variant="outline" onclick={saveCurrent} disabled={busy || !loaded}>
      Save {files.find(f => f.key === tab)?.label}
    </Button>
    <Button size="sm" onclick={saveAllMissing} disabled={busy || !loaded || missingCount === 0}>
      {busy ? "Saving…" : missingCount === 0 ? "Everything exists" : `Create all missing (${missingCount})`}
    </Button>
  {/snippet}
</Modal>

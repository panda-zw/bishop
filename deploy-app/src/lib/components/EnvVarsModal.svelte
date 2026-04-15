<script lang="ts">
  import { api } from "../api";
  import type { EnvLine } from "../types";
  import Modal from "./Modal.svelte";
  import { confirmDialog } from "../confirm.svelte";
  import { toast } from "../toast.svelte";

  interface Props {
    projectPath: string;
    env: string;
    onClose: () => void;
  }
  let { projectPath, env, onClose }: Props = $props();

  let entries = $state<EnvLine[]>([]);
  let revealed = $state<Set<string>>(new Set());
  let editing = $state<string | null>(null);
  let editValue = $state("");
  let newKey = $state("");
  let newValue = $state("");
  let adding = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let dirty = $state(false);
  let opAvailable = $state(false);

  async function checkOp() {
    try {
      const s = await api.opStatus();
      opAvailable = s.installed && s.signed_in;
    } catch { opAvailable = false; }
  }

  async function fillFromOp(target: "edit" | "new") {
    const ref = window.prompt(
      "Paste a 1Password secret reference.\n\n" +
      "It looks like:  op://Vault/Item/field\n" +
      "(get one by right-clicking a field in 1Password → Copy Secret Reference)"
    );
    if (!ref) return;
    try {
      const v = await api.opRead(ref.trim());
      if (target === "edit") editValue = v; else newValue = v;
    } catch (e) {
      toast.error("1Password read failed", String(e));
    }
  }

  async function load() {
    loading = true;
    try {
      entries = await api.getEnvVars(projectPath, env);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function toggle(key: string) {
    const next = new Set(revealed);
    if (next.has(key)) next.delete(key); else next.add(key);
    revealed = next;
  }

  function mask(value: string) {
    if (value.length <= 4) return "••••";
    return value.slice(0, 4) + "••••";
  }

  function startEdit(key: string, value: string) {
    editing = key;
    editValue = value;
    revealed = new Set(revealed).add(key);
  }

  async function saveEdit(key: string) {
    busy = true;
    try {
      await api.setEnvVar(projectPath, env, key, editValue);
      dirty = true;
      editing = null;
      await load();
    } catch (e) {
      toast.error("Save failed", String(e));
    } finally {
      busy = false;
    }
  }

  async function remove(key: string) {
    const ok = await confirmDialog.ask({
      title: `Delete ${key}?`,
      message: "This removes the variable from the remote .env file.",
      confirmLabel: "Delete",
      destructive: true,
    });
    if (!ok) return;
    busy = true;
    try {
      await api.deleteEnvVar(projectPath, env, key);
      dirty = true;
      await load();
    } catch (e) {
      toast.error("Delete failed", String(e));
    } finally {
      busy = false;
    }
  }

  async function addVar() {
    const key = newKey.trim().toUpperCase();
    if (!key) return;
    busy = true;
    try {
      await api.setEnvVar(projectPath, env, key, newValue);
      dirty = true;
      newKey = "";
      newValue = "";
      adding = false;
      await load();
    } catch (e) {
      toast.error("Add failed", String(e));
    } finally {
      busy = false;
    }
  }

  async function close() {
    if (dirty) {
      const ok = await confirmDialog.ask({
        title: "Restart app to apply changes?",
        message: "Env var changes only take effect after the container restarts.",
        confirmLabel: "Restart now",
        cancelLabel: "Skip",
      });
      if (ok) {
        busy = true;
        try {
          await api.restartService(projectPath, env, "app");
          toast.success(`Restarted app on ${env}`);
        } catch (e) {
          toast.error("Restart failed", String(e));
        } finally {
          busy = false;
        }
      }
    }
    onClose();
  }

  $effect(() => { load(); checkOp(); });
</script>

<Modal title="Env vars — {env}" wide onClose={close}>
  <div class="p-4">
    {#if loading}
      <div class="text-sm text-muted">Loading…</div>
    {:else if error}
      <div class="text-sm text-err border border-err/30 bg-err/5 rounded px-3 py-2">{error}</div>
    {:else}
      <div class="font-mono text-xs space-y-0.5">
        {#each entries as entry, i (i)}
          {#if entry.kind === "comment"}
            <div class="text-warn">{entry.text}</div>
          {:else if entry.kind === "blank"}
            <div>&nbsp;</div>
          {:else if entry.kind === "var"}
            <div class="flex items-start gap-2 group">
              <span class="text-accent">{entry.key}</span>
              <span class="text-muted">=</span>
              {#if editing === entry.key}
                <input
                  class="flex-1 modal-sunken modal-sunken-border border rounded-md px-2 py-0.5 font-mono text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                  bind:value={editValue}
                  disabled={busy}
                />
                {#if opAvailable}
                  <button
                    class="text-muted hover:text-foreground text-[11px]"
                    onclick={() => fillFromOp("edit")}
                    disabled={busy}
                    title="Paste a value from 1Password using an op:// reference"
                  >from 1Password</button>
                {/if}
                <button
                  class="text-ok hover:underline text-[11px]"
                  onclick={() => saveEdit(entry.key)}
                  disabled={busy}
                >save</button>
                <button
                  class="text-muted hover:text-zinc-100 text-[11px]"
                  onclick={() => (editing = null)}
                  disabled={busy}
                >cancel</button>
              {:else}
                <span class="flex-1 break-all">
                  {entry.is_secret && !revealed.has(entry.key) ? mask(entry.value) : entry.value}
                </span>
                {#if entry.is_secret}
                  <button
                    class="text-muted hover:text-zinc-100 text-[11px] shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 group-focus-within:opacity-100"
                    onclick={() => toggle(entry.key)}
                  >{revealed.has(entry.key) ? "hide" : "show"}</button>
                {/if}
                <button
                  class="text-muted hover:text-zinc-100 text-[11px] shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 group-focus-within:opacity-100"
                  onclick={() => startEdit(entry.key, entry.value)}
                >edit</button>
                <button
                  class="text-err hover:underline text-[11px] shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 group-focus-within:opacity-100"
                  onclick={() => remove(entry.key)}
                >del</button>
              {/if}
            </div>
          {:else}
            <div class="text-muted">{entry.text}</div>
          {/if}
        {/each}
      </div>

      <div class="mt-4 pt-3 border-t border-border">
        {#if adding}
          <div class="flex gap-2 items-center font-mono text-xs">
            <input
              class="w-48 modal-sunken modal-sunken-border border rounded-md px-2 py-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              placeholder="KEY_NAME"
              bind:value={newKey}
              disabled={busy}
            />
            <span class="text-muted">=</span>
            <input
              class="flex-1 modal-sunken modal-sunken-border border rounded-md px-2 py-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              placeholder="value"
              bind:value={newValue}
              disabled={busy}
            />
            {#if opAvailable}
              <button class="text-muted text-[11px] hover:text-foreground" onclick={() => fillFromOp("new")} disabled={busy} title="Paste a value from 1Password using an op:// reference">from 1Password</button>
            {/if}
            <button class="text-ok text-[11px] hover:underline" onclick={addVar} disabled={busy}>add</button>
            <button class="text-muted text-[11px] hover:text-zinc-100" onclick={() => (adding = false)} disabled={busy}>cancel</button>
          </div>
        {:else}
          <button
            class="text-xs text-muted hover:text-zinc-100"
            onclick={() => (adding = true)}
          >+ Add variable</button>
        {/if}
      </div>
    {/if}
  </div>
</Modal>

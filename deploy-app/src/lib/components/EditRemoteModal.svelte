<script lang="ts">
  import { app } from "../stores.svelte";
  import { api } from "../api";
  import type { Environment, Project } from "../types";
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";
  import Input from "./ui/Input.svelte";
  import Field from "./ui/Field.svelte";

  interface Props {
    project: Project;
    env: Environment;
    onClose: () => void;
  }
  let { project, env, onClose }: Props = $props();

  // Local editable copies. Reading from the `env` prop here is intentional —
  // the modal opens once per edit session with a snapshot.
  /* svelte-ignore state_referenced_locally */
  let sshUser = $state(env.ssh_user);
  /* svelte-ignore state_referenced_locally */
  let sshHost = $state(env.ssh_host);
  /* svelte-ignore state_referenced_locally */
  let appName = $state(env.app_name);
  /* svelte-ignore state_referenced_locally */
  let domain = $state(env.domain ?? "");

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function save() {
    busy = true; error = null;
    try {
      const updated = await api.updateRemote({
        project_path: project.path,
        env_name: env.name,
        ssh_user: sshUser.trim(),
        ssh_host: sshHost.trim(),
        app_name: appName.trim(),
        domain: domain.trim() || null,
      });
      // Refresh in-memory project + re-select env so the dashboard updates.
      app.projects = app.projects.map(p => p.path === updated.path ? updated : p);
      if (app.activeProject?.path === updated.path) {
        app.activeProject = updated;
        const newEnv = updated.remotes.find(e => e.name === env.name) ?? updated.remotes[0] ?? null;
        app.activeEnv = newEnv;
        if (newEnv) app.probeOne(updated.path, newEnv.name);
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="Edit remote — {env.name}" {onClose}>
  <div class="p-5 space-y-4">
    <div class="grid grid-cols-2 gap-4">
      <Field label="SSH user" hint="The account you log in as on the server.">
        <Input bind:value={sshUser} />
      </Field>
      <Field label="Server address" hint="IP or domain name of the server.">
        <Input bind:value={sshHost} />
      </Field>
      <Field label="App name on server" hint="Unique name used for this app's containers and folders.">
        <Input bind:value={appName} />
      </Field>
      <Field label="Public domain" optional hint="The public URL the app is served from.">
        <Input bind:value={domain} />
      </Field>
    </div>
    {#if error}
      <div class="text-xs text-err border border-err/30 bg-err/10 rounded-md px-3 py-2 font-mono">{error}</div>
    {/if}
  </div>
  {#snippet footer()}
    <Button size="sm" variant="outline" onclick={onClose} disabled={busy}>Cancel</Button>
    <Button size="sm" disabled={busy || !sshHost || !appName} onclick={save}>
      {busy ? "Saving…" : "Save"}
    </Button>
  {/snippet}
</Modal>

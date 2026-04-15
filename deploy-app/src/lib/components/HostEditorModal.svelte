<script lang="ts">
  import Modal from "./Modal.svelte";
  import Button from "./ui/Button.svelte";
  import Input from "./ui/Input.svelte";
  import Field from "./ui/Field.svelte";
  import { hosts } from "../hosts.svelte";
  import { toast } from "../toast.svelte";
  import type { SavedHost } from "../types";

  interface Props {
    existing?: SavedHost | null;
    onClose: () => void;
  }
  let { existing = null, onClose }: Props = $props();

  /* svelte-ignore state_referenced_locally */
  let label = $state(existing?.label ?? "");
  /* svelte-ignore state_referenced_locally */
  let user = $state(existing?.user ?? "root");
  /* svelte-ignore state_referenced_locally */
  let host = $state(existing?.host ?? "");
  /* svelte-ignore state_referenced_locally */
  let port = $state<number | null>(existing?.port ?? null);
  /* svelte-ignore state_referenced_locally */
  let identity = $state(existing?.identity ?? "");
  /* svelte-ignore state_referenced_locally */
  let proxyJump = $state(existing?.proxy_jump ?? "");
  /* svelte-ignore state_referenced_locally */
  let initialCwd = $state(existing?.initial_cwd ?? "");
  /* svelte-ignore state_referenced_locally */
  let notes = $state(existing?.notes ?? "");
  /* svelte-ignore state_referenced_locally */
  let useTmux = $state(existing?.use_tmux ?? false);

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function save() {
    busy = true; error = null;
    try {
      const payload = {
        label: label.trim(),
        user: user.trim(),
        host: host.trim(),
        port: port || null,
        identity: identity.trim() || null,
        proxy_jump: proxyJump.trim() || null,
        initial_cwd: initialCwd.trim() || null,
        notes: notes.trim() || null,
        use_tmux: useTmux,
        tmux_session: null,
      };
      if (existing) {
        await hosts.update({ ...existing, ...payload });
        toast.success("Host updated");
      } else {
        await hosts.add(payload);
        toast.success("Host added");
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!existing) return;
    busy = true; error = null;
    try {
      await hosts.remove(existing.id);
      toast.success("Host removed");
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title={existing ? "Edit host" : "Add host"} {onClose}>
  <div class="p-5 space-y-4">
    <Field
      label="Name"
      hint="What you'll see in the sidebar. Short and memorable."
    >
      <Input bind:value={label} placeholder="dev-vps" />
    </Field>

    <div class="grid grid-cols-2 gap-4">
      <Field
        label="SSH user"
        hint="The account you log in as. Usually root, ubuntu, or the name of a deploy user."
      >
        <Input bind:value={user} />
      </Field>
      <Field
        label="Server address"
        hint="The IP address or domain name of the server."
      >
        <Input bind:value={host} placeholder="203.0.113.10 or example.com" />
      </Field>
      <Field
        label="Port"
        optional
        hint="SSH usually runs on 22. Leave blank unless your server is configured differently."
      >
        <Input type="number" placeholder="22" bind:value={port} />
      </Field>
      <Field
        label="Private key file"
        optional
        hint="Path to the SSH key that unlocks this server. Leave blank to use your default keys (~/.ssh) or ssh-agent."
      >
        <Input class="font-mono" placeholder="~/.ssh/id_ed25519" bind:value={identity} />
      </Field>
      <div class="col-span-2">
        <Field
          label="Connect through another server"
          optional
          hint={"Some servers sit on a private network and can only be reached by SSHing into a \u201Cgateway\u201D server first. If that's your setup, enter the gateway as user@host — otherwise leave blank."}
        >
          <Input class="font-mono" placeholder="user@bastion.example.com" bind:value={proxyJump} />
        </Field>
      </div>
      <div class="col-span-2">
        <Field
          label="Start in folder"
          optional
          hint="When the terminal opens, it will change to this folder automatically."
        >
          <Input class="font-mono" placeholder="/opt/app" bind:value={initialCwd} />
        </Field>
      </div>
      <div class="col-span-2">
        <Field
          label="Notes"
          optional
          hint="A reminder to yourself. Stays on your machine, never sent anywhere."
        >
          <Input bind:value={notes} />
        </Field>
      </div>
    </div>

    <div class="pt-2 border-t border-border">
      <label class="flex items-start gap-3 cursor-pointer select-none">
        <input
          type="checkbox"
          bind:checked={useTmux}
          class="mt-0.5 w-4 h-4 accent-accent"
        />
        <div class="flex-1">
          <div class="text-xs font-medium">Keep running programs alive between connections</div>
          <div class="text-[11px] text-muted leading-snug mt-0.5">
            Uses <span class="font-mono text-foreground">tmux</span> on the server to hold the shell open.
            If you leave something running (like a build or a log tail) and reconnect later, it's still there.
            Requires <span class="font-mono text-foreground">tmux</span> installed on the server
            (it is on most Linux distros). If it's missing, you'll get a regular shell with a clear message.
          </div>
        </div>
      </label>
    </div>

    {#if error}
      <div class="text-xs text-err border border-err/30 bg-err/10 rounded-md px-3 py-2 font-mono">{error}</div>
    {/if}
  </div>
  {#snippet footer()}
    {#if existing}
      <Button size="sm" variant="ghost" class="text-err hover:bg-err/10 mr-auto" onclick={remove} disabled={busy}>Delete</Button>
    {/if}
    <Button size="sm" variant="outline" onclick={onClose} disabled={busy}>Cancel</Button>
    <Button size="sm" disabled={busy || !label.trim() || !user.trim() || !host.trim()} onclick={save}>
      {busy ? "Saving…" : existing ? "Save" : "Add"}
    </Button>
  {/snippet}
</Modal>

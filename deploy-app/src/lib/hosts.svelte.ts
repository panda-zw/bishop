/**
 * Aggregates every host the user can connect to:
 *   - Ad-hoc saved hosts (persisted in Bishop settings.json)
 *   - Project remotes (from each tracked project's .deploy/remotes)
 *   - ~/.ssh/config Host aliases (excluding wildcards)
 */

import { api } from "./api";
import { app } from "./stores.svelte";
import type { SavedHost, SshConfigHost, TerminalTarget } from "./types";

export type HostSource = "saved" | "remote" | "ssh-config";

export interface AggregatedHost {
  key: string;
  source: HostSource;
  label: string;
  subtitle: string;
  target: TerminalTarget;
  meta?: string;
  original?: SavedHost | SshConfigHost;
}

function createHosts() {
  const state = $state({
    saved: [] as SavedHost[],
    sshConfig: [] as SshConfigHost[],
  });

  const self = {
    get saved() { return state.saved; },
    set saved(v: SavedHost[]) { state.saved = v; },

    get sshConfig() { return state.sshConfig; },

    async refresh() {
      try { state.saved = await api.listSavedHosts(); } catch {}
      try {
        // Dedupe by alias — ~/.ssh/config can declare the same alias in
        // multiple Host blocks; keeping duplicates breaks keyed Svelte `{#each}`.
        const raw = await api.listSshConfigHosts();
        const seen = new Set<string>();
        const unique: SshConfigHost[] = [];
        for (const h of raw) {
          if (seen.has(h.alias)) continue;
          seen.add(h.alias);
          unique.push(h);
        }
        state.sshConfig = unique;
      } catch {}
    },

    async add(host: Omit<SavedHost, "id">) {
      const created = await api.addSavedHost(host);
      state.saved = [...state.saved, created];
      api.refreshTray().catch(() => {});
      return created;
    },

    async update(host: SavedHost) {
      await api.updateSavedHost(host);
      state.saved = state.saved.map(h => h.id === host.id ? host : h);
      api.refreshTray().catch(() => {});
    },

    async remove(id: string) {
      await api.removeSavedHost(id);
      state.saved = state.saved.filter(h => h.id !== id);
      api.refreshTray().catch(() => {});
    },

    aggregated(): { saved: AggregatedHost[]; remotes: AggregatedHost[]; sshConfig: AggregatedHost[] } {
      const savedList: AggregatedHost[] = state.saved.map(s => ({
        key: `saved:${s.id}`,
        source: "saved",
        label: s.label,
        subtitle: `${s.user}@${s.host}${s.port ? `:${s.port}` : ""}`,
        target: {
          user: s.user, host: s.host, port: s.port ?? null,
          identity: s.identity ?? null, proxy_jump: s.proxy_jump ?? null,
          initial_cwd: s.initial_cwd ?? null,
          use_tmux: s.use_tmux ?? false,
          tmux_session: s.tmux_session ?? s.label,
        },
        original: s,
      }));

      const remoteList: AggregatedHost[] = app.projects.flatMap(p =>
        p.remotes.map(env => ({
          key: `remote:${p.path}:${env.name}`,
          source: "remote" as HostSource,
          label: `${env.name}`,
          subtitle: `${env.ssh_user}@${env.ssh_host}`,
          target: {
            user: env.ssh_user, host: env.ssh_host,
            initial_cwd: env.app_dir,
          },
          meta: p.name,
        }))
      );

      const seen = new Set<string>();
      const sshList: AggregatedHost[] = [];
      for (const h of state.sshConfig) {
        const key = `ssh:${h.alias}`;
        if (seen.has(key)) continue;
        seen.add(key);
        sshList.push({
          key,
          source: "ssh-config",
          label: h.alias,
          subtitle: h.hostname ? `${h.user ?? ""}${h.user ? "@" : ""}${h.hostname}${h.port ? `:${h.port}` : ""}` : "",
          target: { alias: h.alias },
          meta: h.proxy_jump ? `via ${h.proxy_jump}` : undefined,
          original: h,
        });
      }

      return { saved: savedList, remotes: remoteList, sshConfig: sshList };
    },
  };

  return self;
}

export const hosts = createHosts();
export type HostsStore = ReturnType<typeof createHosts>;

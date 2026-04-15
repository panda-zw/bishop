export type ConnectionState = "connected" | "disconnected" | "connecting" | "error";

export interface Project {
  path: string;
  name: string;
  git_repo: string | null;
  remotes: Environment[];
}

export interface Environment {
  name: string;
  ssh_user: string;
  ssh_host: string;
  app_name: string;
  domain: string | null;
  app_dir: string;
}

export interface Container {
  name: string;
  state: string;           // "running" | "restarting" | "exited" | "created"
  status: string;          // human string from docker
  image: string;
  ports: string[];
  health: string | null;   // "healthy" | "unhealthy" | "starting" | null
}

export interface LogLine {
  env: string;
  service: string;
  line: string;
  ts: string;
}

export type EnvLine =
  | { kind: "comment"; text: string }
  | { kind: "blank" }
  | { kind: "var"; key: string; value: string; is_secret: boolean }
  | { kind: "raw"; text: string };

export interface DeployDone {
  ok: boolean;
  code: number | null;
}

export interface DeployRow {
  id: number;
  project_path: string;
  env: string;
  started_at: string;
  finished_at: string | null;
  status: string;
  exit_code: number | null;
}

export interface TerminalTarget {
  /// When true, spawn a local shell on this machine — all ssh fields ignored.
  local?: boolean;
  shell?: string | null;

  alias?: string | null;
  user?: string | null;
  host?: string | null;
  port?: number | null;
  identity?: string | null;
  proxy_jump?: string | null;
  initial_cwd?: string | null;
  /// If true, the remote shell is wrapped in a named tmux session so it
  /// survives disconnects. Requires tmux on the remote.
  use_tmux?: boolean;
  tmux_session?: string | null;
}

export interface SavedHost {
  id: string;
  label: string;
  user: string;
  host: string;
  port?: number | null;
  identity?: string | null;
  proxy_jump?: string | null;
  initial_cwd?: string | null;
  notes?: string | null;
  use_tmux?: boolean;
  tmux_session?: string | null;
}

export interface SshConfigHost {
  alias: string;
  user?: string | null;
  hostname?: string | null;
  port?: number | null;
  identity_file?: string | null;
  proxy_jump?: string | null;
}

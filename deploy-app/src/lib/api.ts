import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Project, Container, EnvLine, DeployDone, DeployRow,
  TerminalTarget, SavedHost, SshConfigHost, BishopError,
} from "./types";

export interface SshTestResult {
  ok: boolean;
  message: string | null;
  error: BishopError | null;
  raw: string | null;
}

export const api = {
  addProject: (path: string) => invoke<Project>("add_project", { path }),
  listProjects: () => invoke<Project[]>("list_projects"),
  removeProject: (path: string) => invoke<void>("remove_project", { path }),
  getContainers: (projectPath: string, env: string) =>
    invoke<Container[]>("get_containers", { projectPath, env }),
  startLogStream: (projectPath: string, env: string, service: string) =>
    invoke<string>("start_log_stream", { projectPath, env, service }),
  stopLogStream: (streamId: string) =>
    invoke<void>("stop_log_stream", { streamId }),
  startDeploy: (projectPath: string, env: string) =>
    invoke<string>("start_deploy", { projectPath, env }),
  cancelDeploy: (streamId: string) =>
    invoke<void>("cancel_deploy", { streamId }),
  restartService: (projectPath: string, env: string, service: string) =>
    invoke<void>("restart_service", { projectPath, env, service }),
  getEnvVars: (projectPath: string, env: string) =>
    invoke<EnvLine[]>("get_env_vars", { projectPath, env }),
  setEnvVar: (projectPath: string, env: string, key: string, value: string) =>
    invoke<void>("set_env_var", { projectPath, env, key, value }),
  deleteEnvVar: (projectPath: string, env: string, key: string) =>
    invoke<void>("delete_env_var", { projectPath, env, key }),
  listDeploys: (projectPath: string, env: string) =>
    invoke<DeployRow[]>("list_deploys", { projectPath, env }),
  getDeployLog: (id: number) =>
    invoke<string | null>("get_deploy_log", { id }),
  startHealthCheck: (projectPath: string, env: string) =>
    invoke<string>("start_health_check", { projectPath, env }),
  cancelHealthCheck: (streamId: string) =>
    invoke<void>("cancel_health_check", { streamId }),
  startTerminal: (target: TerminalTarget, cols: number, rows: number) =>
    invoke<string>("start_terminal", { target, cols, rows }),
  termWrite: (streamId: string, data: string) =>
    invoke<void>("term_write", { streamId, data }),
  termResize: (streamId: string, cols: number, rows: number) =>
    invoke<void>("term_resize", { streamId, cols, rows }),
  termClose: (streamId: string) =>
    invoke<void>("term_close", { streamId }),
  startMetrics: (projectPath: string, env: string) =>
    invoke<string>("start_metrics", { projectPath, env }),
  stopMetrics: (streamId: string) =>
    invoke<void>("stop_metrics", { streamId }),
  testSsh: (user: string, host: string) =>
    invoke<SshTestResult>("test_ssh", { user, host }),
  cloneSample: (destParent: string, folderName: string) =>
    invoke<string>("clone_sample", { destParent, folderName }),
  opStatus: () => invoke<{ installed: boolean; signed_in: boolean }>("op_status"),
  opRead: (reference: string) => invoke<string>("op_read", { reference }),
  initProject: (input: InitProjectInput) => invoke<Project>("init_project", { input }),
  updateRemote: (input: UpdateRemoteInput) => invoke<Project>("update_remote", { input }),
  pingEnv: (projectPath: string, env: string) =>
    invoke<boolean>("ping_env", { projectPath, env }),
  startCliStep: (projectPath: string, subcommand: string, env: string) =>
    invoke<string>("start_cli_step", { projectPath, subcommand, env }),
  cancelCliStep: (streamId: string) =>
    invoke<void>("cancel_cli_step", { streamId }),
  installDeployScript: (projectPath: string) =>
    invoke<string>("install_deploy_script", { projectPath }),
  hasLocalDeployScript: (projectPath: string) =>
    invoke<boolean>("has_local_deploy_script", { projectPath }),
  listSavedHosts: () => invoke<SavedHost[]>("list_saved_hosts"),
  addSavedHost: (host: Omit<SavedHost, "id"> & { id?: string }) =>
    invoke<SavedHost>("add_saved_host", { host: { id: "", ...host } }),
  updateSavedHost: (host: SavedHost) =>
    invoke<void>("update_saved_host", { host }),
  removeSavedHost: (id: string) =>
    invoke<void>("remove_saved_host", { id }),
  listSshConfigHosts: () => invoke<SshConfigHost[]>("list_ssh_config_hosts"),
  readScrollback: (hostKey: string) =>
    invoke<{ data: string; saved_at: number }>("read_scrollback", { hostKey }),
  writeScrollback: (hostKey: string, data: string) =>
    invoke<void>("write_scrollback", { hostKey, data }),
  clearScrollback: (hostKey: string) =>
    invoke<void>("clear_scrollback", { hostKey }),
  refreshTray: () => invoke<void>("refresh_tray"),
  setTrayTooltip: (tooltip: string) => invoke<void>("set_tray_tooltip", { tooltip }),
  hasComposeFile: (projectPath: string) => invoke<boolean>("has_compose_file", { projectPath }),
  readComposeFile: (projectPath: string) => invoke<string | null>("read_compose_file", { projectPath }),
  writeComposeFile: (projectPath: string, content: string) =>
    invoke<void>("write_compose_file", { projectPath, content }),
  hasDockerfile: (projectPath: string) => invoke<boolean>("has_dockerfile", { projectPath }),
  readDockerfile: (projectPath: string) => invoke<string | null>("read_dockerfile", { projectPath }),
  writeDockerfile: (projectPath: string, content: string) =>
    invoke<void>("write_dockerfile", { projectPath, content }),

  hasSharedFiles: (projectPath: string) => invoke<boolean>("has_shared_files", { projectPath }),
  sharedFileStatus: (projectPath: string) =>
    invoke<{ compose: boolean; init_db: boolean; traefik: boolean }>("shared_file_status", { projectPath }),
  readSharedCompose: (projectPath: string) => invoke<string | null>("read_shared_compose", { projectPath }),
  writeSharedCompose: (projectPath: string, content: string) =>
    invoke<void>("write_shared_compose", { projectPath, content }),
  readSharedTraefik: (projectPath: string) => invoke<string | null>("read_shared_traefik", { projectPath }),
  writeSharedTraefik: (projectPath: string, content: string) =>
    invoke<void>("write_shared_traefik", { projectPath, content }),
  readSharedInitDb: (projectPath: string) => invoke<string | null>("read_shared_init_db", { projectPath }),
  writeSharedInitDb: (projectPath: string, content: string) =>
    invoke<void>("write_shared_init_db", { projectPath, content }),

  hasDockerignore: (projectPath: string) => invoke<boolean>("has_dockerignore", { projectPath }),
  readDockerignore: (projectPath: string) => invoke<string | null>("read_dockerignore", { projectPath }),
  writeDockerignore: (projectPath: string, content: string) =>
    invoke<void>("write_dockerignore", { projectPath, content }),
};

export interface UpdateRemoteInput {
  project_path: string;
  env_name: string;
  ssh_user: string;
  ssh_host: string;
  app_name: string;
  domain?: string | null;
}

export interface InitProjectInput {
  project_path: string;
  env_name: string;
  ssh_user: string;
  ssh_host: string;
  app_name: string;
  domain?: string | null;
  app_port?: number | null;
  health_string?: string | null;
  extra_containers?: string | null;
  data_dirs?: string | null;
  auto_secrets?: string | null;
  domain_templates?: string | null;
}

export async function onLogLine(
  streamId: string,
  cb: (line: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`logs:${streamId}`, (e) => cb(e.payload));
}

export async function onDeployLine(
  streamId: string,
  cb: (line: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`deploy:${streamId}`, (e) => cb(e.payload));
}

export async function onDeployDone(
  streamId: string,
  cb: (result: DeployDone) => void,
): Promise<UnlistenFn> {
  return listen<DeployDone>(`deploy:${streamId}:done`, (e) => cb(e.payload));
}

export async function onDeployError(
  streamId: string,
  cb: (err: import("./types").BishopError) => void,
): Promise<UnlistenFn> {
  return listen<import("./types").BishopError>(`deploy:${streamId}:error`, (e) => cb(e.payload));
}

export async function onHealthLine(
  streamId: string,
  cb: (line: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`health:${streamId}`, (e) => cb(e.payload));
}

export async function onHealthDone(
  streamId: string,
  cb: (result: DeployDone) => void,
): Promise<UnlistenFn> {
  return listen<DeployDone>(`health:${streamId}:done`, (e) => cb(e.payload));
}

export async function onTermOut(
  streamId: string,
  cb: (b64: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`term:${streamId}:out`, (e) => cb(e.payload));
}

export async function onTermExit(
  streamId: string,
  cb: (code: number) => void,
): Promise<UnlistenFn> {
  return listen<number>(`term:${streamId}:exit`, (e) => cb(e.payload));
}

export async function onMetrics(
  streamId: string,
  cb: (json: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`metrics:${streamId}`, (e) => cb(e.payload));
}

export async function onStepLine(
  streamId: string,
  cb: (line: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(`step:${streamId}`, (e) => cb(e.payload));
}

export async function onStepDone(
  streamId: string,
  cb: (result: DeployDone) => void,
): Promise<UnlistenFn> {
  return listen<DeployDone>(`step:${streamId}:done`, (e) => cb(e.payload));
}

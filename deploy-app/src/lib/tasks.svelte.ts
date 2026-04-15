/**
 * Background task manager (factory-style store).
 *
 * Owns the lifecycle of long-running CLI streams (deploy / check / step).
 * Modals render state from here instead of holding it locally, so closing a
 * modal doesn't kill the stream — it just hides it. A running task keeps
 * accumulating lines and can be re-attached to its modal later.
 */

import { api, onDeployLine, onDeployDone, onHealthLine, onHealthDone, onStepLine, onStepDone } from "./api";
import { toast } from "./toast.svelte";
import type { UnlistenFn } from "@tauri-apps/api/event";

export type TaskKind = "deploy" | "check" | "step";
export type TaskStatus = "running" | "success" | "failed" | "cancelled";

export interface Task {
  id: string;
  streamId: string;
  kind: TaskKind;
  title: string;
  commandLine: string;
  description?: string;
  projectPath: string;
  env: string;
  status: TaskStatus;
  exitCode: number | null;
  lines: string[];
  startedAt: number;
  finishedAt: number | null;
}

const MAX_LINES = 5000;

interface Listeners {
  line: UnlistenFn | null;
  done: UnlistenFn | null;
}

function createTasks() {
  const state = $state({
    items: [] as Task[],
    activeId: null as string | null,
  });

  const listeners = new Map<string, Listeners>();
  let seq = 0;

  function byId(id: string): Task | null {
    return state.items.find(t => t.id === id) ?? null;
  }

  function pushLine(id: string, line: string) {
    const i = state.items.findIndex(t => t.id === id);
    if (i < 0) return;
    const t = state.items[i];
    const next = t.lines.length >= MAX_LINES
      ? [...t.lines.slice(t.lines.length - MAX_LINES + 1), line]
      : [...t.lines, line];
    state.items[i] = { ...t, lines: next };
  }

  function updateStatus(id: string, status: TaskStatus, code: number | null) {
    const i = state.items.findIndex(t => t.id === id);
    if (i < 0) return;
    state.items[i] = { ...state.items[i], status, exitCode: code, finishedAt: Date.now() };
  }

  function finishTask(id: string, result: { ok: boolean; code: number | null }) {
    const t = byId(id);
    if (!t || t.status !== "running") return;
    updateStatus(id, result.ok ? "success" : "failed", result.code);
    detach(id);
  }

  function detach(id: string) {
    const l = listeners.get(id);
    if (!l) return;
    try { l.line?.(); } catch {}
    try { l.done?.(); } catch {}
    listeners.delete(id);
  }

  function createTask(
    partial: Omit<Task, "id" | "status" | "exitCode" | "lines" | "startedAt" | "finishedAt">,
  ): Task {
    const id = `task-${++seq}`;
    const task: Task = {
      id,
      ...partial,
      status: "running",
      exitCode: null,
      lines: [],
      startedAt: Date.now(),
      finishedAt: null,
    };
    state.items = [...state.items, task];
    state.activeId = id;
    return task;
  }

  async function attach(id: string, fns: { onLine: () => Promise<UnlistenFn>; onDone: () => Promise<UnlistenFn> }) {
    const l: Listeners = { line: null, done: null };
    listeners.set(id, l);
    l.line = await fns.onLine();
    l.done = await fns.onDone();
  }

  function toastOnFinish(id: string) {
    const check = () => {
      const t = byId(id);
      if (!t) return;
      if (t.status === "running") { setTimeout(check, 400); return; }
      if (state.activeId === id) return;
      if (t.status === "success") toast.success(t.title, "Completed successfully.");
      else if (t.status === "failed") toast.error(t.title, `Failed${t.exitCode !== null ? ` (exit ${t.exitCode})` : ""}`);
    };
    setTimeout(check, 400);
  }

  const self = {
    get items() { return state.items; },
    get activeId() { return state.activeId; },

    get active(): Task | null {
      return state.items.find(t => t.id === state.activeId) ?? null;
    },

    get running(): Task[] {
      return state.items.filter(t => t.status === "running");
    },

    byId,

    show(id: string) { state.activeId = id; },
    hide() { state.activeId = null; },

    dismiss(id: string) {
      const t = byId(id);
      if (!t || t.status === "running") return;
      detach(id);
      state.items = state.items.filter(x => x.id !== id);
      if (state.activeId === id) state.activeId = null;
    },

    async cancel(id: string) {
      const t = byId(id);
      if (!t || t.status !== "running") return;
      try {
        if (t.kind === "deploy") await api.cancelDeploy(t.streamId);
        else if (t.kind === "check") await api.cancelHealthCheck(t.streamId);
        else if (t.kind === "step") await api.cancelCliStep(t.streamId);
      } catch {}
      updateStatus(id, "cancelled", null);
    },

    async startDeploy(projectPath: string, env: string): Promise<string> {
      const streamId = await api.startDeploy(projectPath, env);
      const task = createTask({
        streamId, kind: "deploy",
        title: `Deploy — ${env}`,
        commandLine: `./deploy ${env}`,
        projectPath, env,
      });
      attach(task.id, {
        onLine: () => onDeployLine(streamId, (l) => pushLine(task.id, l)),
        onDone: () => onDeployDone(streamId, (r) => finishTask(task.id, r)),
      });
      toastOnFinish(task.id);
      return task.id;
    },

    async startHealthCheck(projectPath: string, env: string): Promise<string> {
      const streamId = await api.startHealthCheck(projectPath, env);
      const task = createTask({
        streamId, kind: "check",
        title: `Health check — ${env}`,
        commandLine: `./deploy check ${env}`,
        projectPath, env,
      });
      attach(task.id, {
        onLine: () => onHealthLine(streamId, (l) => pushLine(task.id, l)),
        onDone: () => onHealthDone(streamId, (r) => finishTask(task.id, r)),
      });
      toastOnFinish(task.id);
      return task.id;
    },

    async startCliStep(
      projectPath: string,
      subcommand: string,
      env: string,
      opts?: { title?: string; description?: string },
    ): Promise<string> {
      const streamId = await api.startCliStep(projectPath, subcommand, env);
      const task = createTask({
        streamId, kind: "step",
        title: opts?.title ?? `${subcommand} — ${env}`,
        commandLine: `./deploy ${subcommand} ${env}`,
        description: opts?.description,
        projectPath, env,
      });
      attach(task.id, {
        onLine: () => onStepLine(streamId, (l) => pushLine(task.id, l)),
        onDone: () => onStepDone(streamId, (r) => finishTask(task.id, r)),
      });
      toastOnFinish(task.id);
      return task.id;
    },
  };

  return self;
}

export const tasks = createTasks();
export type TasksStore = ReturnType<typeof createTasks>;

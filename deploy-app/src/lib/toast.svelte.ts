export type ToastKind = "info" | "success" | "error" | "warn";

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  body?: string;
}

function createToaster() {
  const state = $state({ items: [] as Toast[] });
  let seq = 0;

  function dismiss(id: number) {
    state.items = state.items.filter(t => t.id !== id);
  }

  function push(t: Omit<Toast, "id">, ttl = 5000) {
    const id = ++seq;
    state.items = [...state.items, { ...t, id }];
    if (ttl > 0) setTimeout(() => dismiss(id), ttl);
    return id;
  }

  return {
    get items() { return state.items; },
    push,
    dismiss,
    info(title: string, body?: string)    { return push({ kind: "info",    title, body }); },
    success(title: string, body?: string) { return push({ kind: "success", title, body }); },
    error(title: string, body?: string)   { return push({ kind: "error",   title, body }, 8000); },
    warn(title: string, body?: string)    { return push({ kind: "warn",    title, body }); },
  };
}

export const toast = createToaster();
export type Toaster = ReturnType<typeof createToaster>;

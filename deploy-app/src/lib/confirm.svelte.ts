export interface ConfirmOptions {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

interface PendingConfirm extends ConfirmOptions {
  resolve: (ok: boolean) => void;
}

function createConfirm() {
  const state = $state<{ pending: PendingConfirm | null }>({ pending: null });

  return {
    get pending() { return state.pending; },

    ask(opts: ConfirmOptions): Promise<boolean> {
      return new Promise<boolean>((resolve) => {
        state.pending = { ...opts, resolve };
      });
    },

    answer(ok: boolean) {
      if (!state.pending) return;
      state.pending.resolve(ok);
      state.pending = null;
    },
  };
}

export const confirmDialog = createConfirm();
export type ConfirmStore = ReturnType<typeof createConfirm>;

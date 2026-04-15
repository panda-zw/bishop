function createPalette() {
  const state = $state({ open: false });
  const self = {
    get open() { return state.open; },
    show() { state.open = true; },
    hide() { state.open = false; },
    toggle() { state.open = !state.open; },
  };
  return self;
}

export const palette = createPalette();
export type PaletteStore = ReturnType<typeof createPalette>;

export interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
  group?: string;
  run: () => void | Promise<void>;
}

/// Simple subsequence-match scorer: returns a score or -Infinity if no match.
/// Higher score = better. Rewards consecutive matches and matches at word starts.
export function score(query: string, text: string): number {
  if (!query) return 0;
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  let ti = 0;
  let s = 0;
  let streak = 0;
  for (const ch of q) {
    let found = -1;
    for (let i = ti; i < t.length; i++) {
      if (t[i] === ch) { found = i; break; }
    }
    if (found === -1) return -Infinity;
    const gap = found - ti;
    streak = gap === 0 ? streak + 1 : 1;
    const atWordStart = found === 0 || /[^a-z0-9]/.test(t[found - 1]);
    s += 10 + streak * 3 + (atWordStart ? 6 : 0) - gap;
    ti = found + 1;
  }
  s -= (t.length - ti) * 0.1;
  return s;
}

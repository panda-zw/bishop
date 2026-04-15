type Theme = "dark" | "light";

function createTheme() {
  function initial(): Theme {
    try {
      const stored = localStorage.getItem("bishop-theme");
      if (stored === "light" || stored === "dark") return stored;
    } catch {}
    return "dark";
  }

  const state = $state({ current: initial() });

  function apply() {
    document.documentElement.setAttribute("data-theme", state.current);
    try { localStorage.setItem("bishop-theme", state.current); } catch {}
  }

  return {
    get current() { return state.current; },
    apply,
    toggle() {
      state.current = state.current === "dark" ? "light" : "dark";
      apply();
    },
    set(t: Theme) {
      state.current = t;
      apply();
    },
  };
}

export const theme = createTheme();
export type ThemeStore = ReturnType<typeof createTheme>;

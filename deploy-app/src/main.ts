import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

// One-time: migrate legacy overlord-* localStorage keys to bishop-*.
// Runs before any store reads so hydration picks up the new keys.
(function migrateLegacyStorage() {
  try {
    const pairs: [string, string][] = [
      ["overlord-theme",             "bishop-theme"],
      ["overlord-sidebar-collapsed", "bishop-sidebar-collapsed"],
      ["overlord-terminal-height",   "bishop-terminal-height"],
    ];
    for (const [oldKey, newKey] of pairs) {
      if (localStorage.getItem(newKey) !== null) continue;
      const old = localStorage.getItem(oldKey);
      if (old !== null) {
        localStorage.setItem(newKey, old);
        localStorage.removeItem(oldKey);
      }
    }
  } catch { /* localStorage unavailable — fine, fresh defaults take over */ }
})();

const app = mount(App, { target: document.getElementById("app")! });
export default app;

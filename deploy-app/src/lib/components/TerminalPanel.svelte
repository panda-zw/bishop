<script lang="ts">
  import { terminals, type PanelLayout } from "../terminals.svelte";
  import TerminalTab from "./TerminalTab.svelte";
  import HostPicker from "./HostPicker.svelte";

  let pickerOpen = $state(false);
  let layoutMenuOpen = $state(false);
  /// Tab id currently being renamed (if any), plus the draft label.
  let renamingId = $state<string | null>(null);
  let renameDraft = $state("");

  function startRename(id: string, current: string) {
    renamingId = id;
    renameDraft = current;
  }
  function commitRename() {
    if (renamingId) {
      terminals.renameTab(renamingId, renameDraft);
    }
    renamingId = null;
    renameDraft = "";
  }
  function cancelRename() {
    renamingId = null;
    renameDraft = "";
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitRename(); }
    else if (e.key === "Escape") { e.preventDefault(); cancelRename(); }
  }

  // Resize drag state.
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartHeight = 0;

  function onDragStart(e: PointerEvent) {
    if (terminals.maximized) return;  // no resize while maximized
    dragging = true;
    dragStartY = e.clientY;
    dragStartHeight = terminals.panelHeight;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onDragMove(e: PointerEvent) {
    if (!dragging) return;
    const dy = dragStartY - e.clientY;
    terminals.setPanelHeight(dragStartHeight + dy);
  }
  function onDragEnd(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    try { (e.target as HTMLElement).releasePointerCapture(e.pointerId); } catch {}
  }

  function statusDot(s: string) {
    if (s === "open") return "bg-ok";
    if (s === "connecting") return "bg-warn animate-pulse";
    if (s === "error") return "bg-err";
    return "bg-muted";
  }

  /// Default action for the + button: clone the active tab's host.
  /// When nothing's open yet, show the picker.
  function duplicateCurrentOrPick() {
    const cur = terminals.active;
    if (cur) {
      terminals.addTab(cur.target, cur.label, cur.subtitle);
    } else {
      pickerOpen = true;
    }
  }

  function closeOnClickOutside(node: HTMLElement, cb: () => void) {
    const handler = (e: MouseEvent) => {
      if (!node.contains(e.target as Node)) cb();
    };
    document.addEventListener("mousedown", handler);
    return { destroy() { document.removeEventListener("mousedown", handler); } };
  }

  function layoutIcon(l: PanelLayout) {
    // A tiny svg glyph per layout mode.
    return l;
  }
  const layoutLabel: Record<PanelLayout, string> = {
    single: "Single",
    rows: "Split rows",
    cols: "Split columns",
    grid: "Grid",
  };

  /// Grid geometry for the current layout. Returns the container style + a
  /// helper that tells each pane how many columns/rows it should span — used
  /// so the last pane fills leftover cells when the count doesn't divide evenly.
  const grid = $derived.by(() => {
    const n = Math.max(1, terminals.tabs.length);
    const layout = terminals.layout;
    if (layout === "single" || n === 1) {
      return {
        style: "display:grid; grid-template-columns: 1fr; grid-template-rows: 1fr;",
        cols: 1,
      };
    }
    if (layout === "rows") {
      return {
        style: `display:grid; grid-template-rows: repeat(${n}, minmax(0, 1fr)); gap: 2px;`,
        cols: 1,
      };
    }
    if (layout === "cols") {
      return {
        style: `display:grid; grid-template-columns: repeat(${n}, minmax(0, 1fr)); gap: 2px;`,
        cols: n,
      };
    }
    // grid: 2 columns, as many rows as needed
    const cols = Math.min(2, n);
    const rows = Math.ceil(n / cols);
    return {
      style: `display:grid; grid-template-columns: repeat(${cols}, minmax(0, 1fr)); grid-template-rows: repeat(${rows}, minmax(0, 1fr)); gap: 2px;`,
      cols,
    };
  });

  /// How many columns should the pane at `index` span? Only the last pane when
  /// the final row is short spans extra — every other pane is a normal cell.
  function spanFor(index: number): number {
    const n = terminals.tabs.length;
    const cols = grid.cols;
    if (cols <= 1) return 1;
    if (index !== n - 1) return 1;
    const leftover = n % cols;
    return leftover === 0 ? 1 : cols - leftover + 1;
  }

  const isSplit = $derived(terminals.layout !== "single" && terminals.tabs.length > 1);
</script>

<!-- Drag handle — hidden when panel closed or maximized -->
<div
  class="h-1.5 shrink-0 bg-border hover:bg-ring/50 cursor-row-resize {terminals.open && !terminals.maximized ? '' : 'hidden'} {dragging ? 'bg-ring/50' : ''}"
  onpointerdown={onDragStart}
  onpointermove={onDragMove}
  onpointerup={onDragEnd}
  onpointercancel={onDragEnd}
  role="separator"
  aria-orientation="horizontal"
  aria-label="Resize terminal panel"
></div>

<section
  class="flex flex-col border-t border-border bg-card {terminals.open ? '' : 'hidden'} {terminals.maximized ? 'flex-1 min-h-0' : 'shrink-0'}"
  style={terminals.maximized ? "" : `height: ${terminals.panelHeight}px;`}
  aria-label="Terminal"
>
  <!-- Tab bar -->
  <div class="flex items-center gap-1 pl-2 pr-2 border-b border-border min-h-[36px]">
    <div class="flex items-center gap-0.5 flex-1 overflow-x-auto">
      {#each terminals.tabs as tab (tab.id)}
        {@const active = tab.id === terminals.activeId}
        <div
          class="group flex items-center gap-2 h-7 pl-2.5 pr-1 rounded-md text-xs transition-colors shrink-0
            {active ? 'bg-secondary text-foreground' : 'text-muted hover:text-foreground hover:bg-secondary/60'}"
        >
          <button
            type="button"
            class="flex items-center gap-2 min-w-0 focus-visible:outline-none"
            onclick={() => terminals.setActive(tab.id)}
            ondblclick={() => startRename(tab.id, tab.label)}
            title={`${tab.subtitle}  ·  double-click to rename`}
          >
            <span class="w-1.5 h-1.5 rounded-full shrink-0 {statusDot(tab.status)}"></span>
            {#if renamingId === tab.id}
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="max-w-[180px] h-5 px-1 bg-background border border-ring/60 rounded-sm text-xs focus:outline-none"
                bind:value={renameDraft}
                onkeydown={onRenameKey}
                onblur={commitRename}
                onclick={(e) => e.stopPropagation()}
                autofocus
                aria-label="Rename tab"
              />
            {:else}
              <span class="max-w-[180px] truncate">{tab.label}</span>
            {/if}
          </button>
          <button
            type="button"
            class="text-muted hover:text-foreground w-4 h-4 inline-flex items-center justify-center rounded-sm opacity-0 group-hover:opacity-100 focus-visible:opacity-100 text-[10px]"
            onclick={(e) => { e.stopPropagation(); startRename(tab.id, tab.label); }}
            aria-label="Rename tab {tab.label}"
            title="Rename (or double-click the name)"
          >
            <svg viewBox="0 0 12 12" class="w-3 h-3" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" d="M8.5 2l1.5 1.5-6 6H2.5V8z"/>
            </svg>
          </button>
          <button
            type="button"
            class="text-muted hover:text-err w-4 h-4 inline-flex items-center justify-center rounded-sm opacity-0 group-hover:opacity-100 focus-visible:opacity-100 text-[11px]"
            onclick={(e) => { e.stopPropagation(); terminals.closeTab(tab.id); }}
            aria-label="Close tab {tab.label}"
          >✕</button>
        </div>
      {/each}

      <!-- New tab (split button: click adds same-host tab; chevron opens picker) -->
      <div class="flex items-center shrink-0">
        <button
          type="button"
          class="h-7 pl-2 pr-1.5 inline-flex items-center justify-center rounded-l-md rounded-r-none text-muted hover:text-foreground hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={duplicateCurrentOrPick}
          aria-label="New terminal tab"
          title={terminals.active ? `New tab on ${terminals.active.label}` : "Open a terminal"}
        >+</button>
        <button
          type="button"
          class="h-7 pl-0.5 pr-1.5 inline-flex items-center justify-center rounded-r-md rounded-l-none border-l border-border/60 text-muted hover:text-foreground hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          onclick={() => (pickerOpen = true)}
          aria-label="Open tab to a different host"
          title="Open tab on a different host"
        >
          <svg viewBox="0 0 10 10" class="w-2.5 h-2.5" fill="currentColor" aria-hidden="true">
            <path d="M2 3.5l3 3 3-3H2z" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Right-side controls -->
    <div class="flex items-center gap-1 shrink-0">
      <!-- Font size -->
      <div class="flex items-center h-7 rounded-md border border-border/60 overflow-hidden">
        <button
          type="button"
          class="h-7 w-7 inline-flex items-center justify-center text-muted hover:text-foreground hover:bg-secondary disabled:opacity-40 disabled:cursor-not-allowed"
          onclick={() => terminals.bumpFontSize(-1)}
          disabled={terminals.fontSize <= 8}
          aria-label="Decrease terminal font size"
          title="Decrease font size"
        >
          <svg viewBox="0 0 10 10" class="w-2.5 h-2.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path stroke-linecap="round" d="M2.5 5h5"/>
          </svg>
        </button>
        <button
          type="button"
          class="h-7 px-1.5 text-[11px] font-mono text-muted hover:text-foreground hover:bg-secondary border-l border-r border-border/60"
          onclick={() => terminals.resetFontSize()}
          aria-label="Reset terminal font size"
          title="Reset font size (13)"
        >{terminals.fontSize}</button>
        <button
          type="button"
          class="h-7 w-7 inline-flex items-center justify-center text-muted hover:text-foreground hover:bg-secondary disabled:opacity-40 disabled:cursor-not-allowed"
          onclick={() => terminals.bumpFontSize(1)}
          disabled={terminals.fontSize >= 28}
          aria-label="Increase terminal font size"
          title="Increase font size"
        >
          <svg viewBox="0 0 10 10" class="w-2.5 h-2.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path stroke-linecap="round" d="M5 2.5v5M2.5 5h5"/>
          </svg>
        </button>
      </div>

      <!-- Layout menu -->
      <div class="relative" use:closeOnClickOutside={() => (layoutMenuOpen = false)}>
        <button
          type="button"
          class="h-7 px-2 inline-flex items-center gap-1 rounded-md text-xs text-muted hover:text-foreground hover:bg-secondary"
          onclick={() => (layoutMenuOpen = !layoutMenuOpen)}
          aria-haspopup="menu"
          aria-expanded={layoutMenuOpen}
          title="Change how tabs are arranged"
        >
          {layoutLabel[terminals.layout]}
          <svg viewBox="0 0 10 10" class="w-2.5 h-2.5" fill="currentColor" aria-hidden="true">
            <path d="M2 3.5l3 3 3-3H2z"/>
          </svg>
        </button>
        {#if layoutMenuOpen}
          <div class="absolute right-0 top-full mt-1 w-52 bishop-modal border rounded-md shadow-lg py-1 z-20">
            {#each (["single", "rows", "cols", "grid"] as PanelLayout[]) as opt (opt)}
              <button
                type="button"
                role="menuitem"
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-secondary flex items-center gap-2
                  {terminals.layout === opt ? 'text-foreground' : 'text-muted'}"
                onclick={() => { terminals.setLayout(opt); layoutMenuOpen = false; }}
              >
                <span class="w-3 text-center">{terminals.layout === opt ? '✓' : ''}</span>
                <span class="flex-1">{layoutLabel[opt]}</span>
                <span class="text-[10px] text-muted">
                  {#if opt === "single"}one at a time
                  {:else if opt === "rows"}stacked
                  {:else if opt === "cols"}side by side
                  {:else}2-column grid
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <button
        type="button"
        class="h-7 w-7 inline-flex items-center justify-center rounded-md text-muted hover:text-foreground hover:bg-secondary"
        onclick={() => terminals.toggleMaximize()}
        aria-label={terminals.maximized ? "Restore terminal height" : "Maximize terminal"}
        title={terminals.maximized ? "Shrink terminal (⌘\\)" : "Fill the window with the terminal (⌘\\)"}
      >
        {#if terminals.maximized}
          <!-- restore glyph -->
          <svg viewBox="0 0 14 14" class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <rect x="2" y="4" width="8" height="8" rx="1"/>
            <path d="M4 4V2h8v8h-2"/>
          </svg>
        {:else}
          <!-- maximize glyph -->
          <svg viewBox="0 0 14 14" class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <rect x="2" y="2" width="10" height="10" rx="1"/>
          </svg>
        {/if}
      </button>

      <button
        type="button"
        class="h-7 px-2 inline-flex items-center rounded-md text-xs text-muted hover:text-foreground hover:bg-secondary"
        onclick={() => terminals.hide()}
        aria-label="Hide terminal panel"
        title="Hide terminal (⌘J)"
      >Hide</button>
    </div>
  </div>

  <!-- Pane area -->
  <div class="flex-1 relative overflow-hidden">
    {#if terminals.tabs.length === 0}
      <div class="h-full">
        <HostPicker onPick={() => (pickerOpen = false)} />
      </div>
    {:else}
      <!-- Unified pane grid. Single mode overlaps all panes in one cell and
           hides non-active ones; split/grid modes lay them out side by side.
           Using one {#each} across every layout prevents tabs from being
           remounted (which would spawn fresh PTYs). -->
      <div class="absolute inset-0 p-0.5 {isSplit ? 'bg-border/40' : ''}" style={grid.style}>
        {#each terminals.tabs as tab, i (tab.id)}
          {@const isActive = tab.id === terminals.activeId}
          {@const span = spanFor(i)}
          <div
            class="relative h-full w-full overflow-hidden rounded-sm {isSplit && isActive ? 'ring-1 ring-ring/50' : ''} {!isSplit && !isActive ? 'invisible pointer-events-none' : ''}"
            style={isSplit
              ? (span > 1 ? `grid-column: span ${span};` : "")
              : "grid-area: 1 / 1 / 2 / 2;"}
          >
            {#if isSplit}
              <div class="absolute top-1 left-2 text-[10px] font-mono text-muted bg-background/80 px-1.5 py-0.5 rounded pointer-events-none z-10">
                {tab.label}
              </div>
            {/if}
            <TerminalTab {tab} active={isSplit || isActive} />
          </div>
        {/each}
      </div>
    {/if}

    {#if pickerOpen && terminals.tabs.length > 0}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="absolute inset-0 bg-black/50 flex items-start justify-center pt-6 z-30" onclick={() => (pickerOpen = false)}>
        <div
          class="bishop-modal border rounded-lg shadow-xl w-[min(520px,90%)] max-h-[calc(100%-2rem)] flex flex-col"
          onclick={(e) => e.stopPropagation()}
          role="presentation"
        >
          <div class="flex items-center justify-between px-3 h-9 border-b border-border">
            <div class="text-xs font-medium">Open new tab</div>
            <button type="button" class="text-muted hover:text-foreground" onclick={() => (pickerOpen = false)} aria-label="Close picker">✕</button>
          </div>
          <div class="flex-1 overflow-hidden">
            <HostPicker onPick={() => (pickerOpen = false)} />
          </div>
        </div>
      </div>
    {/if}
  </div>
</section>

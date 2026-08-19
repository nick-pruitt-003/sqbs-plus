<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { restoreStateCurrent, StateFlags } from "@tauri-apps/plugin-window-state";
  import { load as loadStore } from "@tauri-apps/plugin-store";
  import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
  import { commands } from "$lib/bindings";
  import type { Tournament } from "$lib/bindings";
  import TournamentSetup from "$lib/TournamentSetup.svelte";
  import GameEntry from "$lib/GameEntry.svelte";
  import Reports from "$lib/Reports.svelte";
  import Settings from "$lib/Settings.svelte";

  type Tab = "setup" | "games" | "reports" | "settings";

  let activeTab = $state<Tab>("setup");
  let tournament = $state<Tournament | null>(null);
  let isDirty = $state(false);
  let lastError = $state<string | null>(null);
  let recentFiles = $state<string[]>([]);
  let isDragOver = $state(false);
  let duplicateGames = $state<number[]>([]);
  let duplicateCheckGen = 0;

  /// At most this many duplicate games are listed by name; the rest are counted.
  const MAX_LISTED_DUPES = 15;

  // Tracks the in-flight update_tournament promise so saveFile can await it
  let pendingUpdate: Promise<Tournament> | null = null;

  // ── Recent files store helpers ──────────────────────────────────────────────
  async function loadRecentFiles() {
    try {
      const store = await loadStore("sqbs-plus.json");
      const stored = await store.get<string[]>("recentFiles");
      if (stored) recentFiles = stored;
    } catch {
      // store not yet created or unavailable — that's fine
    }
  }

  async function addRecentFile(path: string) {
    try {
      const store = await loadStore("sqbs-plus.json");
      const updated = [path, ...recentFiles.filter(p => p !== path)].slice(0, 5);
      recentFiles = updated;
      await store.set("recentFiles", updated);
      await store.save();
    } catch {
      // non-critical
    }
  }

  async function removeRecentFile(path: string) {
    try {
      const store = await loadStore("sqbs-plus.json");
      const updated = recentFiles.filter(p => p !== path);
      recentFiles = updated;
      await store.set("recentFiles", updated);
      await store.save();
    } catch {
      // non-critical
    }
  }

  // ── Core file operations ────────────────────────────────────────────────────
  async function openFileByPath(path: string) {
    try {
      tournament = await commands.openFile(path);
      isDirty = false;
      lastError = null;
      await addRecentFile(path);
      await checkForDuplicateGames();
    } catch (e: unknown) {
      lastError = String(e);
      await removeRecentFile(path);
    }
  }

  // Games repeating an earlier game's team pair and round — usually the result
  // of a merge that carried games over twice. Advisory: same-round rematches
  // are legal, so we describe the games and let the user decide.
  // Indexes are only meaningful for the tournament they were computed from, so
  // a result that lands after the tournament changed must be discarded.
  async function checkForDuplicateGames() {
    const gen = ++duplicateCheckGen;
    duplicateGames = [];
    try {
      const found = await commands.findDuplicateGames();
      if (gen === duplicateCheckGen) duplicateGames = found;
    } catch {
      // non-critical — a failed check should never block opening a file
    }
  }

  function describeGame(i: number): string {
    const g = tournament?.games[i];
    if (!g) return `Game ${i + 1}`;
    const nameOf = (ti: number) => tournament?.teams[ti]?.name ?? "?";
    return `Round ${g.round}: ${nameOf(g.team_a.team_index)} vs ${nameOf(g.team_b.team_index)}`;
  }

  async function removeDuplicateGames() {
    if (!tournament || duplicateGames.length === 0) return;
    const previous = tournament;
    const removed = duplicateGames;
    const drop = new Set(removed);
    const games = tournament.games
      .filter((_, i) => !drop.has(i))
      .map((g, i) => ({ ...g, game_index: String(i + 1) }));
    onTournamentChanged({ ...tournament, games });
    duplicateGames = [];
    try {
      // Deleting games is destructive — only let the warning go once the
      // backend has actually accepted the shorter game list.
      if (pendingUpdate) await pendingUpdate;
    } catch {
      tournament = previous;
      duplicateGames = removed;
    }
  }

  async function newTournament() {
    try {
      tournament = await commands.newTournament();
      isDirty = false;
      lastError = null;
      // Stale indexes would point into games that no longer exist.
      duplicateCheckGen++;
      duplicateGames = [];
    } catch (e: unknown) {
      lastError = String(e);
    }
  }

  async function openFile() {
    try {
      const path = await open({
        filters: [{ name: "SQBS Tournament", extensions: ["sqbs", "qzx"] }],
      });
      if (path) {
        await openFileByPath(path);
      }
    } catch (e: unknown) {
      lastError = String(e);
    }
  }

  async function saveFile() {
    try {
      // Ensure any in-flight tournament update is committed before saving
      if (pendingUpdate) await pendingUpdate;
      let path: string | null = await commands.getFilePath();
      if (!path) {
        path = await save({
          filters: [{ name: "SQBS Tournament", extensions: ["sqbs"] }],
        });
      }
      if (path) {
        tournament = await commands.saveFile(path);
        isDirty = false;
        lastError = null;
      }
    } catch (e: unknown) {
      lastError = String(e);
    }
  }

  async function saveAs() {
    try {
      // Ensure any in-flight tournament update is committed before saving
      if (pendingUpdate) await pendingUpdate;
      const path = await save({
        filters: [{ name: "SQBS Tournament", extensions: ["sqbs"] }],
      });
      if (path) {
        tournament = await commands.saveFile(path);
        isDirty = false;
        lastError = null;
      }
    } catch (e: unknown) {
      lastError = String(e);
    }
  }

  function onTournamentChanged(updated: Tournament) {
    tournament = updated;
    isDirty = true;
    pendingUpdate = commands.updateTournament(updated);
    pendingUpdate.catch((e: unknown) => { lastError = String(e); });
  }

  // ── Lifecycle ───────────────────────────────────────────────────────────────
  onMount(() => {
    const cleanups: (() => void)[] = [];

    (async () => {
      // Restore window state (size/position)
      await restoreStateCurrent(StateFlags.ALL);

      // Load recent files
      await loadRecentFiles();

      // Listen for single-instance file open event
      const unlistenSingleInstance = await listen<string>("single-instance-file", async (e) => {
        await openFileByPath(e.payload);
      });
      cleanups.push(unlistenSingleInstance);

      // Listen for deep-link file open events
      const unlistenDeep = await onOpenUrl(async (urls: string[]) => {
        const f = urls.find((p: string) => p.endsWith(".sqbs") || p.endsWith(".qzx"));
        if (f) await openFileByPath(f);
      });
      cleanups.push(unlistenDeep);

      // Check if app was launched to open a specific file
      const currentFiles = await getCurrent();
      if (currentFiles?.length) {
        const f = currentFiles.find((p: string) => p.endsWith(".sqbs") || p.endsWith(".qzx"));
        if (f) await openFileByPath(f);
      }

      // Drag-and-drop file open
      const appWindow = getCurrentWebviewWindow();
      const unlistenDrop = await appWindow.onDragDropEvent(async (event) => {
        if (event.payload.type === "drop") {
          const paths = (event.payload as { type: "drop"; paths: string[] }).paths;
          const file = paths?.find(
            (p: string) => p.endsWith(".sqbs") || p.endsWith(".qzx")
          );
          if (file) await openFileByPath(file);
          isDragOver = false;
        } else if (event.payload.type === "enter") {
          isDragOver = true;
        } else if (event.payload.type === "leave") {
          isDragOver = false;
        }
      });
      cleanups.push(unlistenDrop);
    })();

    return () => {
      cleanups.forEach(fn => fn());
    };
  });

  newTournament();
</script>

<div class="app" class:drag-over={isDragOver}>
  {#if isDragOver}
    <div class="drag-overlay">
      <div class="drag-message">Drop .sqbs or .qzx file to open</div>
    </div>
  {/if}

  <div class="toolbar">
    <button class="tool-btn" onclick={newTournament} title="New tournament">
      <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M5 2h7l4 4v12a1 1 0 01-1 1H5a1 1 0 01-1-1V3a1 1 0 011-1z"/>
        <polyline points="12,2 12,6 16,6"/>
        <line x1="7.5" y1="11" x2="12.5" y2="11"/>
        <line x1="10" y1="8.5" x2="10" y2="13.5"/>
      </svg>
      <span>New</span>
    </button>
    <button class="tool-btn" onclick={openFile} title="Open tournament file">
      <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 8a1 1 0 011-1h3.5l1.5 1.5H16a1 1 0 011 1v6a1 1 0 01-1 1H4a1 1 0 01-1-1V8z"/>
        <path d="M3 8V6a1 1 0 011-1h3"/>
      </svg>
      <span>Open</span>
    </button>
    <button class="tool-btn" onclick={saveFile} title="Save tournament">
      <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4 3h9l3 3v11a1 1 0 01-1 1H4a1 1 0 01-1-1V4a1 1 0 011-1z"/>
        <rect x="6.5" y="3" width="4" height="4.5" rx="0.5"/>
        <rect x="5" y="12" width="10" height="5" rx="0.5"/>
      </svg>
      <span>Save{isDirty ? " ●" : ""}</span>
    </button>
    <button class="tool-btn" onclick={saveAs} title="Save as a new file">
      <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 4h8.5l3 3v9a1 1 0 01-1 1H4a1 1 0 01-1-1V5a1 1 0 011-1z"/>
        <rect x="5.5" y="4" width="4" height="4" rx="0.5"/>
        <rect x="4.5" y="12" width="8" height="4" rx="0.5"/>
        <path d="M15 10l2.5 2.5L15 15M17.5 12.5h-4"/>
      </svg>
      <span>Save As</span>
    </button>
    <div class="toolbar-divider"></div>
    <div class="toolbar-spacer"></div>
    <div class="tournament-name">{tournament?.name || "Untitled Tournament"}</div>
    <div class="toolbar-spacer"></div>
  </div>

  {#if recentFiles.length > 0}
    <div class="recent-files-bar">
      <span class="recent-label">Recent:</span>
      {#each recentFiles as filePath}
        <button
          class="recent-file-btn"
          onclick={() => openFileByPath(filePath)}
          title={filePath}
        >
          {filePath.split(/[/\\]/).at(-1)}
        </button>
      {/each}
    </div>
  {/if}

  <nav class="tab-bar">
    <div class="tab-group">
      <button class="tab-btn" class:active={activeTab === "setup"}    onclick={() => (activeTab = "setup")}>Tournament Setup</button>
      <button class="tab-btn" class:active={activeTab === "games"}    onclick={() => (activeTab = "games")}>Game Entry</button>
      <button class="tab-btn" class:active={activeTab === "reports"}  onclick={() => (activeTab = "reports")}>Reports</button>
      <button class="tab-btn" class:active={activeTab === "settings"} onclick={() => (activeTab = "settings")}>Settings</button>
    </div>
  </nav>

  {#if lastError}
    <div class="error-banner" role="alert">
      <span>{lastError}</span>
      <button class="error-dismiss" onclick={() => (lastError = null)}>&#x2715;</button>
    </div>
  {/if}

  {#if duplicateGames.length > 0}
    <div class="dupe-banner" role="status">
      <div class="dupe-text">
        <b>{duplicateGames.length} possible duplicate game{duplicateGames.length === 1 ? "" : "s"}</b>
        — these match an earlier game (same teams and round) and may be counted twice:
        <ul>
          {#each duplicateGames.slice(0, MAX_LISTED_DUPES) as i}
            <li>{describeGame(i)}</li>
          {/each}
          {#if duplicateGames.length > MAX_LISTED_DUPES}
            <li>…and {duplicateGames.length - MAX_LISTED_DUPES} more.</li>
          {/if}
        </ul>
      </div>
      <div class="dupe-actions">
        <button class="dupe-btn" onclick={removeDuplicateGames}>Remove Duplicates</button>
        <button class="error-dismiss" aria-label="Dismiss duplicate-game warning"
          onclick={() => (duplicateGames = [])}>&#x2715;</button>
      </div>
    </div>
  {/if}

  <main class="content">
    {#if tournament}
      {#if activeTab === "setup"}
        <TournamentSetup {tournament} onChange={onTournamentChanged} />
      {:else if activeTab === "games"}
        <GameEntry {tournament} onChange={onTournamentChanged} />
      {:else if activeTab === "reports"}
        <Reports {tournament} />
      {:else if activeTab === "settings"}
        <Settings {tournament} onChange={onTournamentChanged} />
      {/if}
    {/if}
  </main>
</div>

<style>
  /* ── Design tokens ─────────────────────────────────────────────────────── */
  :global(:root) {
    --bg-app:       #e8e8e8;
    --bg-surface:   #ffffff;
    --bg-raised:    #f5f5f7;
    --bg-sunken:    #ebebeb;
    --bg-toolbar:   linear-gradient(180deg, #d8d8d8 0%, #c4c4c4 100%);
    --bg-tabbar:    linear-gradient(180deg, #d0d0d0 0%, #c0c0c0 100%);
    --border:       rgba(0,0,0,0.14);
    --border-light: rgba(0,0,0,0.07);
    --border-strong:rgba(0,0,0,0.22);
    --text:         #1d1d1f;
    --text-2:       #555;
    --text-3:       #888;
    --accent:       #0071e3;
    --accent-hover: #0077ed;
    --accent-fg:    #fff;
    --tab-active-bg:   #0071e3;
    --tab-active-fg:   #fff;
    --tab-inactive-bg: linear-gradient(180deg, #f0f0f0 0%, #e2e2e2 100%);
    --tab-inactive-fg: #333;
    --shadow-sm:    0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.08);
    --radius:       6px;
    --radius-sm:    4px;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg-app:       #1c1c1e;
      --bg-surface:   #2c2c2e;
      --bg-raised:    #3a3a3c;
      --bg-sunken:    #1c1c1e;
      --bg-toolbar:   linear-gradient(180deg, #3a3a3c 0%, #2e2e30 100%);
      --bg-tabbar:    linear-gradient(180deg, #323234 0%, #28282a 100%);
      --border:       rgba(255,255,255,0.11);
      --border-light: rgba(255,255,255,0.06);
      --border-strong:rgba(255,255,255,0.2);
      --text:         #f2f2f7;
      --text-2:       #aeaeb2;
      --text-3:       #636366;
      --accent:       #2997ff;
      --accent-hover: #409cff;
      --accent-fg:    #fff;
      --tab-active-bg:   #2997ff;
      --tab-active-fg:   #fff;
      --tab-inactive-bg: linear-gradient(180deg, #48484a 0%, #3a3a3c 100%);
      --tab-inactive-fg: #d0d0d5;
      --shadow-sm:    0 1px 3px rgba(0,0,0,0.4), 0 1px 2px rgba(0,0,0,0.3);
    }
  }

  /* ── Global resets & base ──────────────────────────────────────────────── */
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
    font-size: 13px;
    color: var(--text);
    background: var(--bg-app);
    -webkit-font-smoothing: antialiased;
  }

  /* ── Global control styling ────────────────────────────────────────────── */
  :global(input[type="text"]),
  :global(input[type="number"]),
  :global(select) {
    padding: 3px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-family: inherit;
    background: var(--bg-surface);
    color: var(--text);
    appearance: none;
    -webkit-appearance: none;
  }

  :global(select) {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%23888' stroke-width='1.5' fill='none' stroke-linecap='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 6px center;
    padding-right: 22px;
  }

  :global(input:focus),
  :global(select:focus) {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(0, 113, 227, 0.2);
  }

  :global(input:disabled),
  :global(select:disabled) { opacity: 0.38; cursor: not-allowed; }

  :global(input[type="checkbox"]),
  :global(input[type="radio"]) {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    cursor: pointer;
    flex-shrink: 0;
  }

  /* ── App shell ─────────────────────────────────────────────────────────── */
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-app);
    position: relative;
  }

  .app.drag-over {
    outline: 3px solid var(--accent);
    outline-offset: -3px;
  }

  /* ── Drag overlay ──────────────────────────────────────────────────────── */
  .drag-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 113, 227, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    pointer-events: none;
  }

  .drag-message {
    background: var(--bg-surface);
    border: 2px dashed var(--accent);
    border-radius: var(--radius);
    padding: 20px 40px;
    font-size: 15px;
    font-weight: 600;
    color: var(--accent);
  }

  /* ── Toolbar ───────────────────────────────────────────────────────────── */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 10px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--border-strong);
    min-height: 52px;
  }

  .tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 5px 10px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 10px;
    font-weight: 500;
    color: var(--text-2);
    min-width: 54px;
    transition: background 0.1s, color 0.1s;
  }

  .tool-btn svg {
    width: 20px;
    height: 20px;
    color: var(--text-2);
  }

  .tool-btn:hover {
    background: rgba(128,128,128,0.18);
    border-color: var(--border);
    color: var(--text);
  }

  .tool-btn:hover svg { color: var(--text); }

  .tool-btn:active {
    background: rgba(128,128,128,0.28);
  }

  .toolbar-divider {
    width: 1px;
    height: 32px;
    background: var(--border);
    margin: 0 4px;
  }

  .toolbar-spacer { flex: 1; }

  .tournament-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.85;
  }

  /* ── Recent files bar ──────────────────────────────────────────────────── */
  .recent-files-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    background: var(--bg-sunken);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    min-height: 26px;
  }

  .recent-label {
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .recent-file-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 8px;
    font-size: 11px;
    color: var(--accent);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.1s;
  }

  .recent-file-btn:hover {
    background: rgba(0, 113, 227, 0.08);
  }

  /* ── Tab bar ───────────────────────────────────────────────────────────── */
  .tab-bar {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg-tabbar);
    border-bottom: 1px solid var(--border-strong);
  }

  .tab-group {
    display: flex;
    background: rgba(128,128,128,0.2);
    border-radius: 7px;
    padding: 2px;
    gap: 1px;
  }

  .tab-btn {
    padding: 4px 20px;
    border: none;
    border-radius: 5px;
    background: transparent;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    color: var(--tab-inactive-fg);
    min-width: 120px;
    transition: background 0.15s, color 0.15s, box-shadow 0.15s;
    white-space: nowrap;
  }

  .tab-btn:hover:not(.active) {
    background: rgba(128,128,128,0.15);
  }

  .tab-btn.active {
    background: var(--tab-active-bg);
    color: var(--tab-active-fg);
    box-shadow: var(--shadow-sm);
  }

  /* ── Error banner ──────────────────────────────────────────────────────── */
  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 14px;
    background: rgba(198, 40, 40, 0.12);
    border-bottom: 1px solid rgba(198, 40, 40, 0.3);
    font-size: 12px;
    color: #c62828;
  }
  @media (prefers-color-scheme: dark) {
    .error-banner { color: #f07070; background: rgba(198, 40, 40, 0.2); }
  }
  .error-dismiss {
    background: none; border: none; cursor: pointer;
    color: inherit; font-size: 12px; padding: 0 4px; line-height: 1;
  }

  .dupe-banner {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    background: rgba(255, 200, 0, 0.12);
    border-bottom: 1px solid rgba(200, 150, 0, 0.4);
    font-size: 12px;
    color: var(--text);
  }
  .dupe-text ul { margin: 4px 0 0; padding-left: 18px; }
  .dupe-text li { line-height: 1.5; }
  .dupe-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .dupe-btn {
    padding: 3px 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer; font-size: 12px; color: var(--text);
  }
  .dupe-btn:hover { background: var(--bg-sunken); }

  /* ── Content area ──────────────────────────────────────────────────────── */
  .content {
    flex: 1;
    overflow: auto;
    padding: 12px;
    background: var(--bg-app);
  }
</style>

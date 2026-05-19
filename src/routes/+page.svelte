<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import TournamentSetup from "$lib/TournamentSetup.svelte";
  import GameEntry from "$lib/GameEntry.svelte";
  import Reports from "$lib/Reports.svelte";
  import Settings from "$lib/Settings.svelte";

  type Tab = "setup" | "games" | "reports" | "settings";

  let activeTab = $state<Tab>("setup");
  let tournament = $state<any>(null);
  let isDirty = $state(false);

  async function newTournament() {
    tournament = await invoke("new_tournament");
    isDirty = false;
  }

  async function openFile() {
    const path = await open({
      filters: [{ name: "SQBS Tournament", extensions: ["sqbs"] }],
    });
    if (path) {
      tournament = await invoke("open_file", { path });
      isDirty = false;
    }
  }

  async function saveFile() {
    let path: string | null = await invoke("get_file_path");
    if (!path) {
      path = await save({
        filters: [{ name: "SQBS Tournament", extensions: ["sqbs"] }],
      });
    }
    if (path) {
      tournament = await invoke("save_file", { path });
      isDirty = false;
    }
  }

  async function saveAs() {
    const path = await save({
      filters: [{ name: "SQBS Tournament", extensions: ["sqbs"] }],
    });
    if (path) {
      tournament = await invoke("save_file", { path });
      isDirty = false;
    }
  }

  function onTournamentChanged(updated: any) {
    tournament = updated;
    isDirty = true;
    invoke("update_tournament", { tournament: updated });
  }

  newTournament();
</script>

<div class="app">
  <div class="toolbar">
    <button class="tool-btn" onclick={newTournament} title="New tournament">
      <span class="icon">📄</span>
      <span>New</span>
    </button>
    <button class="tool-btn" onclick={openFile} title="Open tournament file">
      <span class="icon">📂</span>
      <span>Open</span>
    </button>
    <button class="tool-btn" onclick={saveFile} title="Save tournament">
      <span class="icon">💾</span>
      <span>Save{isDirty ? " *" : ""}</span>
    </button>
    <button class="tool-btn" onclick={saveAs} title="Save as a new file">
      <span class="icon">📋</span>
      <span>Save As</span>
    </button>
    <div class="toolbar-spacer"></div>
    <div class="tournament-name">{tournament?.name || "Untitled Tournament"}</div>
    <div class="toolbar-spacer"></div>
  </div>

  <nav class="tab-bar">
    <button
      class="tab-btn"
      class:active={activeTab === "setup"}
      onclick={() => (activeTab = "setup")}
    >Tournament Setup</button>
    <button
      class="tab-btn"
      class:active={activeTab === "games"}
      onclick={() => (activeTab = "games")}
    >Game Entry</button>
    <button
      class="tab-btn"
      class:active={activeTab === "reports"}
      onclick={() => (activeTab = "reports")}
    >Reports</button>
    <button
      class="tab-btn"
      class:active={activeTab === "settings"}
      onclick={() => (activeTab = "settings")}
    >Settings</button>
  </nav>

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
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) { font-family: system-ui, -apple-system, sans-serif; font-size: 13px; }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f0f0f0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: linear-gradient(to bottom, #e8e8e8, #d0d0d0);
    border-bottom: 1px solid #b0b0b0;
    min-height: 48px;
  }

  .tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 4px 8px;
    background: none;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
    font-size: 11px;
    color: #333;
    min-width: 52px;
  }
  .tool-btn:hover { background: rgba(0,0,0,0.08); border-color: rgba(0,0,0,0.12); }
  .tool-btn .icon { font-size: 20px; line-height: 1; }

  .toolbar-spacer { flex: 1; }

  .tournament-name {
    font-size: 13px;
    font-weight: 600;
    color: #333;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-bar {
    display: flex;
    justify-content: center;
    gap: 2px;
    padding: 5px 8px;
    background: linear-gradient(to bottom, #e0e0e0, #d4d4d4);
    border-bottom: 1px solid #b8b8b8;
  }

  .tab-btn {
    padding: 4px 18px;
    border: 1px solid #aaa;
    border-radius: 4px;
    background: linear-gradient(to bottom, #f5f5f5, #e0e0e0);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    color: #333;
    min-width: 130px;
  }
  .tab-btn:hover { background: linear-gradient(to bottom, #fff, #eee); }
  .tab-btn.active {
    background: linear-gradient(to bottom, #4a90d9, #2d6db5);
    color: white;
    border-color: #1d5a9a;
  }

  .content {
    flex: 1;
    overflow: auto;
    padding: 12px;
    background: #f5f5f5;
  }
</style>

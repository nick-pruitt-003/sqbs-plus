<script lang="ts">
  let { tournament, onChange } = $props<{ tournament: any; onChange: (t: any) => void }>();

  type SettingsTab = "general" | "reports" | "sorting" | "warnings" | "packets";
  let activeTab = $state<SettingsTab>("general");

  function updateScoring(field: string, value: any) {
    onChange({ ...tournament, scoring: { ...tournament.scoring, [field]: value } });
  }

  function updateReports(field: string, value: any) {
    onChange({ ...tournament, reports: { ...tournament.reports, [field]: value } });
  }

  function update(field: string, value: any) {
    onChange({ ...tournament, [field]: value });
  }

  // Warnings bitmask: W1=bit7, W2=bit6, W3=bit5, W4=bit4, W5=bit3, W6=bit2, W7=bit1
  function getWarn(n: number): boolean {
    const bit = 1 << (8 - n);
    return (tournament.warn_flags & bit) !== 0;
  }
  function setWarn(n: number, val: boolean) {
    const bit = 1 << (8 - n);
    const flags = val ? (tournament.warn_flags | bit) : (tournament.warn_flags & ~bit);
    update("warn_flags", flags & 0xff);
  }

  // Packets: BTreeMap<u32, String> serialized as {"1": "name", ...}
  function packetRounds(): number[] {
    return Object.keys(tournament.packets ?? {}).map(Number).sort((a, b) => a - b);
  }

  function setPacket(round: number, name: string) {
    const packets = { ...(tournament.packets ?? {}), [round]: name };
    update("packets", packets);
  }

  function addPacketRound() {
    const rounds = packetRounds();
    const next = rounds.length > 0 ? Math.max(...rounds) + 1 : 1;
    setPacket(next, "-");
  }

  function removePacketRound(round: number) {
    const packets = { ...(tournament.packets ?? {}) };
    delete packets[round];
    update("packets", packets);
  }

  const SORT_OPTIONS = [
    [1, "Record / PPG"],
    [2, "Record / Head-to-Head / PPG"],
    [3, "Record / Strength of Schedule"],
    [4, "Record / Points per TUH"],
    [5, "Record / Head-to-Head / Points per TUH"],
  ];

  const WARN_LABELS: [number, string, boolean][] = $derived([
    [1, "Same team entered 2x for same game (W-1)", true],
    [2, "Same player entered 2x for same game (W-2)", false],
    [3, "Team score inconsistencies (W-3)", false],
    [4, `Games played by all players in a game exceeds ${tournament.max_players_per_team} (W-4)`, false],
    [5, "Team bonus points per bonus heard is < 0 or > 30 (W-5)", false],
    [6, "Bonus points calculated > 0 but bonus heard is zero (W-6)", false],
    [7, "Negative value entered for toss-ups heard (W-7)", false],
  ]);

  const REPORT_ROWS: [string, string, string][] = [
    ["include_rounds",        "rounds",        "Include Round Report"],
    ["include_standings",     "standings",     "Include Team Standings"],
    ["include_individuals",   "individuals",   "Include Individual Standings"],
    ["include_games",         "games",         "Include Scoreboard"],
    ["include_team_detail",   "team_detail",   "Include Team Detail"],
    ["include_player_detail", "player_detail", "Include Individual Detail"],
    ["include_stat_key",      "stat_key",      "Include Stat Key"],
    ["use_style_sheet",       "style",         "Use Style Sheet"],
  ];
</script>

<div class="settings">
  <div class="tab-bar">
    {#each [["general","General"],["reports","Reports"],["sorting","Sorting"],["warnings","Warnings"],["packets","Packets"]] as [id, label]}
      <button class="tab-btn" class:active={activeTab === id} onclick={() => (activeTab = id as SettingsTab)}>{label}</button>
    {/each}
  </div>

  <div class="tab-content">

    <!-- ── General ── -->
    {#if activeTab === "general"}
      <div class="section">
        <h3>Scoring</h3>
        <div class="field-grid">
          {#each [["Power","0"],["Alt Power","1"],["Normal","2"],["Neg","3"]] as [label, si]}
            {@const i = parseInt(si)}
            <div class="field-row">
              <label class="cb-label">
                <input type="checkbox" checked={tournament.scoring.q_enabled[i]}
                  onchange={(e) => {
                    const en = [...tournament.scoring.q_enabled];
                    en[i] = (e.target as HTMLInputElement).checked;
                    updateScoring("q_enabled", en);
                  }} />
                {label} value
              </label>
              <input type="number"
                value={tournament.scoring.q_values[i]}
                disabled={!tournament.scoring.q_enabled[i]}
                oninput={(e) => {
                  const v = [...tournament.scoring.q_values];
                  v[i] = parseInt((e.target as HTMLInputElement).value) || 0;
                  updateScoring("q_values", v);
                }} />
            </div>
          {/each}

          <div class="field-row">
            <label for="bonus-conversion">Bonus conversion</label>
            <select id="bonus-conversion" value={tournament.scoring.auto_track}
              onchange={(e) => updateScoring("auto_track", parseInt((e.target as HTMLSelectElement).value) || 0)}>
              <option value={0}>Manual</option>
              <option value={1}>Automatic</option>
              <option value={2}>Combo</option>
              <option value={3}>Bounceback</option>
              <option value={4}>Auto with Bouncebacks</option>
            </select>
          </div>

          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox" checked={tournament.track_bonuses}
                onchange={(e) => update("track_bonuses", (e.target as HTMLInputElement).checked)} />
              Track bonuses
            </label>
          </div>
          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox" checked={tournament.scoring.track_power_neg}
                onchange={(e) => updateScoring("track_power_neg", (e.target as HTMLInputElement).checked)} />
              Track powers/negs separately
            </label>
          </div>
          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox" checked={tournament.scoring.track_light_round}
                onchange={(e) => updateScoring("track_light_round", (e.target as HTMLInputElement).checked)} />
              Lightning rounds
            </label>
          </div>
          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox" checked={tournament.scoring.track_tuh}
                onchange={(e) => updateScoring("track_tuh", (e.target as HTMLInputElement).checked)} />
              Track tossups heard
            </label>
          </div>
        </div>
      </div>

    <!-- ── Reports ── -->
    {:else if activeTab === "reports"}
      <div class="section">
        <h3>Report Files</h3>
        <p class="hint">Filenames are appended to the base tournament name when generating HTML reports.</p>
        <div class="field-grid">
          {#each REPORT_ROWS as [enableKey, fileKey, label]}
            <div class="field-row">
              <label class="cb-label" style="min-width: 220px;">
                <input type="checkbox" checked={tournament.reports[enableKey]}
                  onchange={(e) => updateReports(enableKey, (e.target as HTMLInputElement).checked)} />
                {label}
              </label>
              <input type="text" style="width: 180px;"
                value={tournament.reports[fileKey]}
                disabled={!tournament.reports[enableKey]}
                oninput={(e) => updateReports(fileKey, (e.target as HTMLInputElement).value)} />
            </div>
          {/each}
          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox" checked={tournament.reports.british_style}
                onchange={(e) => updateReports("british_style", (e.target as HTMLInputElement).checked)} />
              British Style Reports
            </label>
          </div>
        </div>
      </div>

    <!-- ── Sorting ── -->
    {:else if activeTab === "sorting"}
      <div class="section">
        <h3>Standings Sort Order</h3>
        <div class="field-grid">
          {#each SORT_OPTIONS as [val, label]}
            <div class="field-row">
              <label class="cb-label">
                <input type="radio" name="sort_method" value={val}
                  checked={tournament.sort_method === val}
                  onchange={() => update("sort_method", val)} />
                {label}
              </label>
            </div>
          {/each}
        </div>
      </div>
      <div class="section">
        <h3>Individual Standings</h3>
        <div class="field-grid">
          <div class="field-row">
            <label class="cb-label">
              <input type="checkbox"
                checked={tournament.scoring.sort_by_ppg}
                onchange={(e) => updateScoring("sort_by_ppg", e.currentTarget.checked)} />
              Sort Players by Pts/TUH
            </label>
          </div>
        </div>
      </div>

    <!-- ── Warnings ── -->
    {:else if activeTab === "warnings"}
      <div class="section">
        <h3>Check For…</h3>
        <div class="field-grid">
          {#each WARN_LABELS as [n, label, disabled]}
            <div class="field-row">
              <label class="cb-label" class:grayed={disabled}>
                <input type="checkbox" checked={getWarn(n)} disabled={disabled}
                  onchange={(e) => setWarn(n, (e.target as HTMLInputElement).checked)} />
                {label}
              </label>
            </div>
          {/each}
        </div>
      </div>

    <!-- ── Packets ── -->
    {:else if activeTab === "packets"}
      <div class="section">
        <h3>Packet Names</h3>
        <p class="hint">Assign a packet name to each round. Use "–" for rounds without a named packet.</p>
        <div class="packets-grid">
          {#each packetRounds() as round}
            <div class="packet-row">
              <span class="round-label">Round {round}</span>
              <input type="text" class="packet-input"
                value={tournament.packets[round] ?? "-"}
                oninput={(e) => setPacket(round, (e.target as HTMLInputElement).value)} />
              <button class="rm-btn" aria-label="Remove round" onclick={() => removePacketRound(round)}>✕</button>
            </div>
          {:else}
            <p class="hint">No rounds defined yet. Games must be entered first, or add manually below.</p>
          {/each}
          <button class="add-round-btn" onclick={addPacketRound}>+ Add Round</button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings { display: flex; flex-direction: column; gap: 0; }

  /* ── Sub tab bar ── */
  .tab-bar {
    display: flex; gap: 1px; padding: 6px 10px;
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    border-radius: var(--radius) var(--radius) 0 0;
  }

  .tab-btn {
    padding: 4px 14px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer; font-size: 12px; font-weight: 500;
    color: var(--text-2);
    transition: background 0.12s, color 0.12s;
  }
  .tab-btn:hover:not(.active) {
    background: rgba(128,128,128,0.15);
    color: var(--text);
  }
  .tab-btn.active {
    background: var(--accent);
    color: #fff;
    border-color: transparent;
  }

  /* ── Content card ── */
  .tab-content {
    background: var(--bg-surface);
    border: 1px solid var(--border); border-top: none;
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow-sm);
  }

  .section { padding: 14px 18px; }

  .section h3 {
    font-size: 10px; font-weight: 700;
    color: var(--text-3);
    text-transform: uppercase; letter-spacing: 0.07em;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--border-light);
    padding-bottom: 5px;
  }

  .hint { font-size: 11px; color: var(--text-3); margin-bottom: 10px; line-height: 1.5; }

  .field-grid { display: flex; flex-direction: column; gap: 8px; }
  .field-row { display: flex; align-items: center; gap: 8px; }

  .field-row > label:not(.cb-label) {
    width: 180px; font-size: 12px; color: var(--text-2); flex-shrink: 0;
  }

  .cb-label {
    display: flex; align-items: center; gap: 7px;
    cursor: pointer; font-size: 12px; color: var(--text);
    user-select: none;
  }
  .cb-label.grayed { color: var(--text-3); }

  input[type="number"] { width: 70px; }
  input[type="text"] { width: 200px; }

  /* ── Packets tab ── */
  .packets-grid { display: flex; flex-direction: column; gap: 6px; }
  .packet-row { display: flex; align-items: center; gap: 8px; }
  .round-label { width: 70px; font-size: 12px; color: var(--text-2); }
  .packet-input { width: 200px; }

  .add-round-btn {
    margin-top: 8px; padding: 4px 12px;
    background: var(--accent); color: var(--accent-fg);
    border: none; border-radius: var(--radius-sm);
    cursor: pointer; font-size: 12px; font-weight: 600; align-self: flex-start;
    transition: background 0.1s;
  }
  .add-round-btn:hover { background: var(--accent-hover); }

  .rm-btn {
    background: none; border: none; color: var(--text-3);
    cursor: pointer; font-size: 12px; padding: 2px 5px; border-radius: 3px;
    transition: background 0.1s, color 0.1s;
  }
  .rm-btn:hover { background: rgba(200,0,0,0.12); color: #c00; }
</style>

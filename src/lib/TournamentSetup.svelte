<script lang="ts">
  import { commands } from "$lib/bindings";
  import { clampManualRank } from "$lib/validation";

  let { tournament, onChange } = $props<{ tournament: any; onChange: (t: any) => void }>();

  const Q_LABELS = ["20", "15", "10", "-5"];

  // ── W-8: unusual capitalization in team/player names ──────────────────────
  // Enabled via the low bit of the warnings mask (Settings → Warnings).
  const warn8Enabled = $derived((tournament.warn_flags & 1) !== 0);
  let nameWarning = $state<string | null>(null);

  // Checks are async, so a slower earlier reply must not overwrite a later one.
  let checkSeq = 0;

  $effect(() => {
    if (!warn8Enabled) {
      checkSeq++;          // invalidate anything in flight
      nameWarning = null;
    }
  });

  // The banner reflects the most recent check only, so correcting a flagged
  // name clears it rather than leaving a warning about text that is no longer
  // on screen.
  async function checkName(value: string) {
    const seq = ++checkSeq;
    if (!warn8Enabled) return;
    const trimmed = value.trim();
    if (trimmed === "") {
      nameWarning = null;
      return;
    }
    try {
      const unusual = await commands.checkNameCapitalization(trimmed);
      if (seq !== checkSeq) return;  // superseded
      nameWarning = unusual ? trimmed : null;
    } catch {
      // A failed check is not a naming problem — leave the banner as it was
      // rather than reporting a backend error on a name field.
    }
  }

  function setManualRank(ti: number, raw: string) {
    updateTeamField(ti, "manual_rank", clampManualRank(raw, tournament.teams.length));
  }

  function update(field: string, value: any) {
    onChange({ ...tournament, [field]: value });
  }

  function updateScoring(field: string, value: any) {
    onChange({ ...tournament, scoring: { ...tournament.scoring, [field]: value } });
  }

  function updateQValue(i: number, value: number) {
    const q = [...tournament.scoring.q_values];
    q[i] = value;
    updateScoring("q_values", q);
  }

  function updateQEnabled(i: number, enabled: boolean) {
    const e = [...tournament.scoring.q_enabled];
    e[i] = enabled;
    updateScoring("q_enabled", e);
  }

  function addTeam() {
    const n = tournament.max_players_per_team ?? 4;
    const teams = [...tournament.teams, {
      name: `Team ${tournament.teams.length + 1}`,
      players: Array.from({ length: n }, () => ({ name: "" })),
      division: null,
      exhibition: false,
      manual_rank: 0,
    }];
    onChange({ ...tournament, teams });
  }

  function removeTeam(i: number) {
    // Ranks above the new team count no longer name a placement, and the file
    // parser drops them on reload — so drop them here rather than persist a
    // value the next load won't honor.
    const remaining = tournament.teams.filter((_: any, idx: number) => idx !== i);
    const teams = remaining.map((t: any) => ({
      ...t,
      manual_rank: clampManualRank(t.manual_rank ?? 0, remaining.length),
    }));
    onChange({ ...tournament, teams });
  }

  function updateTeamField(ti: number, field: string, value: any) {
    const teams = tournament.teams.map((t: any, i: number) => i === ti ? { ...t, [field]: value } : t);
    onChange({ ...tournament, teams });
  }

  function updatePlayerName(ti: number, pi: number, name: string) {
    const teams = tournament.teams.map((t: any, i: number) => {
      if (i !== ti) return t;
      const players = t.players.map((p: any, j: number) => j === pi ? { ...p, name } : p);
      return { ...t, players };
    });
    onChange({ ...tournament, teams });
  }

  function setMaxPlayers(val: number) {
    const n = Math.max(1, Math.min(8, val));
    const teams = tournament.teams.map((t: any) => {
      let players = [...t.players];
      while (players.length < n) players.push({ name: `P${players.length + 1}` });
      if (players.length > n) players = players.slice(0, n);
      return { ...t, players };
    });
    onChange({ ...tournament, teams, max_players_per_team: n });
  }

  function addDivision() {
    const divisions = [...(tournament.divisions || []), `Division ${(tournament.divisions?.length ?? 0) + 1}`];
    onChange({ ...tournament, divisions });
  }

  function updateDivisionName(i: number, name: string) {
    const old = tournament.divisions[i];
    const divisions = tournament.divisions.map((d: string, idx: number) => idx === i ? name : d);
    const teams = tournament.teams.map((t: any) => t.division === old ? { ...t, division: name } : t);
    onChange({ ...tournament, divisions, teams });
  }

  function removeDivision(i: number) {
    const removed = tournament.divisions[i];
    const divisions = tournament.divisions.filter((_: string, idx: number) => idx !== i);
    const teams = tournament.teams.map((t: any) => t.division === removed ? { ...t, division: null } : t);
    onChange({ ...tournament, divisions, teams });
  }

  const maxPlayers = $derived(tournament.max_players_per_team ?? 4);
  const playerCols = $derived(Array.from({ length: maxPlayers }, (_: any, i: number) => i));
</script>

<div class="setup">
  <div class="top-row">
    <div class="name-col">
      <label class="field-label" for="tournament-name">Tournament Name</label>
      <input class="name-input" id="tournament-name" type="text"
        value={tournament.name}
        oninput={(e) => update("name", (e.target as HTMLInputElement).value)}
        placeholder="Enter tournament name" />
    </div>

    <div class="options-panel">
      <div class="options-group">
        <div class="group-title">Question Values</div>
        {#each Q_LABELS as _label, i}
          <div class="q-row">
            <input type="checkbox" checked={tournament.scoring.q_enabled[i]}
              onchange={(e) => updateQEnabled(i, (e.target as HTMLInputElement).checked)} />
            <input class="q-val" type="number"
              value={tournament.scoring.q_values[i]}
              disabled={!tournament.scoring.q_enabled[i]}
              oninput={(e) => updateQValue(i, parseInt((e.target as HTMLInputElement).value) || 0)} />
          </div>
        {/each}
      </div>

      <div class="options-group">
        <div class="group-title">Stat Tracking</div>
        <label class="cb-row"><input type="checkbox" checked={tournament.scoring.track_tuh}
          onchange={(e) => updateScoring("track_tuh", (e.target as HTMLInputElement).checked)} />Track Toss-Ups Heard</label>
        <label class="cb-row"><input type="checkbox" checked={tournament.scoring.track_power_neg}
          onchange={(e) => updateScoring("track_power_neg", (e.target as HTMLInputElement).checked)} />Track Power and Neg Stats</label>
        <label class="cb-row"><input type="checkbox" checked={tournament.scoring.track_light_round}
          onchange={(e) => updateScoring("track_light_round", (e.target as HTMLInputElement).checked)} />Track Lightning Round Stats</label>
        <label class="cb-row"><input type="checkbox" checked={tournament.uses_divisions}
          onchange={(e) => update("uses_divisions", (e.target as HTMLInputElement).checked)} />Use Divisions</label>
      </div>

      <div class="options-group">
        <div class="group-title">Bonus Conversion Tracking</div>
        {#each [[1,"Automatic"],[0,"Manual"],[2,"Combo"],[3,"Manual with Bouncebacks"],[4,"Auto with Bouncebacks"]] as [val, label]}
          <label class="cb-row">
            <input type="radio" name="auto_track" value={val}
              checked={tournament.scoring.auto_track === val}
              onchange={() => updateScoring("auto_track", val)} />
            {label}
          </label>
        {/each}
      </div>

      <div class="options-group">
        <div class="group-title">Seats / Side</div>
        <div class="seats-row">
          <input class="seats-input" type="number" min="1" max="8"
            value={tournament.max_players_per_team}
            oninput={(e) => setMaxPlayers(parseInt((e.target as HTMLInputElement).value) || 4)} />
          <span class="seats-hint">(# assigned 1 game each<br>in new Game entry)</span>
        </div>
      </div>
    </div>
  </div>

  {#if tournament.uses_divisions}
    <div class="div-section">
      <div class="section-hdr">
        <span class="section-title">Divisions</span>
        <button class="add-btn" onclick={addDivision}>+ Add Division</button>
      </div>
      <div class="div-chips">
        {#each (tournament.divisions || []) as div, i}
          <div class="div-chip">
            <input type="text" value={div}
              oninput={(e) => updateDivisionName(i, (e.target as HTMLInputElement).value)} />
            <button class="rm-btn" onclick={() => removeDivision(i)}>✕</button>
          </div>
        {:else}
          <span class="empty-hint">No divisions yet</span>
        {/each}
      </div>
    </div>
  {/if}

  {#if nameWarning !== null}
    <div class="warn-banner" role="status">
      <span>Unusual capitalization in "{nameWarning}" — check for a typo. (W-8)</span>
      <button class="rm-btn" aria-label="Dismiss warning" onclick={() => (nameWarning = null)}>✕</button>
    </div>
  {/if}

  <div class="teams-section">
    <div class="section-hdr teams-hdr">
      <span class="section-title">Teams ({tournament.teams.length})</span>
      <button class="add-btn" onclick={addTeam}>+ Add Team</button>
    </div>
    <div class="grid-scroll">
      <table class="team-grid">
        <thead>
          <tr>
            <th class="th-rank" title="Manual final-rank override. Blank = automatic (computed from standings).">Rank</th>
            {#if tournament.uses_divisions}<th class="th-div">Div</th>{/if}
            <th class="th-exh">Exh</th>
            <th class="th-team">Team</th>
            {#each playerCols as pi}
              <th class="th-player">Player {pi + 1}</th>
            {/each}
            <th class="th-del"></th>
          </tr>
        </thead>
        <tbody>
          {#each tournament.teams as team, ti}
            <tr class:exh={team.exhibition}>
              <td class="td-rank">
                <input type="number" class="cell-rank" min="1" max={tournament.teams.length}
                  value={team.manual_rank > 0 ? team.manual_rank : ""}
                  placeholder="—"
                  aria-label={`Final rank override for ${team.name || `team ${ti + 1}`}`}
                  title="Manual final-rank override. Blank = automatic (computed from standings)."
                  oninput={(e) => setManualRank(ti, (e.target as HTMLInputElement).value)} />
              </td>
              {#if tournament.uses_divisions}
                <td class="td-div">
                  <select value={team.division ?? ""}
                    onchange={(e) => updateTeamField(ti, "division", (e.target as HTMLSelectElement).value || null)}>
                    <option value="">—</option>
                    {#each (tournament.divisions || []) as div}
                      <option value={div}>{div}</option>
                    {/each}
                  </select>
                </td>
              {/if}
              <td class="td-exh">
                <input type="checkbox" checked={team.exhibition}
                  onchange={(e) => updateTeamField(ti, "exhibition", (e.target as HTMLInputElement).checked)} />
              </td>
              <td class="td-team">
                <input type="text" class="cell-team" value={team.name}
                  onfocus={(e) => (e.target as HTMLInputElement).select()}
                  onblur={(e) => checkName((e.target as HTMLInputElement).value)}
                  oninput={(e) => updateTeamField(ti, "name", (e.target as HTMLInputElement).value)} />
              </td>
              {#each playerCols as pi}
                <td class="td-player">
                  <input type="text" class="cell-player"
                    value={team.players[pi]?.name ?? ""}
                    placeholder={`Player ${pi + 1}`}
                    onblur={(e) => checkName((e.target as HTMLInputElement).value)}
                    oninput={(e) => updatePlayerName(ti, pi, (e.target as HTMLInputElement).value)} />
                </td>
              {/each}
              <td class="td-del">
                <button class="rm-btn" onclick={() => removeTeam(ti)}>✕</button>
              </td>
            </tr>
          {:else}
            <tr><td colspan="20" class="empty-state">No teams — click "+ Add Team"</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

<style>
  .setup { display: flex; flex-direction: column; gap: 10px; }

  /* ── Top options card ── */
  .top-row {
    display: flex; gap: 16px; align-items: flex-start;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    box-shadow: var(--shadow-sm);
  }

  .name-col { display: flex; flex-direction: column; gap: 5px; min-width: 200px; }

  .field-label {
    font-size: 11px; font-weight: 600;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .name-input { font-size: 13px; padding: 5px 7px; width: 200px; }

  .options-panel { display: flex; flex-wrap: wrap; gap: 18px; flex: 1; }
  .options-group { display: flex; flex-direction: column; gap: 5px; min-width: 145px; }

  .group-title {
    font-size: 10px; font-weight: 700;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border-bottom: 1px solid var(--border-light);
    padding-bottom: 4px;
    margin-bottom: 1px;
    white-space: nowrap;
  }

  .q-row { display: flex; align-items: center; gap: 5px; }
  .q-val { width: 52px; text-align: right; }

  .cb-row {
    display: flex; align-items: center; gap: 6px;
    font-size: 12px; color: var(--text); cursor: pointer; white-space: nowrap;
    user-select: none;
  }

  .seats-row { display: flex; align-items: center; gap: 7px; }
  .seats-input { width: 48px; text-align: center; }
  .seats-hint { font-size: 10px; color: var(--text-3); line-height: 1.4; }

  /* ── Section cards ── */
  .div-section {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 9px 12px;
    box-shadow: var(--shadow-sm);
  }

  .section-hdr {
    display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;
  }

  .section-title {
    font-size: 11px; font-weight: 700;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .div-chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }

  .div-chip {
    display: flex; align-items: center; gap: 3px;
    background: rgba(0, 113, 227, 0.1);
    border: 1px solid rgba(0, 113, 227, 0.3);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
  }

  .div-chip input {
    width: 100px; font-size: 12px;
    border: none !important; background: transparent !important;
    box-shadow: none !important; padding: 0 !important;
    color: var(--text);
  }

  .empty-hint { font-size: 11px; color: var(--text-3); font-style: italic; }

  /* ── Teams section ── */
  .teams-section {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  .teams-hdr {
    padding: 7px 12px;
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    margin-bottom: 0;
  }

  .grid-scroll { overflow-x: auto; }

  .team-grid { width: 100%; border-collapse: collapse; font-size: 12px; min-width: 500px; }

  .team-grid th {
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    padding: 5px 7px;
    text-align: left;
    font-weight: 600;
    font-size: 11px;
    color: var(--text-2);
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .team-grid td {
    border-bottom: 1px solid var(--border-light);
    padding: 3px 5px;
    vertical-align: middle;
  }

  tr:hover td { background: rgba(128,128,128,0.06); }
  tr.exh td { background: rgba(255, 200, 0, 0.07); }

  .th-rank, .td-rank { width: 46px; }
  .cell-rank {
    width: 100%;
    text-align: center;
    background: transparent;
    border-color: transparent !important;
    box-shadow: none !important;
    font-size: 12px;
    color: var(--text-2);
  }
  .cell-rank:focus {
    background: var(--bg-surface) !important;
    border-color: var(--accent) !important;
    box-shadow: 0 0 0 3px rgba(0, 113, 227, 0.2) !important;
  }

  .warn-banner {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    background: rgba(255, 200, 0, 0.12);
    border: 1px solid rgba(200, 150, 0, 0.4);
    border-radius: var(--radius);
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text);
  }

  .th-div { width: 90px; }
  .th-exh, .td-exh { width: 32px; text-align: center; }
  .th-team { min-width: 100px; }
  .th-player { min-width: 90px; }
  .th-del { width: 28px; }

  .cell-team, .cell-player {
    width: 100%;
    background: transparent;
    border-color: transparent !important;
    box-shadow: none !important;
    font-size: 12px;
  }
  .cell-team { font-weight: 600; }
  .cell-team:focus, .cell-player:focus {
    background: var(--bg-surface) !important;
    border-color: var(--accent) !important;
    box-shadow: 0 0 0 3px rgba(0, 113, 227, 0.2) !important;
  }

  /* ── Buttons ── */
  .add-btn {
    padding: 4px 11px;
    background: var(--accent);
    color: var(--accent-fg);
    border: none; border-radius: var(--radius-sm);
    cursor: pointer; font-size: 12px; font-weight: 600;
    transition: background 0.1s;
  }
  .add-btn:hover { background: var(--accent-hover); }

  .rm-btn {
    background: none; border: none;
    color: var(--text-3);
    cursor: pointer; font-size: 12px; padding: 2px 5px; border-radius: 3px;
    line-height: 1;
    transition: background 0.1s, color 0.1s;
  }
  .rm-btn:hover { background: rgba(200,0,0,0.12); color: #c00; }

  .empty-state { padding: 20px; color: var(--text-3); text-align: center; font-style: italic; }
</style>

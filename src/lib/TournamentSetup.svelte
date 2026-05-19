<script lang="ts">
  let { tournament, onChange } = $props<{ tournament: any; onChange: (t: any) => void }>();

  const Q_LABELS = ["20", "15", "10", "-5"];

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
      players: Array.from({ length: n }, (_: any, i: number) => ({ name: `P${i + 1}` })),
      division: null,
      exhibition: false,
    }];
    onChange({ ...tournament, teams });
  }

  function removeTeam(i: number) {
    const teams = tournament.teams.filter((_: any, idx: number) => idx !== i);
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
      <label class="field-label">Tournament Name</label>
      <input class="name-input" type="text"
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

  <div class="teams-section">
    <div class="section-hdr teams-hdr">
      <span class="section-title">Teams ({tournament.teams.length})</span>
      <button class="add-btn" onclick={addTeam}>+ Add Team</button>
    </div>
    <div class="grid-scroll">
      <table class="team-grid">
        <thead>
          <tr>
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
                  oninput={(e) => updateTeamField(ti, "name", (e.target as HTMLInputElement).value)} />
              </td>
              {#each playerCols as pi}
                <td class="td-player">
                  <input type="text" class="cell-player"
                    value={team.players[pi]?.name ?? ""}
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

  .top-row {
    display: flex; gap: 12px; align-items: flex-start;
    background: white; border: 1px solid #ccc; border-radius: 6px; padding: 10px 12px;
  }

  .name-col { display: flex; flex-direction: column; gap: 4px; min-width: 200px; }
  .field-label { font-size: 11px; font-weight: 600; color: #555; }
  .name-input { font-size: 13px; padding: 4px 6px; width: 200px; }

  .options-panel { display: flex; flex-wrap: wrap; gap: 14px; flex: 1; }
  .options-group { display: flex; flex-direction: column; gap: 4px; min-width: 140px; }

  .group-title {
    font-size: 11px; font-weight: 700; color: #444;
    border-bottom: 1px solid #e0e0e0; padding-bottom: 2px; margin-bottom: 2px;
    white-space: nowrap;
  }

  .q-row { display: flex; align-items: center; gap: 4px; }
  .q-val { width: 52px; font-size: 12px; text-align: right; }
  .q-val:disabled { opacity: 0.4; }

  .cb-row {
    display: flex; align-items: center; gap: 5px;
    font-size: 12px; color: #333; cursor: pointer; white-space: nowrap;
  }

  .seats-row { display: flex; align-items: center; gap: 6px; }
  .seats-input { width: 48px; font-size: 13px; text-align: center; }
  .seats-hint { font-size: 10px; color: #888; line-height: 1.3; }

  /* ── Divisions ── */
  .div-section {
    background: white; border: 1px solid #ccc; border-radius: 6px; padding: 8px 12px;
  }
  .section-hdr { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .section-title { font-size: 12px; font-weight: 700; color: #333; }
  .div-chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .div-chip {
    display: flex; align-items: center; gap: 2px;
    background: #e8f0fe; border: 1px solid #a8c0f0; border-radius: 4px; padding: 2px 4px;
  }
  .div-chip input { width: 100px; font-size: 12px; border: none; background: transparent; }
  .div-chip input:focus { outline: 1px solid #4a90d9; border-radius: 2px; background: white; }
  .empty-hint { font-size: 11px; color: #999; font-style: italic; }

  /* ── Teams grid ── */
  .teams-section { background: white; border: 1px solid #ccc; border-radius: 6px; overflow: hidden; }
  .teams-hdr { padding: 6px 10px; background: #e8e8e8; border-bottom: 1px solid #ccc; margin-bottom: 0; }
  .grid-scroll { overflow-x: auto; }

  .team-grid { width: 100%; border-collapse: collapse; font-size: 12px; min-width: 500px; }
  .team-grid th {
    background: #f0f0f0; border-bottom: 2px solid #ccc;
    padding: 4px 6px; text-align: left; font-weight: 600; color: #555; white-space: nowrap;
  }
  .team-grid td { border-bottom: 1px solid #f0f0f0; padding: 2px 4px; vertical-align: middle; }
  tr:hover td { background: #f8f8f8; }
  tr.exh td { background: #fffbe6; }

  .th-div { width: 90px; }
  .th-exh, .td-exh { width: 30px; text-align: center; }
  .th-team { min-width: 100px; }
  .th-player { min-width: 90px; }
  .th-del { width: 28px; }

  .cell-team { width: 100%; font-weight: 600; font-size: 12px; }
  .cell-player { width: 100%; font-size: 12px; }

  input[type="text"], input[type="number"], select {
    padding: 2px 5px; border: 1px solid #ccc; border-radius: 3px;
    font-size: 12px; font-family: inherit;
  }
  input:focus, select:focus { outline: none; border-color: #4a90d9; }

  .add-btn {
    padding: 3px 10px; background: #4a90d9; color: white;
    border: none; border-radius: 4px; cursor: pointer; font-size: 12px; font-weight: 600;
  }
  .add-btn:hover { background: #3a7bc8; }

  .rm-btn {
    background: none; border: none; color: #bbb; cursor: pointer;
    font-size: 11px; padding: 1px 4px; border-radius: 3px;
  }
  .rm-btn:hover { background: #ffdddd; color: #c00; }

  .empty-state { padding: 20px; color: #999; text-align: center; }
</style>

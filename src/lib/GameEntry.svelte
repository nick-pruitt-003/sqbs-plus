<script lang="ts">
  let { tournament, onChange } = $props<{ tournament: any; onChange: (t: any) => void }>();

  // ── State ─────────────────────────────────────────────────────────────────
  let currentIdx = $state<number | null>(null);

  // Working copy of the selected game — edits here, committed on Save
  let draft = $state<any>(null);

  const games: any[] = $derived(tournament.games);
  const total = $derived(games.length);
  const game = $derived(currentIdx !== null ? games[currentIdx] : null);

  const qv: number[] = $derived(tournament.scoring.q_values as number[]);
  const qe: boolean[] = $derived(tournament.scoring.q_enabled as boolean[]);
  const bb: boolean = $derived(tournament.scoring.auto_track >= 3);
  const lightning: boolean = $derived(tournament.scoring.track_light_round);

  // Active question columns (only enabled ones)
  const activeCols = $derived(qv.map((v: number, i: number) => ({ i, v })).filter((_: any, i: number) => qe[i]));

  // ── Navigation ────────────────────────────────────────────────────────────
  function goTo(idx: number) {
    if (idx < 0 || idx >= total) return;
    currentIdx = idx;
    draft = JSON.parse(JSON.stringify(games[idx]));
  }

  function gotoNew() {
    currentIdx = total - 1;
    draft = JSON.parse(JSON.stringify(games[total - 1]));
  }

  // ── New game ──────────────────────────────────────────────────────────────
  let newRound = $state(1);
  let newTeamA = $state(0);
  let newTeamB = $state(1);

  function blankTeamScore(teamIdx: number): any {
    const n = tournament.max_players_per_team ?? 4;
    const player_scores = tournament.teams[teamIdx]?.players
      .slice(0, n)
      .map((_: any, pi: number) => ({ player_index: pi, gp: 1.0, q: [0, 0, 0, 0], points: 0 }));
    return {
      team_index: teamIdx,
      total_points: 0,
      bonus_heard: 0, bonus_points: 0,
      bb_heard: 0, bb_points: 0,
      ot_gets: 0, lightning_points: 0,
      player_scores,
    };
  }

  function addGame() {
    if (tournament.teams.length < 2) return;
    const gameIndex = String(total + 1);
    const newGame = {
      game_index: gameIndex,
      round: newRound,
      forfeit: false,
      overtime: false,
      tossups_heard: 20,
      team_a: blankTeamScore(newTeamA),
      team_b: blankTeamScore(newTeamB),
    };
    const updated = { ...tournament, games: [...tournament.games, newGame] };
    onChange(updated);
    // select the new game after state updates
    setTimeout(() => { currentIdx = updated.games.length - 1; draft = JSON.parse(JSON.stringify(newGame)); }, 0);
  }

  // ── Save / Delete ─────────────────────────────────────────────────────────
  function saveGame() {
    if (currentIdx === null || !draft) return;
    const games = tournament.games.map((g: any, i: number) => i === currentIdx ? draft : g);
    onChange({ ...tournament, games });
  }

  function deleteGame() {
    if (currentIdx === null) return;
    const games = tournament.games.filter((_: any, i: number) => i !== currentIdx);
    // re-sequence game_index
    const reindexed = games.map((g: any, i: number) => ({ ...g, game_index: String(i + 1) }));
    onChange({ ...tournament, games: reindexed });
    const newCurrent = Math.min(currentIdx, reindexed.length - 1);
    currentIdx = reindexed.length > 0 ? newCurrent : null;
    draft = currentIdx !== null ? JSON.parse(JSON.stringify(reindexed[currentIdx])) : null;
  }

  function teamSwap() {
    if (!draft) return;
    draft = { ...draft, team_a: { ...draft.team_b, team_index: draft.team_a.team_index }, team_b: { ...draft.team_a, team_index: draft.team_b.team_index } };
    // swap team indices
    const tmpIdx = draft.team_a.team_index;
    draft = { ...draft,
      team_a: { ...draft.team_a, team_index: draft.team_b.team_index },
      team_b: { ...draft.team_b, team_index: tmpIdx },
    };
  }

  // ── Draft mutations ───────────────────────────────────────────────────────
  function setGameField(field: string, value: any) {
    draft = { ...draft, [field]: value };
  }

  function setSideField(sideKey: string, field: string, value: any) {
    const side = { ...draft[sideKey], [field]: value };
    side.total_points = calcSideTotal(side);
    draft = { ...draft, [sideKey]: side };
  }

  function calcPts(q: number[]): number {
    return q.reduce((s: number, v: number, i: number) => s + v * qv[i], 0);
  }

  function calcSideTotal(side: any): number {
    const tuPts = side.player_scores
      .filter(Boolean)
      .reduce((s: number, p: any) => s + calcPts(p.q), 0);
    return tuPts + side.bonus_points + side.bb_points + side.lightning_points;
  }

  function setPlayerActive(sideKey: string, pi: number, active: boolean) {
    const side = draft[sideKey];
    let scores = [...side.player_scores];
    if (active) {
      if (!scores[pi]) scores[pi] = { player_index: pi, gp: 1.0, q: [0, 0, 0, 0], points: 0 };
    } else {
      scores[pi] = null;
    }
    const updated = { ...side, player_scores: scores };
    updated.total_points = calcSideTotal(updated);
    draft = { ...draft, [sideKey]: updated };
  }

  function setPlayerQ(sideKey: string, pi: number, qi: number, value: number) {
    const side = draft[sideKey];
    const scores = side.player_scores.map((ps: any, i: number) => {
      if (i !== pi || !ps) return ps;
      const q = [...ps.q];
      q[qi] = value;
      return { ...ps, q, points: calcPts(q) };
    });
    const updated = { ...side, player_scores: scores };
    updated.total_points = calcSideTotal(updated);
    draft = { ...draft, [sideKey]: updated };
  }

  function teamName(idx: number): string {
    return tournament.teams[idx]?.name ?? `Team ${idx + 1}`;
  }

  function isActive(sideKey: string, pi: number): boolean {
    return draft?.[sideKey]?.player_scores?.[pi] != null;
  }

  function ps(sideKey: string, pi: number): any {
    return draft?.[sideKey]?.player_scores?.[pi] ?? null;
  }
</script>

<div class="game-entry">
  <!-- ── New game bar ──────────────────────────────────────────────────────── -->
  <div class="new-game-bar">
    <span class="new-label">New Game:</span>
    <label>Rd <input class="rd-input" type="number" min="1" bind:value={newRound} /></label>
    <select bind:value={newTeamA}>
      {#each tournament.teams as t, i}<option value={i}>{t.name}</option>{/each}
    </select>
    <span>vs</span>
    <select bind:value={newTeamB}>
      {#each tournament.teams as t, i}<option value={i}>{t.name}</option>{/each}
    </select>
    <button class="add-btn" onclick={addGame} disabled={tournament.teams.length < 2}>Add Game</button>
  </div>

  <!-- ── Score entry ────────────────────────────────────────────────────────── -->
  {#if draft !== null && currentIdx !== null}
    <div class="sides">
      {#each ["team_a", "team_b"] as sideKey}
        {@const side = draft[sideKey]}
        {@const ti = side.team_index}
        {@const players = tournament.teams[ti]?.players ?? []}

        <div class="side-col">
          <div class="side-header">
            <select value={ti}
              onchange={(e) => {
                const newTi = parseInt((e.target as HTMLSelectElement).value);
                const newSide = { ...side, team_index: newTi };
                draft = { ...draft, [sideKey]: newSide };
              }}>
              {#each tournament.teams as t, i}<option value={i}>{t.name}</option>{/each}
            </select>
            <span class="side-score">{side.total_points}</span>
          </div>

          <table class="player-table">
            <thead>
              <tr>
                <th class="th-gp">GP</th>
                <th class="th-name">Player</th>
                {#each activeCols as col}
                  <th class="th-q">{col.v}</th>
                {/each}
                <th class="th-pts">Pts</th>
              </tr>
            </thead>
            <tbody>
              {#each players as player, pi}
                {@const active = isActive(sideKey, pi)}
                {@const p = ps(sideKey, pi)}
                <tr class:active>
                  <td class="td-gp">
                    <input type="checkbox" checked={active}
                      onchange={(e) => setPlayerActive(sideKey, pi, (e.target as HTMLInputElement).checked)} />
                  </td>
                  <td class="td-name">{player.name}</td>
                  {#if active && p}
                    {#each activeCols as col}
                      <td>
                        <input class="q-input" type="number" min="0" value={p.q[col.i]}
                          oninput={(e) => setPlayerQ(sideKey, pi, col.i,
                            parseInt((e.target as HTMLInputElement).value) || 0)} />
                      </td>
                    {/each}
                    <td class="td-pts">{p.points}</td>
                  {:else}
                    <td colspan={activeCols.length + 1} class="no-play">—</td>
                  {/if}
                </tr>
              {/each}
            </tbody>
          </table>

          {#if tournament.track_bonuses}
            <div class="bonus-row">
              <span class="bonus-lbl">Bonus</span>
              <label>Heard
                <input type="number" min="0" value={side.bonus_heard}
                  oninput={(e) => setSideField(sideKey, "bonus_heard", parseInt((e.target as HTMLInputElement).value) || 0)} />
              </label>
              <label>Points
                <input type="number" min="0" value={side.bonus_points}
                  oninput={(e) => setSideField(sideKey, "bonus_points", parseInt((e.target as HTMLInputElement).value) || 0)} />
              </label>
              <span class="ppb">{side.bonus_heard > 0 ? (side.bonus_points / side.bonus_heard).toFixed(2) : "0.00"} PPB</span>
            </div>
          {/if}

          {#if bb}
            <div class="bonus-row">
              <span class="bonus-lbl">BB</span>
              <label>Heard
                <input type="number" min="0" value={side.bb_heard}
                  oninput={(e) => setSideField(sideKey, "bb_heard", parseInt((e.target as HTMLInputElement).value) || 0)} />
              </label>
              <label>Points
                <input type="number" min="0" value={side.bb_points}
                  oninput={(e) => setSideField(sideKey, "bb_points", parseInt((e.target as HTMLInputElement).value) || 0)} />
              </label>
              <span class="ppb">{side.bb_heard > 0 ? (side.bb_points / side.bb_heard).toFixed(2) : "0.00"} PPBB</span>
            </div>
          {/if}

          {#if lightning}
            <div class="bonus-row">
              <span class="bonus-lbl">Lightning</span>
              <label>Points
                <input type="number" min="0" value={side.lightning_points}
                  oninput={(e) => setSideField(sideKey, "lightning_points", parseInt((e.target as HTMLInputElement).value) || 0)} />
              </label>
            </div>
          {/if}

          <div class="side-total">Total: <b>{side.total_points}</b></div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">Add a game above to begin entering scores.</div>
  {/if}

  <!-- ── Bottom control bar ───────────────────────────────────────────────── -->
  <div class="control-bar">
    <label class="ctrl-field">Round
      <input class="ctrl-num" type="number" min="1"
        value={draft?.round ?? 1}
        oninput={(e) => setGameField("round", parseInt((e.target as HTMLInputElement).value) || 1)}
        disabled={draft === null} />
    </label>
    <label class="ctrl-field">ID
      <input class="ctrl-id" type="text"
        value={draft?.game_index ?? ""}
        oninput={(e) => setGameField("game_index", (e.target as HTMLInputElement).value)}
        disabled={draft === null} />
    </label>
    <label class="ctrl-cb">
      <input type="checkbox" checked={draft?.overtime ?? false}
        onchange={(e) => setGameField("overtime", (e.target as HTMLInputElement).checked)}
        disabled={draft === null} />
      Overtime
    </label>
    <label class="ctrl-cb">
      <input type="checkbox" checked={draft?.forfeit ?? false}
        onchange={(e) => setGameField("forfeit", (e.target as HTMLInputElement).checked)}
        disabled={draft === null} />
      Forfeit
    </label>
    <label class="ctrl-field">TUH
      <input class="ctrl-num" type="number" min="1" max="30"
        value={draft?.tossups_heard ?? 20}
        oninput={(e) => setGameField("tossups_heard", parseInt((e.target as HTMLInputElement).value) || 20)}
        disabled={draft === null} />
    </label>

    <div class="ctrl-spacer"></div>

    <button class="ctrl-btn swap-btn" onclick={teamSwap} disabled={draft === null}>Team Swap</button>

    <div class="goto-group">
      <button class="nav-btn" onclick={() => goTo((currentIdx ?? 0) - 1)} disabled={currentIdx === null || currentIdx <= 0}>◀</button>
      <span class="goto-label">Game</span>
      <input class="goto-input" type="number" min="1" max={total}
        value={currentIdx !== null ? currentIdx + 1 : ""}
        oninput={(e) => { const v = parseInt((e.target as HTMLInputElement).value); if (v >= 1 && v <= total) goTo(v - 1); }}
        disabled={total === 0} />
      <span class="goto-label">of {total}</span>
      <button class="nav-btn" onclick={() => goTo((currentIdx ?? 0) + 1)} disabled={currentIdx === null || currentIdx >= total - 1}>▶</button>
    </div>

    <button class="ctrl-btn save-btn" onclick={saveGame} disabled={draft === null}>Save</button>
    <button class="ctrl-btn delete-btn" onclick={deleteGame} disabled={currentIdx === null}>Delete</button>
  </div>
</div>

<style>
  .game-entry { display: flex; flex-direction: column; gap: 10px; height: 100%; min-height: 0; }

  /* ── New game bar ── */
  .new-game-bar {
    display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
    background: white; border: 1px solid #ccc; border-radius: 6px; padding: 7px 10px;
    font-size: 12px;
  }
  .new-label { font-weight: 700; color: #555; white-space: nowrap; }
  .rd-input { width: 50px; }
  .add-btn {
    padding: 3px 12px; background: #4a90d9; color: white;
    border: none; border-radius: 4px; cursor: pointer; font-weight: 600; font-size: 12px;
  }
  .add-btn:hover:not(:disabled) { background: #3a7bc8; }
  .add-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Sides ── */
  .sides {
    display: grid; grid-template-columns: 1fr 1fr; gap: 14px;
    flex: 1; min-height: 0; overflow-y: auto;
    background: white; border: 1px solid #ccc; border-radius: 6px; padding: 10px 12px;
  }

  .side-col { display: flex; flex-direction: column; gap: 6px; min-width: 0; }

  .side-header {
    display: flex; align-items: center; justify-content: space-between;
    border-bottom: 2px solid #4a90d9; padding-bottom: 4px;
  }
  .side-header select { font-size: 13px; font-weight: 700; border: none; background: transparent; flex: 1; }
  .side-header select:focus { outline: 1px solid #4a90d9; border-radius: 2px; background: white; }
  .side-score { font-size: 16px; font-weight: 700; color: #1a1a2e; }

  /* ── Player table ── */
  .player-table { width: 100%; border-collapse: collapse; font-size: 11px; }
  .player-table th {
    background: #f0f0f0; border-bottom: 1px solid #ddd;
    padding: 3px 4px; text-align: center; font-weight: 600; color: #555;
  }
  .player-table td { padding: 2px 3px; border-bottom: 1px solid #f5f5f5; vertical-align: middle; }
  tr.active td { background: #f8fbff; }

  .th-gp { width: 24px; }
  .th-name { text-align: left; min-width: 60px; }
  .th-q { width: 38px; }
  .th-pts { width: 34px; }
  .td-gp { text-align: center; }
  .td-name { text-align: left; }
  .td-pts { text-align: right; font-weight: 600; }
  .no-play { color: #ccc; text-align: center; }

  .q-input { width: 34px; text-align: center; }

  /* ── Bonus rows ── */
  .bonus-row {
    display: flex; align-items: center; gap: 8px; font-size: 11px;
    padding: 3px 0; border-top: 1px solid #f0f0f0;
  }
  .bonus-lbl { width: 50px; color: #666; font-weight: 700; font-size: 10px; text-transform: uppercase; }
  .bonus-row label { display: flex; align-items: center; gap: 3px; }
  .bonus-row input[type="number"] { width: 50px; }
  .ppb { margin-left: auto; color: #888; font-size: 10px; }

  .side-total {
    text-align: right; font-size: 13px;
    padding: 4px 0; border-top: 2px solid #4a90d9; color: #333;
  }

  /* ── Control bar ── */
  .control-bar {
    display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
    background: white; border: 1px solid #ccc; border-radius: 6px; padding: 6px 10px;
    font-size: 12px;
  }

  .ctrl-field { display: flex; align-items: center; gap: 4px; font-size: 12px; }
  .ctrl-num { width: 50px; }
  .ctrl-id { width: 50px; }
  .ctrl-cb { display: flex; align-items: center; gap: 4px; cursor: pointer; font-size: 12px; }
  .ctrl-spacer { flex: 1; }

  .goto-group { display: flex; align-items: center; gap: 4px; }
  .goto-label { font-size: 12px; color: #555; white-space: nowrap; }
  .goto-input { width: 44px; text-align: center; }
  .nav-btn {
    padding: 2px 6px; background: #f0f0f0; border: 1px solid #ccc;
    border-radius: 3px; cursor: pointer; font-size: 11px;
  }
  .nav-btn:hover:not(:disabled) { background: #e0e0e0; }
  .nav-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .ctrl-btn {
    padding: 4px 14px; border: none; border-radius: 4px;
    cursor: pointer; font-size: 12px; font-weight: 600; font-family: inherit;
  }
  .ctrl-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .save-btn { background: #4a90d9; color: white; }
  .save-btn:hover:not(:disabled) { background: #3a7bc8; }
  .delete-btn { background: #f0f0f0; color: #333; border: 1px solid #ccc; }
  .delete-btn:hover:not(:disabled) { background: #ffdddd; color: #c00; border-color: #c00; }
  .swap-btn { background: #f0f0f0; color: #333; border: 1px solid #ccc; }
  .swap-btn:hover:not(:disabled) { background: #e8e8e8; }

  input[type="text"], input[type="number"], select {
    padding: 2px 5px; border: 1px solid #ccc; border-radius: 3px;
    font-size: 12px; font-family: inherit;
  }
  input:focus, select:focus { outline: none; border-color: #4a90d9; }
  input:disabled, select:disabled { opacity: 0.5; }

  .empty-state {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: #999; font-size: 12px;
    background: white; border: 1px solid #ccc; border-radius: 6px;
  }
</style>

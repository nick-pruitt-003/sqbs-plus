<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  let { tournament } = $props<{ tournament: any }>();

  let generating = $state(false);
  let lastResult = $state<{ ok: boolean; msg: string } | null>(null);

  async function generateReports() {
    const dir = await open({ directory: true, title: "Choose output folder for HTML reports" });
    if (!dir) return;
    generating = true;
    lastResult = null;
    try {
      const files: string[] = await invoke("generate_reports", { dir });
      lastResult = { ok: true, msg: `Generated ${files.length} report files in ${dir}` };
    } catch (e: any) {
      lastResult = { ok: false, msg: String(e) };
    } finally {
      generating = false;
    }
  }

  // ── Stats calculation ──────────────────────────────────────────────────────
  const stats = $derived.by(() => {
    const qv: number[] = tournament.scoring.q_values;
    const qe: boolean[] = tournament.scoring.q_enabled;

    interface TeamRow {
      name: string; index: number; exhibition: boolean;
      w: number; l: number; t: number; games: number;
      pf: number; pa: number;
      tuh: number;
      q: number[]; // per-category tossup counts
      neg: number;  // negs (last q category if negative)
      bhrd: number; bpts: number;
      bbhrd: number; bbpts: number;
    }

    const rows: TeamRow[] = tournament.teams.map((t: any, i: number) => ({
      name: t.name, index: i, exhibition: t.exhibition ?? false,
      w: 0, l: 0, t: 0, games: 0,
      pf: 0, pa: 0, tuh: 0,
      q: [0, 0, 0, 0], neg: 0,
      bhrd: 0, bpts: 0, bbhrd: 0, bbpts: 0,
    }));

    for (const game of tournament.games as any[]) {
      if (game.forfeit) continue;
      const a = rows[game.team_a.team_index];
      const b = rows[game.team_b.team_index];
      if (!a || !b) continue;

      a.pf += game.team_a.total_points; a.pa += game.team_b.total_points;
      b.pf += game.team_b.total_points; b.pa += game.team_a.total_points;
      a.tuh += game.tossups_heard; b.tuh += game.tossups_heard;
      a.games++; b.games++;

      a.bhrd += game.team_a.bonus_heard; a.bpts += game.team_a.bonus_points;
      b.bhrd += game.team_b.bonus_heard; b.bpts += game.team_b.bonus_points;
      a.bbhrd += game.team_a.bb_heard; a.bbpts += game.team_a.bb_points;
      b.bbhrd += game.team_b.bb_heard; b.bbpts += game.team_b.bb_points;

      if (game.team_a.total_points > game.team_b.total_points) { a.w++; b.l++; }
      else if (game.team_b.total_points > game.team_a.total_points) { b.w++; a.l++; }
      else { a.t++; b.t++; }

      // tossup breakdown from player scores
      for (const side of [game.team_a, game.team_b]) {
        const row = rows[side.team_index];
        if (!row) continue;
        for (const ps of side.player_scores) {
          if (!ps) continue;
          for (let qi = 0; qi < 4; qi++) {
            row.q[qi] += ps.q[qi];
          }
        }
      }
    }

    // neg count = last q category if its value is negative
    const negIdx = qv.findLastIndex((v: number, i: number) => qe[i] && v < 0);

    return rows
      .filter((r: TeamRow) => r.games > 0 || tournament.teams.length < 4)
      .map((r: TeamRow) => {
        const pct = r.games ? (r.w + r.t * 0.5) / r.games : 0;
        const ppg = r.games ? r.pf / r.games : 0;
        const papg = r.games ? r.pa / r.games : 0;
        const mrg = ppg - papg;
        const tu_pts = r.q.reduce((s: number, v: number, i: number) => s + v * qv[i], 0);
        const ppth = r.tuh ? r.pf / r.tuh : 0;
        const pn = r.q[0]; // powers (first enabled high category)
        const neg = negIdx >= 0 ? r.q[negIdx] : 0;
        const gn = neg > 0 ? pn / neg : 0;
        const ppb = r.bhrd ? r.bpts / r.bhrd : 0;
        const ppbb = r.bbhrd ? r.bbpts / r.bbhrd : 0;
        return { ...r, pct, ppg, papg, mrg, ppth, tu_pts, pn, neg, gn, ppb, ppbb };
      })
      .sort((a: any, b: any) => {
        if (b.pct !== a.pct) return b.pct - a.pct;
        if (tournament.sort_method === 4 || tournament.sort_method === 5) return b.ppth - a.ppth;
        return b.ppg - a.ppg;
      });
  });

  const activeQ = $derived(
    (tournament.scoring.q_values as number[])
      .map((v: number, i: number) => ({ v, i }))
      .filter((_: any, i: number) => (tournament.scoring.q_enabled as boolean[])[i])
  );
</script>

<div class="reports">
  <div class="report-section">
    <h3>Generate HTML Reports</h3>
    <div class="generate-area">
      <button class="generate-btn" onclick={generateReports} disabled={generating}>
        {generating ? "Generating…" : "Generate Reports…"}
      </button>
      <span class="hint">Outputs HTML report files (standings, individuals, scoreboard, team detail, individual detail, rounds, stat key)</span>
    </div>
    {#if lastResult}
      <div class="result" class:error={!lastResult.ok}>{lastResult.msg}</div>
    {/if}
  </div>

  <div class="report-section">
    <h3>Team Standings</h3>
    {#if stats.length === 0}
      <div class="empty-state">No games have been played yet.</div>
    {:else}
      <div class="table-scroll">
        <table class="standings-table">
          <thead>
            <tr>
              <th>#</th>
              <th class="th-team">Team</th>
              <th>W</th><th>L</th><th>T</th>
              <th>Pct</th>
              <th>PPG</th>
              <th>PAPG</th>
              <th>Mrg</th>
              {#each activeQ as col}
                <th>{col.v}</th>
              {/each}
              {#if tournament.scoring.track_tuh}<th>TUH</th>{/if}
              {#if tournament.scoring.track_tuh}<th>P/TU</th>{/if}
              {#if tournament.scoring.track_power_neg}<th>P/N</th><th>G/N</th>{/if}
              {#if tournament.track_bonuses}
                <th>BHrd</th><th>BPts</th><th>P/B</th>
              {/if}
              {#if tournament.scoring.auto_track >= 3}
                <th>BBHrd</th><th>BBPts</th><th>P/BB</th>
              {/if}
            </tr>
          </thead>
          <tbody>
            {#each stats as row, i}
              <tr class:even={i % 2 === 1} class:exh={row.exhibition}>
                <td class="td-rank">{row.games > 0 ? i + 1 : "—"}</td>
                <td class="td-team">{row.name}</td>
                <td>{row.w}</td><td>{row.l}</td><td>{row.t}</td>
                <td>{row.pct.toFixed(3)}</td>
                <td>{row.ppg.toFixed(1)}</td>
                <td>{row.papg.toFixed(1)}</td>
                <td class:pos={row.mrg > 0} class:neg={row.mrg < 0}>{row.mrg.toFixed(1)}</td>
                {#each activeQ as col}
                  <td>{row.q[col.i]}</td>
                {/each}
                {#if tournament.scoring.track_tuh}<td>{row.tuh}</td>{/if}
                {#if tournament.scoring.track_tuh}<td>{row.ppth.toFixed(2)}</td>{/if}
                {#if tournament.scoring.track_power_neg}
                  <td>{row.pn}/{row.neg}</td>
                  <td>{row.gn.toFixed(2)}</td>
                {/if}
                {#if tournament.track_bonuses}
                  <td>{row.bhrd}</td>
                  <td>{row.bpts}</td>
                  <td>{row.ppb.toFixed(2)}</td>
                {/if}
                {#if tournament.scoring.auto_track >= 3}
                  <td>{row.bbhrd}</td>
                  <td>{row.bbpts}</td>
                  <td>{row.ppbb.toFixed(2)}</td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>

  <div class="report-section">
    <h3>Game Results</h3>
    {#if tournament.games.length === 0}
      <div class="empty-state">No games recorded.</div>
    {:else}
      <table class="standings-table">
        <thead>
          <tr>
            <th>Rd</th>
            <th class="th-team">Winner</th>
            <th>Score</th>
            <th class="th-team">Loser</th>
            {#if tournament.scoring.track_tuh}<th>TUH</th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each [...tournament.games].sort((a: any, b: any) => a.round - b.round || parseInt(a.game_index) - parseInt(b.game_index)) as g, i}
            {@const aName = tournament.teams[g.team_a.team_index]?.name ?? "?"}
            {@const bName = tournament.teams[g.team_b.team_index]?.name ?? "?"}
            {@const aWon = g.team_a.total_points >= g.team_b.total_points}
            <tr class:even={i % 2 === 1} class:forfeit={g.forfeit}>
              <td>{g.round}</td>
              <td class="td-team">{aWon ? aName : bName}</td>
              <td class="td-score">
                {#if g.forfeit}
                  Forfeit
                {:else}
                  {Math.max(g.team_a.total_points, g.team_b.total_points)}–{Math.min(g.team_a.total_points, g.team_b.total_points)}
                {/if}
              </td>
              <td class="td-team">{aWon ? bName : aName}</td>
              {#if tournament.scoring.track_tuh}<td>{g.tossups_heard}</td>{/if}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .reports { display: flex; flex-direction: column; gap: 16px; }

  .report-section {
    background: white; border: 1px solid #ccc; border-radius: 6px; overflow: hidden;
  }
  .report-section h3 {
    padding: 8px 12px; background: #e8e8e8; border-bottom: 1px solid #ccc;
    font-size: 12px; font-weight: 600;
  }

  .generate-area { padding: 12px; display: flex; align-items: center; gap: 12px; }
  .generate-btn {
    padding: 6px 16px; background: #4a90d9; color: white;
    border: none; border-radius: 5px; cursor: pointer; font-size: 12px;
    font-weight: 600; white-space: nowrap;
  }
  .generate-btn:hover:not(:disabled) { background: #3a7bc8; }
  .generate-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .hint { font-size: 11px; color: #888; }

  .result {
    margin: 0 12px 12px; padding: 6px 10px; border-radius: 4px; font-size: 11px;
    background: #e8f5e9; color: #2e7d32; border: 1px solid #c8e6c9;
  }
  .result.error { background: #ffebee; color: #c62828; border-color: #ffcdd2; }

  .table-scroll { overflow-x: auto; }

  .standings-table { width: 100%; border-collapse: collapse; font-size: 11px; white-space: nowrap; }
  .standings-table th {
    padding: 4px 7px; text-align: right; background: #f0f0f0;
    border-bottom: 1px solid #ddd; font-weight: 600; color: #444;
  }
  .standings-table td { padding: 3px 7px; border-bottom: 1px solid #f0f0f0; text-align: right; }
  tr.even td { background: #fafafa; }
  tr.exh td { background: #fffbe6; }
  tr.forfeit td { color: #999; }

  .th-team, .td-team { text-align: left; }
  .td-rank { color: #666; width: 24px; }
  .td-score { font-weight: 600; text-align: center; }
  td.pos { color: #2a7d2a; }
  td.neg { color: #c62828; }

  .empty-state { padding: 20px; color: #999; text-align: center; }
</style>

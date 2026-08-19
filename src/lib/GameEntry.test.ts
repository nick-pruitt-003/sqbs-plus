import { describe, it, expect, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import GameEntry from "./GameEntry.svelte";

// Minimal two-team, one-game tournament with a single scoring player a side.
function fixture() {
  const side = (team_index: number) => ({
    team_index,
    total_points: 0,
    bonus_heard: 0, bonus_points: 0,
    bb_heard: 0, bb_points: 0,
    ot_gets: 0, lightning_points: 0,
    player_scores: [{ player_index: 0, gp: 1, q: [0, 0, 0, 0], points: 0 }],
  });
  return {
    name: "Test",
    teams: [
      { name: "Alpha", players: [{ name: "Alice" }], division: null, exhibition: false, manual_rank: 0 },
      { name: "Beta", players: [{ name: "Bob" }], division: null, exhibition: false, manual_rank: 0 },
    ],
    games: [{
      game_index: "1", round: 1, tossups_heard: 20, overtime: false, forfeit: false,
      team_a: side(0), team_b: side(1),
    }],
    scoring: {
      q_values: [20, 15, 10, -5], q_enabled: [false, true, true, false],
      track_power_neg: false, track_light_round: false, track_tuh: true,
      auto_track: 1, sort_by_ppg: false,
    },
    reports: {},
    uses_divisions: false, divisions: [], track_bonuses: false,
    max_players_per_team: 1, sort_method: 1, warn_flags: 0, packets: {},
  } as any;
}

/** Select the first game so the entry form is populated. */
async function selectFirstGame(container: HTMLElement) {
  const goto = container.querySelector<HTMLInputElement>(".goto-input");
  expect(goto).not.toBeNull();
  goto!.value = "1";
  goto!.dispatchEvent(new Event("input", { bubbles: true }));
  await Promise.resolve();
}

describe("GameEntry games-played entry", () => {
  it("commits a focused GP edit when the component unmounts", async () => {
    const onChange = vi.fn();
    const { container } = render(GameEntry, { tournament: fixture(), onChange });
    await selectFirstGame(container);

    const gp = container.querySelector<HTMLInputElement>(".gp-input");
    expect(gp, "GP input should render for an active player").not.toBeNull();

    // Type into the field and leave it focused — no blur, as when switching
    // tabs by keyboard or when the app closes the view programmatically.
    gp!.focus();
    gp!.value = "11/23";
    gp!.dispatchEvent(new Event("input", { bubbles: true }));
    expect(document.activeElement).toBe(gp);
    await Promise.resolve();

    cleanup(); // unmounts, running onDestroy

    expect(onChange).toHaveBeenCalled();
    const saved = onChange.mock.calls.at(-1)![0];
    expect(saved.games[0].team_a.player_scores[0].gp).toBeCloseTo(11 / 23, 6);
  });

  it("drops in-progress GP text when the side switches to another team", async () => {
    const { container } = render(GameEntry, { tournament: fixture(), onChange: vi.fn() });
    await selectFirstGame(container);

    const gp = container.querySelector<HTMLInputElement>(".gp-input")!;
    gp.value = "11/";  // mid-typing, parses to 0
    gp.dispatchEvent(new Event("input", { bubbles: true }));
    await Promise.resolve();

    // Changing the team on this side rebuilds its player scores at gp 1.
    const teamSelect = container.querySelector<HTMLSelectElement>(".side-header select")!;
    teamSelect.value = "1";
    teamSelect.dispatchEvent(new Event("change", { bubbles: true }));
    await Promise.resolve();

    const after = container.querySelector<HTMLInputElement>(".gp-input");
    // Must show the new record's value, not the text typed for the old one.
    expect(after?.value).not.toBe("11/");
  });

  it("leaves the stored value alone when a GP field is blurred unedited", async () => {
    const t = fixture();
    // A value with more precision than the field displays.
    t.games[0].team_a.player_scores[0].gp = 11 / 23;
    const onChange = vi.fn();
    const { container } = render(GameEntry, { tournament: t, onChange });
    await selectFirstGame(container);

    const gp = container.querySelector<HTMLInputElement>(".gp-input");
    gp!.focus();
    gp!.dispatchEvent(new FocusEvent("blur"));
    cleanup();

    // Either nothing was saved, or what was saved kept full precision — the
    // displayed rounding must not be written back over the stored value.
    const saved = onChange.mock.calls.at(-1)?.[0];
    if (saved) {
      expect(saved.games[0].team_a.player_scores[0].gp).toBeCloseTo(11 / 23, 10);
    }
  });
});

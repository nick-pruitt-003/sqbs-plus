/**
 * Input parsing shared by the entry screens.
 *
 * These deliberately mirror the Rust side (`parse_games_played` in
 * sqbs_format.rs, and the manual-rank bounds check in the parser) so a value
 * typed in the UI and the same value read back from a file agree.
 */

/**
 * Whole-string numeric match, with an optional exponent so we accept the same
 * spellings Rust's `f32::from_str` does. `parseFloat` alone would read "11abc"
 * as 11.
 */
const NUMERIC = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/;

/** The number `s` denotes, or null if it isn't a complete finite number. */
export function toNumber(s: string): number | null {
  const trimmed = s.trim();
  if (!NUMERIC.test(trimmed)) return null;
  const val = parseFloat(trimmed);
  return Number.isFinite(val) ? val : null;
}

/**
 * Games played: a decimal (`0.5`) or a slash fraction (`11/23` = in for 11 of
 * 23 tossups), matching the NAQT guide and Mac SQBS entry rules. Anything
 * unparseable, non-finite, or divided by zero is 0.
 */
export function normalizeGp(raw: string): number {
  const slash = raw.indexOf("/");
  if (slash !== -1) {
    const num = toNumber(raw.slice(0, slash));
    const den = toNumber(raw.slice(slash + 1));
    if (num === null || den === null || den === 0) return 0;
    const val = num / den;
    return Number.isFinite(val) ? val : 0;
  }
  return toNumber(raw) ?? 0;
}

/**
 * A manual final-rank override, or 0 for "automatic". Ranks name a placement,
 * so anything outside 1..=teamCount is meaningless — the file parser drops
 * such values too, and a fractional or partly-numeric entry is not a rank.
 */
export function clampManualRank(raw: string | number, teamCount: number): number {
  const value = typeof raw === "number" ? raw : Number(raw.trim());
  if (!Number.isInteger(value) || value <= 0) return 0;
  return Math.min(value, Math.max(0, teamCount));
}

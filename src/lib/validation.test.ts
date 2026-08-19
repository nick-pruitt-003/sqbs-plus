import { describe, it, expect } from "vitest";
import { toNumber, normalizeGp, clampManualRank } from "./validation";

describe("toNumber", () => {
  it("accepts complete numbers", () => {
    expect(toNumber("1")).toBe(1);
    expect(toNumber(" 0.5 ")).toBe(0.5);
    expect(toNumber(".5")).toBe(0.5);
    expect(toNumber("-2")).toBe(-2);
    expect(toNumber("1e-3")).toBe(0.001);
    expect(toNumber("2E+2")).toBe(200);
  });

  it("rejects partial or non-numeric input", () => {
    // parseFloat would read these as 11, 1 and 0 respectively.
    expect(toNumber("11abc")).toBeNull();
    expect(toNumber("1/2")).toBeNull();
    expect(toNumber("junk")).toBeNull();
    expect(toNumber("")).toBeNull();
    expect(toNumber("  ")).toBeNull();
  });

  it("rejects the non-finite spellings Rust's parser would take", () => {
    for (const s of ["inf", "-inf", "Infinity", "NaN"]) {
      expect(toNumber(s)).toBeNull();
    }
  });
});

describe("normalizeGp", () => {
  it("reads decimals", () => {
    expect(normalizeGp("1")).toBe(1);
    expect(normalizeGp("0.75")).toBe(0.75);
  });

  it("reads slash fractions", () => {
    expect(normalizeGp("1/2")).toBe(0.5);
    expect(normalizeGp(" 11 / 23 ")).toBeCloseTo(11 / 23, 10);
  });

  it("is 0 for unusable input", () => {
    expect(normalizeGp("1/0")).toBe(0);
    expect(normalizeGp("junk")).toBe(0);
    expect(normalizeGp("11abc")).toBe(0);
    expect(normalizeGp("")).toBe(0);
    expect(normalizeGp("1e400")).toBe(0);
    expect(normalizeGp("NaN")).toBe(0);
  });

  it("never yields a non-finite value", () => {
    for (const s of ["1e400/1e-400", "inf/1", "1/NaN", "-inf"]) {
      expect(Number.isFinite(normalizeGp(s))).toBe(true);
    }
  });

  // The Rust parser (parse_games_played in sqbs_format.rs) must agree, or a
  // value typed here and the same value read back from a file would differ.
  it("agrees with the Rust parser's cases", () => {
    const cases: [string, number][] = [
      ["11/23", 11 / 23],
      ["1/2", 0.5],
      ["0.75", 0.75],
      ["1", 1],
      ["1/0", 0],
      ["junk", 0],
    ];
    for (const [input, expected] of cases) {
      expect(normalizeGp(input)).toBeCloseTo(expected, 6);
    }
  });
});

describe("clampManualRank", () => {
  it("keeps ranks within the field", () => {
    expect(clampManualRank("1", 8)).toBe(1);
    expect(clampManualRank("8", 8)).toBe(8);
    expect(clampManualRank(3, 8)).toBe(3);
  });

  it("clamps ranks above the team count", () => {
    expect(clampManualRank("99", 8)).toBe(8);
    expect(clampManualRank(12, 4)).toBe(4);
  });

  it("treats anything that isn't a whole placement as automatic", () => {
    expect(clampManualRank("", 8)).toBe(0);
    expect(clampManualRank("0", 8)).toBe(0);
    expect(clampManualRank("-3", 8)).toBe(0);
    expect(clampManualRank("3.5", 8)).toBe(0);
    expect(clampManualRank("3abc", 8)).toBe(0);
    expect(clampManualRank("abc", 8)).toBe(0);
  });

  it("collapses to automatic when there are no teams", () => {
    expect(clampManualRank("1", 0)).toBe(0);
  });
});

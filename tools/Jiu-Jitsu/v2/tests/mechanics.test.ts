import { describe, expect, it } from "vitest";
import { legPoints, momentArm, supportMargin, movementTarget } from "../src/anatomy/mechanics";

describe("同じ力でも、力の向きと距離で回そうとする作用が変わる", () => {
  it("水平の腕で距離が倍なら作用も倍、垂直なら回転作用は0", () => {
    expect(momentArm(20, 0)).toBeCloseTo(20);
    expect(momentArm(40, 0)).toBeCloseTo(40);
    expect(momentArm(40, 60)).toBeCloseTo(20);
    expect(momentArm(40, 90)).toBeCloseTo(0);
  });
});

describe("動かして達成する課題", () => {
  it("腕の距離を近づけても、傾きを変えても、力の線を肘へ近づけられる", () => {
    expect(movementTarget("lever", [20, 0])).toBe(false);
    expect(movementTarget("lever", [10, 0])).toBe(true);
    expect(movementTarget("lever", [20, 60])).toBe(true);
  });
  it("重心を戻す・支えを広げる、両方の動きで余裕を作れる", () => {
    expect(movementTarget("base", [20, 20])).toBe(false);
    expect(movementTarget("base", [20, 0])).toBe(true);
    expect(movementTarget("base", [60, 20])).toBe(true);
  });
  it("膝だけ曲げても枠に届かず、股関節と膝を組み合わせると収まる", () => {
    expect(movementTarget("leg", [0, 0])).toBe(false);
    expect(movementTarget("leg", [0, 90])).toBe(false);
    expect(movementTarget("leg", [90, 0])).toBe(false);
    expect(movementTarget("leg", [90, 90])).toBe(true);
  });
});

describe("重心投影と支持範囲", () => {
  it("内側・境界・外側を区別し、左右対称に判定する", () => {
    expect(supportMargin(40, 0)).toBe(20);
    expect(supportMargin(40, 20)).toBe(0);
    expect(supportMargin(40, 30)).toBe(-10);
    expect(supportMargin(40, -30)).toBe(-10);
    expect(supportMargin(80, 30)).toBe(10);
  });
});

describe("股関節と膝を別々に動かす", () => {
  it("膝だけ曲げても膝頭の位置は変わらず、足先の位置が変わる", () => {
    const straight = legPoints(0, 0);
    const bent = legPoints(0, 90);
    expect(straight.knee).toEqual({ x: 40, y: 0 });
    expect(straight.ankle).toEqual({ x: 80, y: 0 });
    expect(bent.knee).toEqual(straight.knee);
    expect(bent.ankle.x).toBeCloseTo(40);
    expect(bent.ankle.y).toBeCloseTo(-40);
  });
  it("股関節を曲げると膝の位置が変わり、骨の長さは変わらない", () => {
    const points = legPoints(90, 90);
    expect(points.knee.x).toBeCloseTo(0);
    expect(points.knee.y).toBeCloseTo(40);
    expect(points.ankle.x).toBeCloseTo(40);
    expect(points.ankle.y).toBeCloseTo(40);
    for (const hip of [0, 30, 60, 110]) for (const knee of [0, 40, 90, 130]) {
      const p = legPoints(hip, knee);
      expect(Math.hypot(p.knee.x, p.knee.y)).toBeCloseTo(40);
      expect(Math.hypot(p.ankle.x - p.knee.x, p.ankle.y - p.knee.y)).toBeCloseTo(40);
    }
  });
});

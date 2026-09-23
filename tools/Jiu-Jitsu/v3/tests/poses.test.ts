import { describe, expect, it } from "vitest";
import { clampJointEuler, deg, jointLimitViolations } from "../src/anatomy/joints";
import type { RigJointName } from "../src/anatomy/types";
import { POSES, type PoseName } from "../src/render/poses";
import { SCENARIOS } from "../src/content/scenarios";
import type { Stage } from "../src/content/types";

const POSE_ENTRIES = Object.entries(POSES) as [PoseName, (typeof POSES)[PoseName]][];

describe("全ポーズの可動域", () => {
  it.each(POSE_ENTRIES)("%s の全関節が rigRangeDeg 内", (name, pose) => {
    const violations: string[] = [];
    for (const [rig, euler] of Object.entries(pose.joints ?? {})) {
      violations.push(
        ...jointLimitViolations(name, rig as RigJointName, euler as [number, number, number]),
      );
    }
    expect(violations).toEqual([]);
  });
});

describe("scenario が参照するポーズの sanity", () => {
  const stages: { where: string; stage: Stage }[] = SCENARIOS.flatMap((s) => [
    { where: `${s.id}:setup`, stage: s.setup },
    { where: `${s.id}:attack`, stage: s.attack },
    ...s.opponentActions.map((a) => ({ where: `${s.id}:action:${a.id}`, stage: a.attack })),
    ...s.options.map((o, i) => ({ where: `${s.id}:option[${i}].result`, stage: o.result })),
  ]);

  it("red/blue のポーズ名が役割と一致し、同一ポーズを二人で共有しない", () => {
    for (const { where, stage } of stages) {
      expect(POSES[stage.red], where).toBeDefined();
      expect(POSES[stage.blue], where).toBeDefined();
      expect(stage.red, where).toMatch(/^(red|standing)/);
      expect(stage.blue, where).toMatch(/^(blue|standing)/);
      expect(stage.red, where).not.toBe(stage.blue);
    }
  });

  it("badge が空文字でない", () => {
    for (const { where, stage } of stages) {
      expect(stage.badge.length, where).toBeGreaterThan(0);
    }
  });
});

describe("clampJointEuler", () => {
  it("範囲外入力を rigRangeDeg に丸める (neck x は ±35°)", () => {
    const [x, y, z] = clampJointEuler("neck", [deg(90), deg(0), deg(-80)]);
    expect(x).toBeCloseTo(deg(35), 10);
    expect(y).toBeCloseTo(0, 10);
    expect(z).toBeCloseTo(deg(-30), 10);
  });

  it("膝 (shin) は負方向屈曲 (逆関節) を 0 に丸める", () => {
    const [x] = clampJointEuler("shinL", [deg(-40), 0, 0]);
    expect(x).toBeCloseTo(0, 10);
    const [x2] = clampJointEuler("shinR", [deg(170), 0, 0]);
    expect(x2).toBeCloseTo(deg(150), 10);
  });

  it("範囲内入力は変えない", () => {
    const input: [number, number, number] = [deg(20), deg(-10), deg(5)];
    const out = clampJointEuler("neck", input);
    expect(out[0]).toBeCloseTo(input[0], 10);
    expect(out[1]).toBeCloseTo(input[1], 10);
    expect(out[2]).toBeCloseTo(input[2], 10);
  });

  it("解剖スペック対象外の関節 (chest 等) は素通し", () => {
    const input: [number, number, number] = [deg(400), deg(-300), deg(99)];
    expect(clampJointEuler("chest", input)).toEqual(input);
  });
});

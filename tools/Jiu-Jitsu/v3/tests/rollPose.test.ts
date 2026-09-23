import { describe, expect, it } from "vitest";
import { SCENARIOS, scenarioById } from "../src/content/scenarios";
import { rollPair } from "../src/render/rollPose";
import { BODY, Mannequin } from "../src/render/mannequin";
import { Mesh, Vector3 } from "three";

const stages = SCENARIOS.flatMap((s) => [s.setup, s.attack, ...s.opponentActions.map((a) => a.attack), ...s.options.map((o) => o.result)].map((stage, index) => ({ name: `${s.id}/${index}`, stage })));

describe("応用ロールの人形", () => {
  it("マウントの開始・初動・回答後でも両膝が床につく", () => {
    const scenario = scenarioById("mount-escape");
    for (const stage of [scenario.setup, scenario.attack, scenario.options.find((choice) => choice.result.red === "redMountArmbar")!.result]) {
      const pair = rollPair(stage);
      expect(pair.red.kneeL.y).toBeCloseTo(0.073, 3);
      expect(pair.red.kneeR.y).toBeCloseTo(0.073, 3);
    }
  });

  it.each(stages)("$name: 支持点・骨長・床と身体の交差", ({ stage }) => {
    const pair = rollPair(stage);
    for (const s of pair.supports) expect(pair[s.actor][s.joint].y, `${s.actor}/${s.joint}`).toBeCloseTo(s.height, 6);
    for (const actor of ["red", "blue"] as const) {
      const body = pair[actor], other = pair[actor === "red" ? "blue" : "red"];
      const inverse = other.rotation.clone().invert();
      for (const side of ["L", "R"] as const) {
        for (const [a, b, length] of [
          [body[`shoulder${side}`], body[`elbow${side}`], BODY.upperArm],
          [body[`elbow${side}`], body[`wrist${side}`], BODY.forearm],
          [body[`hip${side}`], body[`knee${side}`], BODY.thigh],
          [body[`knee${side}`], body[`ankle${side}`], BODY.shin],
        ] as const) {
          expect(a.distanceTo(b)).toBeCloseTo(length, 6);
          for (const t of [0.1, 0.3, 0.5, 0.7, 0.9]) {
            const point = a.clone().lerp(b, t);
            for (const [center, radii] of [[other.chest, new Vector3(0.16, 0.15, 0.075)], [other.pelvis, new Vector3(0.12, 0.075, 0.07)], [other.head, new Vector3(0.075, 0.10, 0.08)]] as const) {
              expect(point.clone().sub(center).applyQuaternion(inverse).divide(radii).length(), `${actor}/${side}: ${a.toArray()} → ${b.toArray()} at ${t}`).toBeGreaterThanOrEqual(1);
            }
          }
        }
      }
      const mannequin = new Mannequin(body, 0);
      mannequin.root.updateMatrixWorld(true);
      let minimum = Infinity;
      const point = new Vector3();
      mannequin.root.traverse((object) => {
        if (!(object instanceof Mesh)) return;
        const vertices = object.geometry.getAttribute("position");
        for (let i = 0; i < vertices.count; i++) minimum = Math.min(minimum, point.fromBufferAttribute(vertices, i).applyMatrix4(object.matrixWorld).y);
      });
      mannequin.dispose();
      expect(minimum, `${actor}: mesh below floor`).toBeGreaterThanOrEqual(-0.002);
    }
    expect(pair.red.head.distanceTo(pair.blue.head)).toBeGreaterThan(0.20);
  });

  it("高いマウントと、ガードで姿勢を折る初動が開始姿勢と異なる", () => {
    const mount = scenarioById("mount-escape");
    const high = mount.opponentActions.find((a) => a.id === "high-mount-climb")!.attack;
    expect(rollPair(mount.setup).red.kneeL.z - rollPair(high).red.kneeL.z).toBeGreaterThan(0.20);
    const guard = scenarioById("closed-guard-posture");
    const pull = rollPair(guard.opponentActions.find((a) => a.id === "posture-break")!.attack);
    expect(pull.blue.head.y).toBeLessThan(rollPair(guard.setup).blue.head.y - 0.25);
    expect(pull.red.wristL.distanceTo(pull.blue.neck)).toBeLessThan(0.11);
  });

  it("未対応のペアを別の姿勢に置き換えない", () => {
    expect(() => rollPair({ red: "redMountTop", blue: "blueSeatedFront", badge: "" })).toThrow("No roll mannequins");
  });

  it("腕十字では手首を両手で保持し、肘が腰の近くに来る", () => {
    const { red, blue } = rollPair(scenarioById("attack-armbar-guard").attack);
    expect(red.wristL.distanceTo(blue.wristR)).toBeLessThan(0.05);
    expect(red.wristR.distanceTo(blue.wristR)).toBeLessThan(0.05);
    expect(red.pelvis.distanceTo(blue.elbowR)).toBeLessThan(0.17);
  });
});

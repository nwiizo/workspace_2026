import { expect, it } from "vitest";
import { Box3, Mesh, Vector3 } from "three";
import { Mannequin } from "../src/render/mannequin";
import { guardPair, practicePair } from "../src/render/practicePose";
import { neutralModelPose } from "../src/render/modelStudy";
import { rollPair } from "../src/render/rollPose";

it("再生時に形状を再確保せず、更新後も新規描画と同じ位置になる", () => {
  const model = new Mannequin(guardPair("stable", "blue").blue, 0x629de6);
  const geometryIds: string[] = [];
  model.root.traverse((o) => { if (o instanceof Mesh) geometryIds.push(o.geometry.uuid); });
  const before = new Box3().setFromObject(model.root, true);
  const pose = guardPair("pull", "blue").blue;
  model.updatePose(pose);
  const updated = new Box3().setFromObject(model.root, true);
  expect(updated.getCenter(new Vector3()).distanceTo(before.getCenter(new Vector3()))).toBeGreaterThan(0.03);
  const ids: string[] = [];
  model.root.traverse((o) => { if (o instanceof Mesh) ids.push(o.geometry.uuid); });
  expect(ids).toEqual(geometryIds);
  const fresh = new Mannequin(pose, 0x629de6);
  const expected = new Box3().setFromObject(fresh.root, true);
  expect(updated.min.distanceTo(expected.min)).toBeLessThan(1e-6);
  expect(updated.max.distanceTo(expected.max)).toBeLessThan(1e-6);
  model.dispose(); fresh.dispose();
});

it("寝た姿勢で生成した固定部分も、立位・うつ伏せ・腕十字への更新でずれない", () => {
  const model = new Mannequin(practicePair("half-bottom", "shield").blue, 0x629de6);
  for (const pose of [neutralModelPose(), practicePair("turtle-bottom", "compact").blue,
    practicePair("mount-base", "low").red,
    rollPair({ red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "" }).red]) {
    model.updatePose(pose);
    const updated = new Box3().setFromObject(model.root, true), fresh = new Mannequin(pose, 0x629de6);
    const expected = new Box3().setFromObject(fresh.root, true);
    expect(updated.min.distanceTo(expected.min)).toBeLessThan(1e-6);
    expect(updated.max.distanceTo(expected.max)).toBeLessThan(1e-6);
    expect(updated.min.y).toBeGreaterThanOrEqual(-0.002);
    fresh.dispose();
  }
  model.dispose();
});

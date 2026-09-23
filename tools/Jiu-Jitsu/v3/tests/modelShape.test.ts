import { describe, expect, it } from "vitest";
import { Box3, Mesh, Vector3 } from "three";
import { bodyFrame, Mannequin } from "../src/render/mannequin";
import { neutralModelPose } from "../src/render/modelStudy";
import { headGeometry, torsoGeometry } from "../src/render/mannequinShape";
import { bodyTargets, inspectRoutePose } from "../src/engine/route";

it("立っている足は踵から身体の前へ伸びる", () => {
  const pose = neutralModelPose(), model = new Mannequin(pose, 0x629de6);
  model.root.updateMatrixWorld(true);
  for (const side of ["L", "R"] as const) {
    const foot = model.root.children[0]!.children.find((obj) => obj instanceof Mesh && obj.scale.z > 0.09 && obj.scale.x > 0.05 && obj.position.distanceTo(pose[`ankle${side}`]) < 0.1) as Mesh;
    expect(foot).toBeDefined();
    const box = new Box3().setFromObject(foot, true);
    expect(box.getCenter(new Vector3()).z).toBeGreaterThan(pose[`ankle${side}`].z + 0.02);
    expect(box.min.y).toBeGreaterThanOrEqual(-0.002);
    expect(box.min.y).toBeLessThan(0.005);
  }
  model.dispose();
});

describe("頭と胴体の連続した断面", () => {
  it.each([torsoGeometry, headGeometry])("閉じた面で有限の頂点と外向きの面を持つ", (create) => {
    const geometry = create(), positions = geometry.getAttribute("position"), index = geometry.index!;
    const edges = new Map<string, number>();
    let volume = 0;
    const a = new Vector3(), b = new Vector3(), c = new Vector3();
    for (let i = 0; i < positions.count; i++) expect([...a.fromBufferAttribute(positions, i).toArray()].every(Number.isFinite)).toBe(true);
    for (let i = 0; i < index.count; i += 3) {
      const indices = [index.getX(i), index.getX(i + 1), index.getX(i + 2)];
      a.fromBufferAttribute(positions, indices[0]!); b.fromBufferAttribute(positions, indices[1]!); c.fromBufferAttribute(positions, indices[2]!);
      volume += a.dot(b.clone().cross(c)) / 6;
      expect(b.clone().sub(a).cross(c.clone().sub(a)).length()).toBeGreaterThan(1e-9);
      for (let edge = 0; edge < 3; edge++) {
        const key = [indices[edge], indices[(edge + 1) % 3]].sort((x, y) => x! - y!).join(":");
        edges.set(key, (edges.get(key) ?? 0) + 1);
      }
    }
    expect([...edges.values()].every((count) => count === 2)).toBe(true);
    expect(volume).toBeGreaterThan(0);
    geometry.dispose();
  });
});

it("形状を共有し、人形1体の描画と三角形の予算を守る", () => {
  const model = new Mannequin(neutralModelPose(), 0x629de6);
  let visible = 0, triangles = 0;
  const geometries = new Set<string>();
  model.root.traverseVisible((obj) => { if (obj instanceof Mesh) visible++; });
  model.root.traverse((obj) => { if (obj instanceof Mesh) {
    triangles += (obj.geometry.index?.count ?? obj.geometry.getAttribute("position").count) / 3;
    geometries.add(obj.geometry.uuid);
  } });
  expect(visible).toBeLessThanOrEqual(20);
  expect(triangles).toBeLessThan(18_000);
  expect(geometries.size).toBeLessThanOrEqual(12);
  model.dispose();
});

it("共有した形状を一度ずつ解放し、別の人形の形状を解放しない", () => {
  const first = new Mannequin(neutralModelPose(), 0x629de6), second = new Mannequin(neutralModelPose(), 0xd87966);
  const counts = new Map<string, number>(); let otherDisposed = 0;
  first.root.traverse((obj) => { if (obj instanceof Mesh && !counts.has(obj.geometry.uuid)) {
    const id = obj.geometry.uuid; counts.set(id, 0);
    obj.geometry.addEventListener("dispose", () => counts.set(id, counts.get(id)! + 1));
  } });
  second.root.traverse((obj) => { if (obj instanceof Mesh) obj.geometry.addEventListener("dispose", () => otherDisposed++); });
  first.dispose(); first.dispose();
  expect([...counts.values()].every((count) => count === 1)).toBe(true);
  expect(otherDisposed).toBe(0);
  second.dispose();
});

it("胴体を傾けたとき、股関節の外形だけが床へ入る場合も指摘する", () => {
  const blue = bodyTargets(neutralModelPose()), red = bodyTargets(neutralModelPose());
  blue.pelvis.set(0, 0.17, -1); blue.up.set(Math.sin(70 * Math.PI / 180), Math.cos(70 * Math.PI / 180), 0);
  const frame = bodyFrame(blue.pelvis, blue.up, blue.front);
  for (const [side, sign] of [["L", 1], ["R", -1]] as const) {
    const hip = frame.point(sign * 0.11, 0, 0), shoulder = frame.point(sign * 0.20, 0.43, 0);
    blue.legs[side] = { end: hip.clone().add(new Vector3(0.001, 0.79, 0.01)), pole: hip.clone().add(new Vector3(0.04, 0.41, 0)) };
    blue.arms[side] = { end: shoulder.clone().add(new Vector3(sign * 0.4, 0.30, 0.2)), pole: shoulder.clone().add(new Vector3(sign * 0.2, 0.2, 0.2)) };
  }
  const result = inspectRoutePose({ blue, red });
  expect(result.issues.some((issue) => issue.actor === "blue" && issue.level === "blocked" && issue.message.includes("床") && issue.point.distanceTo(result.pair.blue.hipL) < 1e-6)).toBe(true);
});

import { describe, expect, it } from "vitest";
import { RoutePlayback } from "../src/engine/routePlayback";
import { routePose, type RouteFrame } from "../src/engine/route";
import { backPair } from "../src/render/practicePose";

const pose = () => routePose(backPair("safe"));
const frames = (): RouteFrame[] => [
  { label: "開始", seconds: 2, pose: pose() },
  { label: "途中", seconds: 2, pose: pose() },
  { label: "終了", seconds: 4, pose: pose() },
];

describe("ルートの自動再生", () => {
  it("姿勢ごとの秒数を使い、遅い描画でも経過時間を失わない", () => {
    const player = new RoutePlayback(frames());
    expect(player.advance(3).position).toBeCloseTo(1.25);
    expect(player.advance(3).done).toBe(true);
  });
  it("一時停止した位置から続きの時間だけ進む", () => {
    const first = new RoutePlayback(frames());
    const paused = first.advance(1).position;
    const resumed = new RoutePlayback(frames(), paused);
    expect(resumed.advance(0.5).position).toBeCloseTo(0.75);
  });
  it("低速・倍速に相当する時間を渡すと同じ経路を進む", () => {
    expect(new RoutePlayback(frames()).advance(0.5).position).toBeCloseTo(0.25);
    expect(new RoutePlayback(frames()).advance(2).position).toBeCloseTo(1);
  });
  it("大きな時間差でも途中の衝突を飛ばして完走しない", () => {
    const a = pose();
    const route = [{ label: "開始", seconds: 1, pose: a }, { label: "入れ替わり", seconds: 1, pose: { blue: a.red, red: a.blue } }];
    const step = new RoutePlayback(route).advance(20);
    expect(step.problem).not.toBeNull();
    expect(step.position).toBeGreaterThan(0);
    expect(step.position).toBeLessThan(1);
    expect(step.done).toBe(false);
  });
});

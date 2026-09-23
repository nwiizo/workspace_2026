import { expect, it } from "vitest";
import { DuelEngine } from "../src/engine/duel";
import { duelPair } from "../src/render/duelPose";
import { inspectRoutePose, routePose } from "../src/engine/route";

it("下から支えを崩して準備を整えると、青が上を取り返せる", () => {
  const duel = new DuelEngine(2);
  duel.turn("frame"); duel.turn("frame");
  const beats = duel.turn("escape");
  expect(beats[0]?.after.top).toBe("blue");
  expect(beats[0]?.after.position).toBe("guard");
  expect(beats[0]?.after.points.blue).toBe(1);
});
it("上下に合わない操作は状態を変えず拒否する", () => {
  const duel = new DuelEngine(2);
  const before = structuredClone(duel.state);
  expect(() => duel.turn("advance")).toThrow();
  expect(duel.state).toEqual(before);
});
it("下では腰の移動と腕の防御、上では姿勢と返しの阻止を選べる", () => {
  const bottom = new DuelEngine(2);
  expect(bottom.available()).toEqual(expect.arrayContaining(["frame", "hip-escape", "hand-fight", "escape", "rest"]));
  const top = new DuelEngine(2, "pressure", "guard-top");
  expect(top.available()).toEqual(expect.arrayContaining(["secure", "posture", "deny", "advance", "rest"]));
  expect(bottom.available()).not.toContain("posture");
  expect(top.available()).not.toContain("hip-escape");
});
it("腰を引くには空間が必要で、作った空間があれば返しの準備が整う", () => {
  const open = new DuelEngine(2), pinned = new DuelEngine(2);
  pinned.state.control = 3;
  expect(open.turn("hip-escape")[0]?.after.preparation.blue).toBe(2);
  const blocked = pinned.turn("hip-escape")[0]!;
  expect(blocked.after.preparation.blue).toBe(0);
  expect(blocked.after.control).toBe(2);
});
it("腕を守ると相手の極めの準備を外せるが、上下は変わらない", () => {
  const duel = new DuelEngine(2, "pressure", "mount-bottom");
  duel.state.preparation.red = 1;
  const after = duel.turn("hand-fight")[0]!.after;
  expect(after.preparation.red).toBe(0);
  expect(after.top).toBe("red");
  expect(after.energy.blue).toBeLessThan(100);
});
it("腕の保持には安定が必要で、保持に手を使うと支配を1段階失う", () => {
  const stable = new DuelEngine(2, "pressure", "side-top"), unstable = new DuelEngine(2, "pressure", "side-top");
  stable.state.control = 2;
  const after = stable.turn("isolate")[0]!.after;
  expect(after.preparation.blue).toBe(1);
  expect(after.control).toBe(1);
  expect(unstable.turn("isolate")[0]?.after.preparation.blue).toBe(0);
});
it("返しの支えを外す行動は、相手の準備を消す代わりに自分の仕掛けを中断する", () => {
  const duel = new DuelEngine(2, "pressure", "side-top");
  duel.state.preparation = { blue: 1, red: 2 };
  const after = duel.turn("deny")[0]!.after;
  expect(after.preparation).toEqual({ blue: 0, red: 0 });
  expect(after.control).toBe(1);
});
it("サイドで作った極めの準備を、その場の腕十字へつなげられる", () => {
  const duel = new DuelEngine(2, "pressure", "side-top");
  duel.state.control = 2; duel.state.preparation.blue = 1;
  const beats = duel.turn("attack");
  expect(beats).toHaveLength(1);
  expect(beats[0]?.after.winner).toBe("blue");
  expect(inspectRoutePose(routePose(duelPair(duel.state))).issues.filter((i) => i.level === "blocked")).toEqual([]);
});
it("行動後に相手が反応し、同じseedなら同じ展開を再現できる", () => {
  const a = new DuelEngine(2), b = new DuelEngine(2);
  const beats = a.turn("frame");
  expect(beats.map((beat) => beat.actor)).toEqual(["blue", "red"]);
  expect(beats).toEqual(b.turn("frame"));
  expect(beats[0]?.after).not.toBe(a.state);
});
it("12手で試合を区切り、終了後の操作を拒否する", () => {
  const duel = new DuelEngine(5);
  for (let i = 0; i < 12 && !duel.state.ended; i++) duel.turn("rest");
  expect(duel.state.ended).toBe(true);
  expect(() => duel.turn("rest")).toThrow();
});

it("開始位置と相手の戦い方を変えても、数値と3D配置が崩れない", () => {
  for (const style of ["pressure", "counter"] as const) for (const start of ["guard-bottom", "guard-top", "mount-bottom", "side-top"] as const) for (let seed = 1; seed <= 8; seed++) {
    const duel = new DuelEngine(seed, style, start);
    for (let turn = 0; turn < 12 && !duel.state.ended; turn++) {
      const options = duel.available();
      const action = options[(seed + turn) % options.length]!;
      for (const beat of duel.turn(action)) {
        expect(beat.after.control).toBeGreaterThanOrEqual(0); expect(beat.after.control).toBeLessThanOrEqual(3);
        for (const energy of Object.values(beat.after.energy)) { expect(energy).toBeGreaterThanOrEqual(0); expect(energy).toBeLessThanOrEqual(100); }
        expect(inspectRoutePose(routePose(duelPair(beat.after))).issues.filter((i) => i.level === "blocked"), `${style}/${start}/${seed}/${action}`).toEqual([]);
      }
    }
    expect(duel.state.ended).toBe(true);
  }
});

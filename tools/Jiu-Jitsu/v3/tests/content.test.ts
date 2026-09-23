import { describe, expect, it } from "vitest";
import { SCENARIOS } from "../src/content/scenarios";
import { jointById } from "../src/anatomy/joints";
import { normalizeNext } from "../src/engine/roll";

const IDS = new Set(SCENARIOS.map((s) => s.id));

describe("scenario id と next 参照", () => {
  it("id が一意", () => {
    expect(IDS.size).toBe(SCENARIOS.length);
  });

  it("全 choice の next が実在する scenario id を参照し weight が正", () => {
    for (const s of SCENARIOS) {
      for (const [i, o] of s.options.entries()) {
        const refs = normalizeNext(o);
        expect(refs.length, `${s.id}[${i}] next`).toBeGreaterThan(0);
        for (const { id, weight } of refs) {
          expect(IDS.has(id), `${s.id}[${i}] -> ${id}`).toBe(true);
          expect(weight, `${s.id}[${i}] -> ${id} weight`).toBeGreaterThan(0);
        }
      }
    }
  });
});

describe.each(SCENARIOS)("$id の不変条件", (s) => {
  it("readCues が 2〜4 個", () => {
    expect(s.readCues.length).toBeGreaterThanOrEqual(2);
    expect(s.readCues.length).toBeLessThanOrEqual(4);
  });

  it("opponentActions が 2 件以上で各初動が cue/attack/readCues/pressure/weight を持つ", () => {
    expect(s.opponentActions.length).toBeGreaterThanOrEqual(2);
    const actionIds = new Set(s.opponentActions.map((a) => a.id));
    expect(actionIds.size).toBe(s.opponentActions.length);
    for (const a of s.opponentActions) {
      expect(a.cue.length, `${s.id}:${a.id} cue`).toBeGreaterThan(0);
      expect(a.label.length, `${s.id}:${a.id} label`).toBeGreaterThan(0);
      expect(a.weight, `${s.id}:${a.id} weight`).toBeGreaterThan(0);
      expect(a.attack.red, `${s.id}:${a.id} attack.red`).toBeTruthy();
      expect(a.attack.blue, `${s.id}:${a.id} attack.blue`).toBeTruthy();
      expect(a.attack.badge.length, `${s.id}:${a.id} attack.badge`).toBeGreaterThan(0);
      expect(a.readCues.length, `${s.id}:${a.id} readCues`).toBeGreaterThanOrEqual(2);
      expect(a.readCues.length, `${s.id}:${a.id} readCues`).toBeLessThanOrEqual(4);
      expect(a.pressure.early.length, `${s.id}:${a.id} pressure.early`).toBeGreaterThan(0);
      expect(a.pressure.urgent.length, `${s.id}:${a.id} pressure.urgent`).toBeGreaterThan(0);
    }
  });

  it("requiresAction / forbiddenAction は同じ scenario の初動 id のみ参照", () => {
    const actionIds = new Set(s.opponentActions.map((a) => a.id));
    for (const [i, o] of s.options.entries()) {
      for (const ref of [...(o.requiresAction ?? []), ...(o.forbiddenAction ?? [])]) {
        expect(actionIds.has(ref), `${s.id}[${i}] action ref: ${ref}`).toBe(true);
      }
    }
  });

  it("正解 choice は reaction と next を、不正解 choice は consequence を持つ", () => {
    for (const [i, o] of s.options.entries()) {
      if (o.correct) {
        expect(o.reaction, `${s.id}[${i}] reaction`).toBeTruthy();
        expect(o.next.length, `${s.id}[${i}] next`).toBeGreaterThan(0);
      } else {
        expect(o.consequence, `${s.id}[${i}] consequence`).toBeTruthy();
      }
    }
  });

  it("stateBias が空配列でない", () => {
    expect(s.stateBias.length).toBeGreaterThan(0);
  });

  it("focusJoints が 1 個以上で jointById が解決できる", () => {
    expect(s.focusJoints.length).toBeGreaterThan(0);
    for (const id of s.focusJoints) {
      expect(jointById(id).id).toBe(id);
    }
  });

  it("timeLimitSec が正、pressure と situation/prompt/principle が空でない", () => {
    expect(s.timeLimitSec).toBeGreaterThan(0);
    expect(s.pressure.early.length).toBeGreaterThan(0);
    expect(s.pressure.urgent.length).toBeGreaterThan(0);
    expect(s.situation.length).toBeGreaterThan(0);
    expect(s.prompt.length).toBeGreaterThan(0);
    expect(s.principle.length).toBeGreaterThan(0);
  });
});

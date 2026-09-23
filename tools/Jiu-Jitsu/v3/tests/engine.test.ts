import { describe, expect, it } from "vitest";
import { RollEngine, allItemKeys, normalizeNext, visibleOptions, type RollConfig } from "../src/engine/roll";
import { scenarioById, SCENARIOS } from "../src/content/scenarios";

const NOW = 1_000_000;

function mkEngine(overrides: Partial<RollConfig> = {}): RollEngine {
  return new RollEngine(
    { focus: "mixed", uniform: "gi", difficulty: "beginner", seed: 42, ...overrides },
    {},
    NOW,
  );
}

describe("start", () => {
  it("step が得られ、選択肢 2 件以上・正解ちょうど 1 つ", () => {
    const step = mkEngine().start();
    expect(step.index).toBe(0);
    expect(step.options.length).toBeGreaterThanOrEqual(2);
    expect(step.options.filter((o) => o.correct).length).toBe(1);
    expect(step.statesAtEntry).toEqual([]);
  });

  it("startId 指定でその局面から始まる (苦手局面の再ロール)", () => {
    const step = mkEngine().start("mount-escape");
    expect(step.scenario.id).toBe("mount-escape");
  });

  it("入門は timeLimitSec が null、実戦は scenario の制限時間", () => {
    expect(mkEngine().start().timeLimitSec).toBeNull();
    const live = mkEngine({ difficulty: "live" }).start();
    expect(live.timeLimitSec).toBe(live.scenario.timeLimitSec);
  });
});

describe("answer", () => {
  it("正解: correct=true / readCues=初動の読む線 / missedCues 空", () => {
    const engine = mkEngine();
    const step = engine.start();
    const outcome = engine.answer(step.options.findIndex((o) => o.correct));
    expect(outcome.correct).toBe(true);
    expect(outcome.timedOut).toBe(false);
    expect(outcome.readCues).toEqual(step.action.readCues);
    expect(outcome.missedCues).toEqual([]);
    expect(outcome.srsKey).toBe(`${step.scenario.id}:${step.action.id}`);
  });

  it("不正解: correct=false / missedCues=初動の読む線 / readCues 空", () => {
    const engine = mkEngine();
    const step = engine.start();
    const outcome = engine.answer(step.options.findIndex((o) => !o.correct));
    expect(outcome.correct).toBe(false);
    expect(outcome.readCues).toEqual([]);
    expect(outcome.missedCues).toEqual(step.action.readCues);
  });

  it("不正解でもロールは続き、consequence 側の next へ遷移する", () => {
    const engine = mkEngine();
    const step = engine.start();
    const wrongIndex = step.options.findIndex((o) => !o.correct);
    const wrong = step.options[wrongIndex]!;
    const outcome = engine.answer(wrongIndex);
    expect(outcome.rollEnded).toBe(false);
    expect(outcome.nextId).not.toBeNull();
    const candidateIds = normalizeNext(wrong).map((n) => n.id);
    expect(candidateIds).toContain(outcome.nextId);
    const next = engine.advance();
    expect(next?.scenario.id).toBe(outcome.nextId);
    expect(next?.index).toBe(1);
  });
});

describe("ロール終了", () => {
  it("maxSteps に達すると rollEnded=true / advance() は null", () => {
    const engine = mkEngine({ maxSteps: 1 });
    const step = engine.start();
    const outcome = engine.answer(step.options.findIndex((o) => o.correct));
    expect(outcome.rollEnded).toBe(true);
    expect(outcome.nextId).toBeNull();
    expect(engine.advance()).toBeNull();
    expect(engine.history.length).toBe(1);
  });
});

describe("timeout (実戦の時間切れ)", () => {
  it("correct=false / timedOut=true で不正解 choice が選ばれる", () => {
    const engine = mkEngine({ difficulty: "live" });
    engine.start();
    const outcome = engine.timeout();
    expect(outcome.correct).toBe(false);
    expect(outcome.timedOut).toBe(true);
    expect(outcome.choice.correct).toBe(false);
  });
});

describe("stateEffects の引き継ぎ", () => {
  it("mount-escape の high-mount-climb 正解が guard-recovered を与え、attack-from-side で専用選択肢が現れる", () => {
    const mountEscape = scenarioById("mount-escape");
    const escapeChoice = mountEscape.options.find((o) =>
      o.requiresAction?.includes("high-mount-climb"),
    );
    expect(escapeChoice?.correct).toBe(true);
    expect(escapeChoice?.stateEffects?.add).toContain("guard-recovered");

    const side = scenarioById("attack-from-side");
    const withState = visibleOptions(side, "frame-recovery", "gi", ["guard-recovered"]);
    const gated = withState.find((o) => o.requiresState?.includes("guard-recovered"));
    expect(gated).toBeDefined();
    expect(gated?.correct).toBe(true);
    const withoutState = visibleOptions(side, "frame-recovery", "gi", []);
    expect(withoutState.some((o) => o.requiresState?.includes("guard-recovered"))).toBe(false);
  });
});

describe("focus と uniform の出し分け", () => {
  it("focus=defense の開始局面は role=defense (複数シードで確認)", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const step = mkEngine({ focus: "defense", seed }).start();
      expect(step.scenario.role).toBe("defense");
    }
  });

  it("uniform=gi では nogiOnly が、nogi では giOnly が出ない (ロールを歩いて確認)", () => {
    for (const uniform of ["gi", "nogi"] as const) {
      const engine = mkEngine({ uniform, seed: 7 });
      let step: ReturnType<RollEngine["start"]> | null = engine.start();
      while (step) {
        for (const o of step.options) {
          if (uniform === "gi") expect(o.nogiOnly, step.scenario.id).not.toBe(true);
          else expect(o.giOnly, step.scenario.id).not.toBe(true);
        }
        engine.answer(0);
        step = engine.advance();
      }
    }
  });
});

describe("シード決定性", () => {
  it("同一 seed で start → answer(0) → advance ×3 の系列が再現される", () => {
    const trace = (): string[] => {
      const engine = mkEngine({ seed: 123 });
      const log: string[] = [];
      let step = engine.start();
      for (let i = 0; i < 3; i++) {
        log.push(step.scenario.id, step.action.id, ...step.options.map((o) => o.jp));
        const outcome = engine.answer(0);
        log.push(String(outcome.correct), String(outcome.nextId));
        const next = engine.advance();
        if (!next) break;
        step = next;
      }
      return log;
    };
    expect(trace()).toEqual(trace());
  });

  it("history が回答内容を記録する", () => {
    const engine = mkEngine({ seed: 5, maxSteps: 2 });
    const step = engine.start();
    engine.answer(step.options.findIndex((o) => o.correct));
    const next = engine.advance();
    expect(next).not.toBeNull();
    engine.answer(0);
    expect(engine.history.length).toBe(2);
    expect(engine.history[0]?.scenarioId).toBe(step.scenario.id);
    expect(engine.history[0]?.correct).toBe(true);
    expect(engine.history[1]?.nextId).toBeNull();
  });
});

describe("allItemKeys", () => {
  it("全 (局面 × 初動) を一意キーで列挙する", () => {
    const items = allItemKeys();
    const expected = SCENARIOS.reduce((n, s) => n + s.opponentActions.length, 0);
    expect(items.length).toBe(expected);
    expect(new Set(items.map((i) => i.key)).size).toBe(expected);
  });
});

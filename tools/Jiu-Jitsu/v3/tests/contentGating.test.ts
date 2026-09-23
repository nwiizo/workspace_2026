import { describe, expect, it } from "vitest";
import { SCENARIOS } from "../src/content/scenarios";
import type { Scenario, StateFlag, Uniform } from "../src/content/types";
import { visibleOptions } from "../src/engine/roll";

const UNIFORMS: Uniform[] = ["gi", "nogi"];

/** scenario の requiresState / forbiddenState に登場する状態フラグ */
function gatingFlags(s: Scenario): StateFlag[] {
  const flags = new Set<StateFlag>();
  for (const o of s.options) {
    for (const f of o.requiresState ?? []) flags.add(f);
    for (const f of o.forbiddenState ?? []) flags.add(f);
  }
  return [...flags];
}

describe("正解一意性 (scenario × 初動 × uniform × 状態集合)", () => {
  for (const s of SCENARIOS) {
    const stateSets: StateFlag[][] = [[], ...gatingFlags(s).map((f) => [f])];
    for (const action of s.opponentActions) {
      for (const uniform of UNIFORMS) {
        for (const states of stateSets) {
          const label = `${s.id} × ${action.id} × ${uniform} × [${states.join(",")}]`;
          it(label, () => {
            const visible = visibleOptions(s, action.id, uniform, states);
            const correct = visible.filter((o) => o.correct);
            expect(correct.length, `${label}: 正解ちょうど 1 つ`).toBe(1);
            expect(visible.length, `${label}: 選択肢 2 つ以上`).toBeGreaterThanOrEqual(2);
          });
        }
      }
    }
  }
});

describe("状態ゲートの参照整合", () => {
  it("stateEffects.add / remove と requiresState / forbiddenState が stateBias の語彙と同じ StateFlag 空間にある", () => {
    // 型で保証されるが、データとして空配列 gating (requiresState: []) が
    // 「常に非表示」の罠にならないことを確認する
    for (const s of SCENARIOS) {
      for (const [i, o] of s.options.entries()) {
        if (o.requiresState) {
          expect(o.requiresState.length, `${s.id}[${i}] requiresState が空`).toBeGreaterThan(0);
        }
        if (o.forbiddenState) {
          expect(o.forbiddenState.length, `${s.id}[${i}] forbiddenState が空`).toBeGreaterThan(0);
        }
        if (o.requiresAction) {
          expect(o.requiresAction.length, `${s.id}[${i}] requiresAction が空`).toBeGreaterThan(0);
        }
      }
    }
  });
});

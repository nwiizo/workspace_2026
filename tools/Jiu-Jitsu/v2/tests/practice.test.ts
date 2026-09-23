import { describe, expect, it } from "vitest";
import { PRACTICE_LESSONS } from "../src/content/practice";
import { nextPractice, practiceKey, PracticeRun } from "../src/engine/practice";
import { recordResult } from "../src/engine/srs";
import { practicePair } from "../src/render/practicePose";

function move(run: PracticeRun, action: string): void {
  run.act(action);
  run.continue();
}

describe("一手ずつのポジション練習", () => {
  it("支えなしの海老では進めず、支え→空間→膝でガードを回復する", () => {
    const run = new PracticeRun("side-space", "check");
    const blocked = run.act("hip");
    expect(blocked?.progressed).toBe(false);
    expect(run.nodeId).toBe("pinned");
    run.continue();
    move(run, "frame");
    expect(run.nodeId).toBe("framed");
    move(run, "hip");
    expect(run.nodeId).toBe("space");
    move(run, "knee");
    expect(run.phase).toBe("complete");
    const pair = practicePair(run.lesson.id, run.nodeId);
    expect(pair.blue.chest.y).toBeLessThan(pair.red.chest.y);
    expect(run.mistakes).toBe(1);
    expect(run.takeResult()).toEqual({ key: "practice:side-space:check", correct: false });
  });

  it("相手が支えを潰したら、同じ手順の海老では進めない", () => {
    const run = new PracticeRun("side-space", "check", 1);
    move(run, "frame");
    expect(run.nodeId).toBe("pressured");
    move(run, "hip");
    expect(run.nodeId).toBe("pressured");
    move(run, "frame");
    move(run, "hip");
    move(run, "knee");
    expect(run.phase).toBe("complete");
  });

  it("空間を作った後でも、腕を伸ばして押すと支えを失い、最初の形へ戻る", () => {
    const run = new PracticeRun("side-space", "check");
    move(run, "frame"); move(run, "hip");
    move(run, "push");
    expect(run.nodeId).toBe("pinned");
    expect(run.node.progress).toBe(0);
    expect(run.mistakes).toBe(1);
    move(run, "knee");
    expect(run.nodeId).toBe("pinned");
  });

  it("低いマウントでは手足を止めてから返し、次はガード内の上になる", () => {
    const run = new PracticeRun("mount-base", "check");
    move(run, "bridge");
    expect(run.nodeId).toBe("low");
    move(run, "elbows");
    move(run, "trap");
    move(run, "bridge");
    const pair = practicePair(run.lesson.id, run.nodeId);
    expect(pair.blue.chest.y).toBeGreaterThan(pair.red.chest.y);
    expect(run.phase).toBe("complete");
  });

  it("高いマウントでは同じ返し方を通さず、膝肘からガードを回復する", () => {
    const run = new PracticeRun("mount-base", "check", 1);
    move(run, "elbows");
    move(run, "trap");
    expect(run.nodeId).toBe("read-high");
    move(run, "knee");
    move(run, "knee");
    const pair = practicePair(run.lesson.id, run.nodeId);
    expect(pair.blue.chest.y).toBeLessThan(pair.red.chest.y);
    expect(run.phase).toBe("complete");
  });

  it("フィードバック確認前の連打、完了後の入力、重複記録を受け付けない", () => {
    const run = new PracticeRun("side-space", "check");
    run.act("frame");
    expect(run.act("hip")).toBeNull();
    expect(run.history).toHaveLength(1);
    run.continue();
    move(run, "hip");
    move(run, "knee");
    expect(run.act("knee")).toBeNull();
    expect(run.takeResult()?.correct).toBe(true);
    expect(run.takeResult()).toBeNull();
    expect(run.history).toHaveLength(3);
  });

  it("ヒント後の完了を、ヒントなしで判断できた記録にしない", () => {
    const run = new PracticeRun("side-space", "check");
    expect(run.hint()).not.toBe("");
    move(run, "frame"); move(run, "hip"); move(run, "knee");
    expect(run.takeResult()?.correct).toBe(false);
  });

  it("通常のタップは減点せず中断し、絞め完成の課題では安全な正答になる", () => {
    const stopped = new PracticeRun("side-space", "check");
    stopped.act("tap");
    expect(stopped.phase).toBe("stopped");
    expect(stopped.mistakes).toBe(0);
    expect(stopped.takeResult()).toBeNull();
    const safety = new PracticeRun("back-safety", "check", 1);
    move(safety, "tap");
    expect(safety.phase).toBe("complete");
    expect(safety.node.end).toBe("safety");
    expect(safety.takeResult()?.correct).toBe(true);
  });

  it("存在しない操作で状態や記録が変わらない", () => {
    const run = new PracticeRun("side-space", "learn");
    expect(run.act("unknown")).toBeNull();
    expect(run.phase).toBe("acting");
    expect(run.history).toHaveLength(0);
  });
});

describe("学習内容全体の到達性", () => {
  for (const lesson of PRACTICE_LESSONS) {
    for (const [variant, start] of lesson.starts.entries()) {
      it(`${lesson.id}/${start}: 全分岐に終点があり、青が自分のまま完了できる`, () => {
        const visited = new Set<string>();
        const inspect = (id: string, ancestors: Set<string>): void => {
          expect(ancestors.has(id), `loop at ${id}`).toBe(false);
          const node = lesson.nodes[id];
          expect(node, `missing ${id}`).toBeDefined();
          visited.add(id);
          if (node.end) { expect(node.progress).toBe(lesson.steps.length); return; }
          expect(Object.keys(node.moves).length).toBeGreaterThan(0);
          for (const [action, edge] of Object.entries(node.moves)) {
            expect(action === "tap" || lesson.actions.some((a) => a.id === action)).toBe(true);
            inspect(edge.to, new Set([...ancestors, id]));
          }
        };
        inspect(start, new Set());
        const run = new PracticeRun(lesson.id, "check", variant);
        for (let i = 0; i < 10 && run.phase !== "complete"; i++) {
          move(run, Object.keys(run.node.moves)[0]!);
        }
        expect(run.phase).toBe("complete");
        expect(run.mistakes).toBe(0);
        expect(run.takeResult()?.correct).toBe(true);
      });
    }
  }
});

describe("次の練習", () => {
  it("まず未学習へ、学習後は確認へ、確認を通ったら次の課題へ進める", () => {
    let srs = recordResult({}, practiceKey("side-space", "learn"), true, 100);
    expect(nextPractice(srs, 101)).toEqual({ lessonId: "side-space", mode: "check" });
    srs = recordResult(srs, practiceKey("side-space", "check"), true, 100);
    expect(nextPractice(srs, 101)).toEqual({ lessonId: "mount-base", mode: "learn" });
  });
  it("確認で失敗した課題や期限の来た課題を復習する", () => {
    let srs = recordResult({}, practiceKey("side-space", "learn"), true, 100);
    srs = recordResult(srs, practiceKey("side-space", "check"), false, 100);
    expect(nextPractice(srs, 101)).toEqual({ lessonId: "side-space", mode: "check" });
    srs = recordResult(srs, practiceKey("side-space", "check"), true, 100);
    expect(nextPractice(srs, 100 + 4 * 60 * 60 * 1000)).toEqual({ lessonId: "side-space", mode: "check" });
  });
});

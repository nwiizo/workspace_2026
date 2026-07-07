import { describe, expect, it } from "vitest";
import {
  BOX_INTERVALS_MS,
  MAX_BOX,
  beltFor,
  isDue,
  itemKey,
  priorityOf,
  recordResult,
  type SrsState,
} from "../src/engine/srs";
import { loadProgress, memoryStore, saveProgress, type ProgressData } from "../src/engine/storage";

const NOW = 1_000_000;

describe("recordResult", () => {
  it("正解で box が 1 上がり dueAt = now + 間隔になる", () => {
    const key = itemKey("mount-escape", "arm-isolation");
    const s1 = recordResult({}, key, true, NOW);
    expect(s1[key]?.box).toBe(1);
    expect(s1[key]?.dueAt).toBe(NOW + BOX_INTERVALS_MS[1]);
    expect(s1[key]?.attempts).toBe(1);
    expect(s1[key]?.correct).toBe(1);
  });

  it("連続正解しても box は MAX_BOX で頭打ち", () => {
    const key = "k";
    let s: SrsState = {};
    for (let i = 0; i < MAX_BOX + 3; i++) s = recordResult(s, key, true, NOW + i);
    expect(s[key]?.box).toBe(MAX_BOX);
    expect(s[key]?.dueAt).toBe(NOW + MAX_BOX + 2 + BOX_INTERVALS_MS[MAX_BOX]);
  });

  it("不正解で box 0 に戻り即時再出題 (dueAt = now)", () => {
    const key = "k";
    let s: SrsState = recordResult({}, key, true, NOW);
    s = recordResult(s, key, true, NOW);
    s = recordResult(s, key, false, NOW + 10);
    expect(s[key]?.box).toBe(0);
    expect(s[key]?.dueAt).toBe(NOW + 10 + BOX_INTERVALS_MS[0]);
    expect(s[key]?.attempts).toBe(3);
    expect(s[key]?.correct).toBe(2);
  });
});

describe("isDue", () => {
  it("未学習は true", () => {
    expect(isDue({}, "unknown", NOW)).toBe(true);
  });

  it("dueAt が未来なら false、過ぎたら true", () => {
    const s = recordResult({}, "k", true, NOW); // dueAt = NOW + 4h
    expect(isDue(s, "k", NOW + 1)).toBe(false);
    expect(isDue(s, "k", NOW + BOX_INTERVALS_MS[1])).toBe(true);
  });
});

describe("priorityOf", () => {
  it("未学習 > 期日超過 > 未期日 の順で大きい", () => {
    const learned = recordResult({}, "k", true, NOW);
    const fresh = priorityOf({}, "k", NOW);
    const overdue = priorityOf(learned, "k", NOW + BOX_INTERVALS_MS[1] + 1);
    const notDue = priorityOf(learned, "k", NOW + 1);
    expect(fresh).toBeGreaterThan(overdue);
    expect(overdue).toBeGreaterThan(notDue);
    expect(notDue).toBeLessThan(1);
  });
});

describe("beltFor", () => {
  it("習得 0 は白帯、全習得は黒帯", () => {
    expect(beltFor(0, 20)).toBe("白帯");
    expect(beltFor(20, 20)).toBe("黒帯");
  });

  it("項目 0 件でも白帯 (0 除算しない)", () => {
    expect(beltFor(0, 0)).toBe("白帯");
  });
});

describe("storage", () => {
  it("save → load でラウンドトリップする", () => {
    const store = memoryStore();
    const data: ProgressData = {
      version: 1,
      srs: { "a:b": { box: 2, dueAt: 123, attempts: 4, correct: 3, lastAt: 100 } },
      rollsCompleted: 7,
    };
    saveProgress(store, data);
    expect(loadProgress(store)).toEqual(data);
  });

  it("壊れた JSON / 未知バージョンは EMPTY に戻る", () => {
    const KEY = "jiu-jitsu-dojo-v2/progress";
    const empty: ProgressData = { version: 1, srs: {}, rollsCompleted: 0 };
    const broken = memoryStore();
    broken.setItem(KEY, "{not json");
    expect(loadProgress(broken)).toEqual(empty);
    const wrongVersion = memoryStore();
    wrongVersion.setItem(KEY, JSON.stringify({ version: 99 }));
    expect(loadProgress(wrongVersion)).toEqual(empty);
    expect(loadProgress(memoryStore())).toEqual(empty);
  });
});

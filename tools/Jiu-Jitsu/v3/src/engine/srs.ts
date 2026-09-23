// Leitner 式の間隔反復。(局面 × 相手初動) を 1 学習項目とし、
// 忘れかけた頃に再出題されるよう箱 (box) と次回期日を管理する。
// box が上がるほど間隔が伸び、間違えると box 0 へ戻る。

export interface SrsItem {
  box: number;
  dueAt: number;
  attempts: number;
  correct: number;
  lastAt: number;
}

export type SrsState = Record<string, SrsItem>;

/** box → 次回出題までの間隔 (ms) */
export const BOX_INTERVALS_MS = [
  0, // box0: 即時 (同セッションで再出題対象)
  4 * 60 * 60 * 1000, // 4h
  24 * 60 * 60 * 1000, // 1d
  3 * 24 * 60 * 60 * 1000, // 3d
  7 * 24 * 60 * 60 * 1000, // 7d
  14 * 24 * 60 * 60 * 1000, // 14d
] as const;

export const MAX_BOX = BOX_INTERVALS_MS.length - 1;
/** この box 以上を「習得済み」とみなす (帯・ダッシュボード表示) */
export const MASTERY_BOX = 3;

export function itemKey(scenarioId: string, actionId: string): string {
  return `${scenarioId}:${actionId}`;
}

export function recordResult(state: SrsState, key: string, correct: boolean, now: number): SrsState {
  const prev = state[key];
  const box = correct ? Math.min((prev?.box ?? 0) + 1, MAX_BOX) : 0;
  return {
    ...state,
    [key]: {
      box,
      dueAt: now + (BOX_INTERVALS_MS[box] ?? 0),
      attempts: (prev?.attempts ?? 0) + 1,
      correct: (prev?.correct ?? 0) + (correct ? 1 : 0),
      lastAt: now,
    },
  };
}

export function isDue(state: SrsState, key: string, now: number): boolean {
  const item = state[key];
  return !item || item.dueAt <= now;
}

/** 未学習 > 期日超過 > box が低い、の順で優先度が高い (大きいほど優先) */
export function priorityOf(state: SrsState, key: string, now: number): number {
  const item = state[key];
  if (!item) return 3;
  if (item.dueAt <= now) return 2 + (MAX_BOX - item.box) / (MAX_BOX + 1);
  return (MAX_BOX - item.box) / (MAX_BOX + 1);
}

export function masteredCount(state: SrsState): number {
  return Object.values(state).filter((i) => i.box >= MASTERY_BOX).length;
}

export function dueCount(state: SrsState, keys: readonly string[], now: number): number {
  return keys.filter((k) => isDue(state, k, now)).length;
}

const BELTS = ["白帯", "青帯", "紫帯", "茶帯", "黒帯"] as const;

/** 習得項目数 → 帯。totalItems は全 (局面×初動) 数 */
export function beltFor(mastered: number, totalItems: number): (typeof BELTS)[number] {
  const ratio = totalItems > 0 ? mastered / totalItems : 0;
  if (ratio >= 0.95) return BELTS[4];
  if (ratio >= 0.7) return BELTS[3];
  if (ratio >= 0.45) return BELTS[2];
  if (ratio >= 0.2) return BELTS[1];
  return BELTS[0];
}

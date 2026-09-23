// 稽古記録の永続化。localStorage を注入可能にしてテストではメモリ実装を使う。

import { MAX_BOX, type SrsItem, type SrsState } from "./srs";

export interface ProgressData {
  version: 1;
  srs: SrsState;
  rollsCompleted: number;
}

const KEY = "jiu-jitsu-dojo-v3/progress";

export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const EMPTY: ProgressData = { version: 1, srs: {}, rollsCompleted: 0 };

const count = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const object = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);

function isSrsItem(value: unknown): value is SrsItem {
  return object(value) && count(value.box) && value.box <= MAX_BOX &&
    count(value.attempts) && count(value.correct) && value.correct <= value.attempts &&
    count(value.dueAt) && value.dueAt <= 8.64e15 && count(value.lastAt) && value.lastAt <= 8.64e15;
}

export function loadProgress(store: KeyValueStore): ProgressData {
  try {
    const raw = store.getItem(KEY);
    if (!raw) return { ...EMPTY };
    const parsed: unknown = JSON.parse(raw);
    if (object(parsed) && parsed.version === 1) {
      const entries = object(parsed.srs) ? Object.entries(parsed.srs) : [];
      const srs = Object.fromEntries(entries.filter((entry): entry is [string, SrsItem] => isSrsItem(entry[1])));
      return { version: 1, srs, rollsCompleted: count(parsed.rollsCompleted) ? parsed.rollsCompleted : 0 };
    }
    return { ...EMPTY };
  } catch {
    return { ...EMPTY };
  }
}

export function saveProgress(store: KeyValueStore, data: ProgressData): void {
  try {
    store.setItem(KEY, JSON.stringify(data));
  } catch {
    // ストレージ不可 (プライベートモード等) でも稽古自体は続行できる
  }
}

export function memoryStore(): KeyValueStore {
  const map = new Map<string, string>();
  return {
    getItem: (k) => map.get(k) ?? null,
    setItem: (k, v) => void map.set(k, v),
  };
}

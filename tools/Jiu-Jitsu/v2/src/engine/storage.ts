// 稽古記録の永続化。localStorage を注入可能にしてテストではメモリ実装を使う。

import type { SrsState } from "./srs";

export interface ProgressData {
  version: 1;
  srs: SrsState;
  rollsCompleted: number;
}

const KEY = "jiu-jitsu-dojo-v2/progress";

export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const EMPTY: ProgressData = { version: 1, srs: {}, rollsCompleted: 0 };

export function loadProgress(store: KeyValueStore): ProgressData {
  try {
    const raw = store.getItem(KEY);
    if (!raw) return { ...EMPTY };
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      (parsed as { version?: unknown }).version === 1
    ) {
      const p = parsed as ProgressData;
      return { version: 1, srs: p.srs ?? {}, rollsCompleted: p.rollsCompleted ?? 0 };
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

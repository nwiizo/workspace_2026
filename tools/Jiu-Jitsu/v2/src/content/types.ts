// 教育コンテンツの型定義。
// 旧 techniques.js のデータ形を型で固定し、旧 validate-data.mjs が担っていた
// 参照整合の大半をコンパイル時に落とす (PoseName / ScenarioId / StateFlag が型)。

import type { AnatomyJointId } from "../anatomy/types";
import type { PoseName } from "../render/poses";

export type Role = "defense" | "offense";
export type Uniform = "gi" | "nogi";

export type ScenarioId =
  | "back-defense"
  | "mount-escape"
  | "side-escape"
  | "closed-guard-posture"
  | "attack-from-mount"
  | "attack-from-back"
  | "attack-from-side"
  | "attack-armbar-guard"
  | "attack-triangle-guard";

/** 回答結果として次局面へ引き継がれる状態フラグ */
export type StateFlag =
  | "neck-safe"
  | "neck-exposed"
  | "back-exposed"
  | "arm-exposed"
  | "top-base"
  | "guard-recovered"
  | "frame-lost"
  | "knee-shield"
  | "posture-safe"
  | "posture-broken"
  | "angle-created"
  | "stack-pressure";

/** 局面の 3D 表示 (赤/青ポーズ + 説明バッジ。バッジは作者管理の HTML) */
export interface Stage {
  red: PoseName;
  blue: PoseName;
  badge: string;
}

/** 判断タイマー中に表示する相手の能動アクション */
export interface Pressure {
  early: string;
  urgent: string;
}

/** 相手の初動。局面内で重み付き選択され、attack ポーズ・読む線・pressure を上書きする */
export interface OpponentAction {
  id: string;
  label: string;
  /** 回答後に返す「初動の読み」 */
  cue: string;
  weight: number;
  attack: Stage;
  readCues: string[];
  pressure: Pressure;
}

export type NextRef = ScenarioId | { id: ScenarioId; weight: number };

export interface Choice {
  jp: string;
  en: string;
  correct: boolean;
  /** モード限定の選択肢 (ギ=襟 / ノーギ=手首のような技セット分岐) */
  giOnly?: boolean;
  nogiOnly?: boolean;
  /** この初動のときだけ表示 / このときは非表示 */
  requiresAction?: string[];
  forbiddenAction?: string[];
  /** この状態フラグがあるときだけ表示 / あるときは非表示 */
  requiresState?: StateFlag[];
  forbiddenState?: StateFlag[];
  /** 回答結果として次局面へ残す状態 */
  stateEffects?: { add?: StateFlag[]; remove?: StateFlag[] };
  /** 正解: 相手の反応 / 不正解: 相手の追撃 */
  reaction?: string;
  consequence?: string;
  next: NextRef[];
  result: Stage;
  feedback: string;
}

export interface Scenario {
  id: ScenarioId;
  role: Role;
  belt: string;
  positionJp: string;
  positionEn: string;
  term: string;
  /** この局面で危険に晒される関節 — 関節ラボへのリンク */
  focusJoints: AnatomyJointId[];
  /** 引き継ぎ状態がこのフラグと一致すると次局面候補として出やすくなる */
  stateBias: StateFlag[];
  setup: Stage;
  attack: Stage;
  timeLimitSec: number;
  pressure: Pressure;
  opponentActions: OpponentAction[];
  readCues: string[];
  situation: string;
  prompt: string;
  options: Choice[];
  principle: string;
}

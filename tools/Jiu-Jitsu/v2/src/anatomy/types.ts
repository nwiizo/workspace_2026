// 解剖モデルの型定義。
// この 1 つのモデルが (1) レンダリングの関節 clamp、(2) 全ポーズの検証テスト、
// (3) 関節ラボの教育コンテンツ、の 3 役を駆動する。

/** リグ上の関節名 (poses のキーと一致する) */
export type RigJointName =
  | "hips"
  | "spine"
  | "chest"
  | "neck"
  | "head"
  | "upperArmL"
  | "forearmL"
  | "handL"
  | "upperArmR"
  | "forearmR"
  | "handR"
  | "thighL"
  | "shinL"
  | "footL"
  | "thighR"
  | "shinR"
  | "footR";

/** 教育対象としての解剖学的関節 ID */
export type AnatomyJointId =
  | "neck"
  | "shoulder"
  | "elbow"
  | "wrist"
  | "hip"
  | "knee"
  | "ankle";

/** 関節の構造タイプ (滑膜関節の分類) */
export type JointKind = "hinge" | "ball-socket" | "pivot" | "condyloid";

export type Axis = "x" | "y" | "z";

/** 1 軸ぶんの運動仕様 */
export interface AxisSpec {
  axis: Axis;
  /** 運動名 (負方向 / 正方向)。例: ["伸展", "屈曲"] */
  motion: [string, string];
  /** 表示リグで許す角度レンジ (度)。ポーズ検証と clamp に使う */
  rigRangeDeg: [number, number];
  /**
   * 解剖学的な参考可動域 (度)。関節ラボの安全域表示に使う。
   * リグは簡易ヒンジ化のため rigRange と一致しないことがある。
   */
  anatomicalRangeDeg: [number, number];
}

/** 対応するサブミッション (この関節の限界を攻める技) */
export interface SubmissionLink {
  name: string;
  how: string;
}

export interface JointSpec {
  id: AnatomyJointId;
  jp: string;
  en: string;
  kind: JointKind;
  kindJp: string;
  /** このスペックが適用されるリグ関節 */
  rigJoints: RigJointName[];
  axes: AxisSpec[];
  /** 可動域を制限している構造 (骨・靭帯・関節包) */
  limitedBy: string;
  /** 可動域を超えると何が壊れるか */
  failureMode: string;
  /** この関節を攻めるサブミッション */
  submissions: SubmissionLink[];
  /** 補足 (例: 絞めは関節でなく血流の技) */
  note?: string;
}

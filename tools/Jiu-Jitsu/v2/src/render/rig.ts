// 骨格トポロジーと体格寸法。旧 fighter.js の資産を型付きで継承する。
// 立ち姿 (全関節 identity) を基準。腕・脚はローカル -Y、胴は +Y に伸びる。

import type { RigJointName } from "../anatomy/types";

/** 体格 (おおよそ身長 1.8 相当) */
export const DIMS = {
  hipY: 0.92,
  spine: 0.2,
  chest: 0.26,
  neck: 0.09,
  headR: 0.125,
  shoulderHalf: 0.185,
  upperArm: 0.27,
  forearm: 0.25,
  hand: 0.1,
  hipHalf: 0.11,
  thigh: 0.42,
  shin: 0.4,
  foot: 0.22,
  limbR: 0.066,
  torsoR: 0.135,
} as const;

export interface BoneDef {
  name: RigJointName;
  parent: RigJointName | null;
  /** 親関節原点からのローカル位置 */
  localPos: readonly [number, number, number];
}

const D = DIMS;

export const SKELETON: readonly BoneDef[] = [
  { name: "hips", parent: null, localPos: [0, 0, 0] },
  { name: "spine", parent: "hips", localPos: [0, 0, 0] },
  { name: "chest", parent: "spine", localPos: [0, D.spine, 0] },
  { name: "neck", parent: "chest", localPos: [0, D.chest, 0] },
  { name: "head", parent: "neck", localPos: [0, D.neck, 0] },

  { name: "upperArmL", parent: "chest", localPos: [D.shoulderHalf, D.chest - 0.03, 0] },
  { name: "forearmL", parent: "upperArmL", localPos: [0, -D.upperArm, 0] },
  { name: "handL", parent: "forearmL", localPos: [0, -D.forearm, 0] },

  { name: "upperArmR", parent: "chest", localPos: [-D.shoulderHalf, D.chest - 0.03, 0] },
  { name: "forearmR", parent: "upperArmR", localPos: [0, -D.upperArm, 0] },
  { name: "handR", parent: "forearmR", localPos: [0, -D.forearm, 0] },

  { name: "thighL", parent: "hips", localPos: [D.hipHalf, 0, 0] },
  { name: "shinL", parent: "thighL", localPos: [0, -D.thigh, 0] },
  { name: "footL", parent: "shinL", localPos: [0, -D.shin, 0] },

  { name: "thighR", parent: "hips", localPos: [-D.hipHalf, 0, 0] },
  { name: "shinR", parent: "thighR", localPos: [0, -D.thigh, 0] },
  { name: "footR", parent: "shinR", localPos: [0, -D.shin, 0] },
] as const;

/** ポーズ: ルート変換 + 関節オイラー角 (ラジアン) */
export interface Pose {
  root?: {
    pos: readonly [number, number, number];
    rot: readonly [number, number, number];
  };
  joints?: Partial<Record<RigJointName, readonly [number, number, number]>>;
}

// 名前付き全身ポーズのライブラリ (旧 poses.js から移植)。角度は度で書き、ラジアンに変換して保持する。
//
// 座標規約:
//   立位: root=(0,0.92,0)・回転 identity・顔は +Z 向き・脊柱は +Y・手脚はローカル -Y。
//   仰向け(頭 -Z): root.rot=[-90,0,0] → 腹(local +Z)が +Y。
//   仰向け(頭 +Z): root.rot=[-90,0,180]。THREE Euler XYZ 合成では反転は Y ではなく Z に置く
//     ([-90,180,0] は 腹が -Y のうつ伏せになる)。中央 Y は「角度切り」(腹を横に転がす roll) に使える。
//   うつ伏せ(頭 +Z): root.rot=[+90,0,0] → 背中が +Y。
//   上の人は原則として下の人の胴上に置き、ポジションの可読性を優先する。

import type { RigJointName } from "../anatomy/types";
import type { Pose } from "./rig";

const deg = (d: number): number => (d * Math.PI) / 180;

type JointDegrees = Partial<Record<RigJointName, readonly [number, number, number]>>;

function P(
  rootPos: readonly [number, number, number],
  rootRotDeg: readonly [number, number, number],
  joints: JointDegrees,
): Pose {
  const rad: Pose["joints"] = {};
  for (const [k, v] of Object.entries(joints)) {
    rad[k as RigJointName] = [deg(v[0]), deg(v[1]), deg(v[2])];
  }
  return {
    root: { pos: rootPos, rot: [deg(rootRotDeg[0]), deg(rootRotDeg[1]), deg(rootRotDeg[2])] },
    joints: rad,
  };
}

// 四肢フラグメント ------------------------------------------------------------
const kneel: JointDegrees = {
  thighL: [-92, 6, 12], shinL: [112, 0, 0],
  thighR: [-92, -6, -12], shinR: [112, 0, 0],
};
const supineBent: JointDegrees = {
  thighL: [34, 0, 8], shinL: [72, 0, 0],
  thighR: [34, 0, -8], shinR: [72, 0, 0],
};
const supineFlat: JointDegrees = {
  thighL: [6, 0, 6], thighR: [6, 0, -6],
};
const guardLegs: JointDegrees = {
  thighL: [112, 12, 58], shinL: [122, -24, -20], footL: [0, -34, 0],
  thighR: [112, -12, -58], shinR: [122, 24, 20], footR: [0, 34, 0],
};

export type PoseName =
  | "standingRed"
  | "standingBlue"
  | "blueSeatedFront"
  | "redBackControl"
  | "blueBackDefend"
  | "blueTapped"
  | "blueUnderMount"
  | "redMountTop"
  | "redMountArmbar"
  | "blueUpaTop"
  | "redRolledBottom"
  | "redClosedGuardBottom"
  | "blueTopInGuard"
  | "blueGuardPass"
  | "redGuardOpened"
  | "redSideControl"
  | "blueUnderSide"
  | "blueShrimpRecover"
  | "redGuardArmbarFinish"
  | "blueGuardArmbarCaught"
  | "blueGivesBack"
  | "redTriangleFinish"
  | "blueCaughtInTriangle";

export const POSES: Record<PoseName, Pose> = {
  // 立ち姿 (礼) --------------------------------------------------------------
  standingRed: P([0.42, 0.92, 0], [0, -90, 0], {
    upperArmL: [4, 0, 6], upperArmR: [4, 0, -6],
  }),
  standingBlue: P([-0.42, 0.92, 0], [0, 90, 0], {
    upperArmL: [4, 0, 6], upperArmR: [4, 0, -6],
  }),

  // 1) バックコントロール: 赤が背後 / 青が前で座位。両者 +Z 向き。 -----------
  blueSeatedFront: P([0, 0.42, 0.2], [10, 0, 0], {
    thighL: [-72, 0, 20], shinL: [62, 0, 0],
    thighR: [-72, 0, -20], shinR: [62, 0, 0],
    upperArmL: [-12, 0, 14], forearmL: [-62, 0, 0],
    upperArmR: [-12, 0, -14], forearmR: [-62, 0, 0],
    neck: [8, 0, 0],
  }),
  redBackControl: P([0, 0.38, -0.08], [12, 0, 0], {
    thighL: [-88, 0, 54], shinL: [108, -18, -12], footL: [0, -20, 0],
    thighR: [-88, 0, -54], shinR: [108, 18, 12], footR: [0, 20, 0],
    upperArmL: [-122, 0, 24], forearmL: [-112, 34, 0],
    upperArmR: [-120, 0, -22], forearmR: [-104, -26, 0],
    neck: [6, 0, 0],
  }),
  blueBackDefend: P([-0.14, 0.42, 0.24], [12, -16, 0], {
    thighL: [-82, 0, 18], shinL: [72, 0, 0],
    thighR: [-82, 0, -18], shinR: [72, 0, 0],
    upperArmL: [-118, 5, 16], forearmL: [-112, 40, 0],
    upperArmR: [-118, -5, -16], forearmR: [-112, -40, 0],
    neck: [24, 0, 0],
  }),
  blueTapped: P([0, 0.42, 0.2], [14, 0, 0], {
    thighL: [-78, 0, 18], shinL: [62, 0, 0],
    thighR: [-78, 0, -18], shinR: [62, 0, 0],
    upperArmL: [-118, 0, 24], forearmL: [-112, 30, 0],
    upperArmR: [60, 0, -18], forearmR: [112, 0, 0],
    neck: [30, 0, 0],
  }),

  // 2) マウント: 青が仰向け下(頭 -Z) / 赤が馬乗り(顔 -Z) ---------------------
  blueUnderMount: P([0, 0.16, 0], [-90, 0, 0], {
    ...supineBent,
    upperArmL: [-30, 0, 22], forearmL: [-95, 0, 0],
    upperArmR: [-30, 0, -22], forearmR: [-95, 0, 0],
    neck: [-14, 0, 0],
  }),
  redMountTop: P([0, 0.5, -0.1], [12, 180, 0], {
    ...kneel,
    upperArmL: [38, 0, 18], forearmL: [54, 0, 0],
    upperArmR: [38, 0, -18], forearmR: [54, 0, 0],
    neck: [8, 0, 0],
  }),
  redMountArmbar: P([0.06, 0.5, -0.09], [22, 180, -8], {
    ...kneel,
    upperArmL: [92, 0, 28], forearmL: [76, 0, 0],
    upperArmR: [42, 0, -14], forearmR: [52, 0, 0],
    neck: [14, 0, 0],
  }),
  blueUpaTop: P([0, 0.48, -0.06], [6, 180, 0], {
    ...kneel,
    upperArmL: [50, 0, 18], forearmL: [34, 0, 0],
    upperArmR: [50, 0, -18], forearmR: [34, 0, 0],
    neck: [2, 0, 0],
  }),
  redRolledBottom: P([0, 0.16, 0.06], [-90, 0, 0], {
    ...supineBent,
    upperArmL: [-26, 0, 18], forearmL: [-70, 0, 0],
    upperArmR: [-26, 0, -18], forearmR: [-70, 0, 0],
    neck: [-10, 0, 0],
  }),

  // 3) クローズドガード: 青が上で膝立ち / 赤が仰向け下(頭 +Z) -----------------
  // 頭+Z の仰向けは [-90,0,180] が正 (旧 [-90,180,0] は腹が下を向くバグ)。
  redClosedGuardBottom: P([0, 0.15, 0.2], [-90, 0, 180], {
    ...guardLegs,
    upperArmL: [-46, 0, 18], forearmL: [-72, 0, 0],
    upperArmR: [-46, 0, -18], forearmR: [-72, 0, 0],
    neck: [-14, 0, 0],
  }),
  blueTopInGuard: P([0, 0.46, -0.04], [10, 0, 0], {
    ...kneel,
    thighL: [-96, 8, 14], shinL: [116, 0, 0],
    thighR: [-96, -8, -14], shinR: [116, 0, 0],
    upperArmL: [-36, 0, 16], forearmL: [-58, 0, 0],
    upperArmR: [-36, 0, -16], forearmR: [-58, 0, 0],
    neck: [4, 0, 0],
  }),
  blueGuardPass: P([0.14, 0.42, -0.06], [6, 40, 0], {
    thighL: [-118, 10, 14], shinL: [120, 0, 0],
    thighR: [-70, -10, -16], shinR: [80, 0, 0],
    upperArmL: [-30, 0, 18], forearmL: [-55, 0, 0],
    upperArmR: [-20, 0, -18], forearmR: [-45, 0, 0],
    neck: [-2, 0, 0],
  }),
  redGuardOpened: P([0, 0.16, 0.16], [-90, 0, 180], {
    ...supineBent,
    upperArmL: [-26, 0, 18], forearmL: [-55, 0, 0],
    upperArmR: [-26, 0, -18], forearmR: [-55, 0, 0],
    neck: [-14, 0, 0],
  }),

  // 4) サイドコントロール: 赤が上で横から抑える / 青が仰向け(頭 -Z) -----------
  redSideControl: P([0.03, 0.25, -0.02], [92, 88, -12], {
    thighL: [-118, 0, 34], shinL: [124, 0, 0],
    thighR: [-50, 0, -36], shinR: [74, 0, 0],
    upperArmL: [-112, 0, 34], forearmL: [-96, 0, 0],
    upperArmR: [-52, 0, -32], forearmR: [-88, 0, 0],
    neck: [-16, 0, 0],
  }),
  blueUnderSide: P([0, 0.15, 0], [-90, 0, 0], {
    ...supineFlat,
    upperArmL: [-68, 0, 40], forearmL: [-100, 0, 0],
    upperArmR: [-50, 0, -38], forearmR: [-94, 0, 0],
    neck: [-12, 0, 0],
  }),
  blueShrimpRecover: P([-0.28, 0.16, -0.06], [-90, 0, -18], {
    thighL: [68, 0, 30], shinL: [105, 0, 0],
    thighR: [34, 0, -10], shinR: [80, 0, 0],
    upperArmL: [-68, 0, 36], forearmL: [-86, 0, 0],
    upperArmR: [-42, 0, -24], forearmR: [-96, 0, 0],
    neck: [-10, 0, 0],
  }),

  // 5) ガード内の極め / 悪手の結果 --------------------------------------------
  // 頭+Z 仰向け [-90,·,180]。中央 Y=角度切り (腹を青側 +X へ 35° 転がす)。
  redGuardArmbarFinish: P([-0.16, 0.16, 0.06], [-90, 35, 180], {
    thighL: [130, 4, 72], shinL: [70, -24, -12], footL: [0, -42, 0],
    thighR: [118, -2, -42], shinR: [124, 24, 10], footR: [0, 28, 0],
    upperArmL: [-124, 0, 30], forearmL: [-116, 0, 0],
    upperArmR: [-102, 0, -24], forearmR: [-110, 0, 0],
    neck: [-12, 0, 0],
  }),
  blueGuardArmbarCaught: P([0.1, 0.24, 0.01], [78, -44, 8], {
    thighL: [-118, 0, 24], shinL: [118, 0, 0],
    thighR: [-112, 0, -18], shinR: [124, 0, 0],
    upperArmL: [112, 0, 26], forearmL: [-4, 0, 0],
    upperArmR: [-84, 0, -22], forearmR: [-106, 0, 0],
    neck: [28, 0, 0],
  }),
  blueGivesBack: P([0, 0.26, 0.12], [82, 0, 0], {
    ...supineBent,
    upperArmL: [-20, 0, 20], upperArmR: [-20, 0, -20],
    neck: [16, 0, 0],
  }),
  // 頭+Z 仰向け [-90,·,180]。中央 Y=角度切り (三角は浅めに 22°)。
  redTriangleFinish: P([-0.1, 0.16, 0.1], [-90, 22, 180], {
    thighL: [132, 10, 70], shinL: [122, -28, -14], footL: [0, -42, 0],
    thighR: [126, -10, -58], shinR: [122, 28, 14], footR: [0, 36, 0],
    upperArmL: [-56, 0, 20], forearmL: [-100, 0, 0],
    upperArmR: [-60, 0, -18], forearmR: [-102, 0, 0],
    neck: [-16, 0, 0],
  }),
  blueCaughtInTriangle: P([0.03, 0.25, 0.04], [82, -18, 2], {
    thighL: [-114, 0, 16], shinL: [124, 0, 0],
    thighR: [-106, 0, -14], shinR: [118, 0, 0],
    upperArmL: [112, 0, 12], forearmL: [-12, 0, 0],
    upperArmR: [-84, 0, -30], forearmR: [-116, 0, 0],
    neck: [28, 0, 0],
  }),
};

export function poseByName(name: PoseName): Pose {
  return POSES[name];
}

// poses.js
// 名前付き全身ポーズのライブラリ。各ポーズは { root?, joints? } 形式。角度は度。
//
// 座標規約:
//   立位: root=(0,0.92,0)・回転 identity・顔は +Z 向き・脊柱は +Y・手脚はローカル -Y。
//   supineHeadMinusZ: root.rot=[-90,0,0] → 頭が -Z、腹(local +Z)が +Y。
//   supineHeadPlusZ:  root.rot=[-90,180,0] → 頭が +Z、腹(local +Z)が +Y。
//   proneHeadPlusZ:   root.rot=[+90,0,0] → 頭が +Z、背中が +Y。
//   上の人は原則として下の人の胴上に置き、複雑な倒れ込みよりポジションの可読性を優先する。

const deg = (d) => (d * Math.PI) / 180;
const P = (root, joints) => ({ root, joints });
const R = (pos, rot = [0, 0, 0]) => ({ pos, rot: rot.map(deg) });
const J = (obj) => {
  const out = {};
  for (const [k, v] of Object.entries(obj)) out[k] = v.map(deg);
  return out;
};

// 四肢フラグメント -----------------------------------------------------------
// 膝立ち(馬乗り/ガード内): 読みやすさ優先で、膝と脛が床に近い形にする。
const kneel = {
  thighL: [-92, 6, 12], shinL: [112, 0, 0],
  thighR: [-92, -6, -12], shinR: [112, 0, 0],
};
// 仰向けで膝を少し立てる。足を上げすぎると局面が読みにくいため控えめにする。
const supineBent = {
  thighL: [34, 0, 8], shinL: [72, 0, 0],
  thighR: [34, 0, -8], shinR: [72, 0, 0],
};
const supineFlat = {
  thighL: [6, 0, 6], thighR: [6, 0, -6],
};
const guardLegs = {
  thighL: [112, 12, 58], shinL: [122, -24, -20], footL: [0, -34, 0],
  thighR: [112, -12, -58], shinR: [122, 24, 20], footR: [0, 34, 0],
};

export const POSES = {
  // 立ち姿 (礼) ---------------------------------------------------------
  standingRed: P(R([0.42, 0.92, 0], [0, -90, 0]), J({
    upperArmL: [4, 0, 6], upperArmR: [4, 0, -6],
  })),
  standingBlue: P(R([-0.42, 0.92, 0], [0, 90, 0]), J({
    upperArmL: [4, 0, 6], upperArmR: [4, 0, -6],
  })),

  // =====================================================================
  // 1) バックコントロール: 赤が背後 / 青(あなた)が前で座位。両者 +Z 向き。
  // =====================================================================
  blueSeatedFront: P(R([0, 0.42, 0.2], [10, 0, 0]), J({
    thighL: [-72, 0, 20], shinL: [62, 0, 0],
    thighR: [-72, 0, -20], shinR: [62, 0, 0],
    upperArmL: [-12, 0, 14], forearmL: [-62, 0, 0],
    upperArmR: [-12, 0, -14], forearmR: [-62, 0, 0],
    neck: [8, 0, 0],
  })),
  redBackControl: P(R([0, 0.38, -0.08], [12, 0, 0]), J({
    thighL: [-88, 0, 54], shinL: [108, -18, -12], footL: [0, -20, 0],
    thighR: [-88, 0, -54], shinR: [108, 18, 12], footR: [0, 20, 0],
    upperArmL: [-122, 0, 24], forearmL: [-112, 34, 0],
    upperArmR: [-120, 0, -22], forearmR: [-104, -26, 0],
    neck: [6, 0, 0],
  })),
  blueBackDefend: P(R([-0.14, 0.42, 0.24], [12, -16, 0]), J({
    thighL: [-82, 0, 18], shinL: [72, 0, 0],
    thighR: [-82, 0, -18], shinR: [72, 0, 0],
    upperArmL: [-118, 5, 16], forearmL: [-112, 40, 0],
    upperArmR: [-118, -5, -16], forearmR: [-112, -40, 0],
    neck: [24, 0, 0],
  })),
  blueTapped: P(R([0, 0.42, 0.2], [14, 0, 0]), J({
    thighL: [-78, 0, 18], shinL: [62, 0, 0],
    thighR: [-78, 0, -18], shinR: [62, 0, 0],
    upperArmL: [-118, 0, 24], forearmL: [-112, 30, 0],
    upperArmR: [60, 0, -18], forearmR: [112, 0, 0],
    neck: [30, 0, 0],
  })),

  // =====================================================================
  // 2) マウント: 青(あなた)が仰向け下(頭 -Z) / 赤が馬乗り(顔 -Z で青を見る)
  // =====================================================================
  blueUnderMount: P(R([0, 0.16, 0], [-90, 0, 0]), J({
    ...supineBent,
    upperArmL: [-30, 0, 22], forearmL: [-95, 0, 0],
    upperArmR: [-30, 0, -22], forearmR: [-95, 0, 0],
    neck: [-14, 0, 0],
  })),
  redMountTop: P(R([0, 0.50, -0.1], [12, 180, 0]), J({
    ...kneel,
    upperArmL: [38, 0, 18], forearmL: [54, 0, 0],
    upperArmR: [38, 0, -18], forearmR: [54, 0, 0],
    neck: [8, 0, 0],
  })),
  // 赤が青の腕を隔離しにくる。倒れ込みは控えめにして「上から腕を取りに行く」形を読ませる。
  redMountArmbar: P(R([0.06, 0.50, -0.09], [22, 180, -8]), J({
    ...kneel,
    upperArmL: [92, 0, 28], forearmL: [76, 0, 0],
    upperArmR: [42, 0, -14], forearmR: [52, 0, 0],
    neck: [14, 0, 0],
  })),
  // 青: アッパ(ブリッジ&ロール)で上を奪取 → 赤のガード内で上に
  blueUpaTop: P(R([0, 0.48, -0.06], [6, 180, 0]), J({
    ...kneel,
    upperArmL: [50, 0, 18], forearmL: [34, 0, 0],
    upperArmR: [50, 0, -18], forearmR: [34, 0, 0],
    neck: [2, 0, 0],
  })),
  redRolledBottom: P(R([0, 0.16, 0.06], [-90, 0, 0]), J({
    ...supineBent,
    upperArmL: [-26, 0, 18], forearmL: [-70, 0, 0],
    upperArmR: [-26, 0, -18], forearmR: [-70, 0, 0],
    neck: [-10, 0, 0],
  })),

  // =====================================================================
  // 3) クローズドガード: 青(あなた)が上で膝立ち / 赤が仰向け下(頭 +Z)
  //    ここでは赤が下なので赤=仰向け頭+Z(rot[+90]だと背が上=うつ伏せなので頭+Z仰向けは
  //    rot[-90]+Y180 で頭+Z & 腹上)。青は赤の腰上(z<0側)で +Z(赤の頭)を向く。
  // =====================================================================
  // 赤: 仰向けで頭+Z・腹上。脚で青の胴を抱える(膝を立て足を青の背側へ)。
  redClosedGuardBottom: P(R([0, 0.15, 0.2], [-90, 180, 0]), J({
    ...guardLegs,
    upperArmL: [-46, 0, 18], forearmL: [-72, 0, 0],
    upperArmR: [-46, 0, -18], forearmR: [-72, 0, 0],
    neck: [-14, 0, 0],
  })),
  // 青: 赤の腰上に膝立ち、+Z(赤の頭側)を向き手で赤の胸/襟を制す
  blueTopInGuard: P(R([0, 0.46, -0.04], [10, 0, 0]), J({
    ...kneel,
    thighL: [-96, 8, 14], shinL: [116, 0, 0],
    thighR: [-96, -8, -14], shinR: [116, 0, 0],
    upperArmL: [-36, 0, 16], forearmL: [-58, 0, 0],
    upperArmR: [-36, 0, -16], forearmR: [-58, 0, 0],
    neck: [4, 0, 0],
  })),
  // 青: ガードを割り片膝を跨がせてパス(横へ回り込む初動)
  blueGuardPass: P(R([0.14, 0.42, -0.06], [6, 40, 0]), J({
    thighL: [-118, 10, 14], shinL: [120, 0, 0],
    thighR: [-70, -10, -16], shinR: [80, 0, 0],
    upperArmL: [-30, 0, 18], forearmL: [-55, 0, 0],
    upperArmR: [-20, 0, -18], forearmR: [-45, 0, 0],
    neck: [-2, 0, 0],
  })),
  redGuardOpened: P(R([0, 0.16, 0.16], [-90, 180, 0]), J({
    ...supineBent,
    upperArmL: [-26, 0, 18], forearmL: [-55, 0, 0],
    upperArmR: [-26, 0, -18], forearmR: [-55, 0, 0],
    neck: [-14, 0, 0],
  })),

  // =====================================================================
  // 4) サイドコントロール: 赤が上で横から抑える / 青(あなた)が仰向け(頭 -Z)
  //    赤は青の胸を横切り、胸で胸を潰す(root.rot=[0,90,0]+前傾で青の上に伏せる)。
  // =====================================================================
  redSideControl: P(R([0.03, 0.25, -0.02], [92, 88, -12]), J({
    thighL: [-118, 0, 34], shinL: [124, 0, 0],
    thighR: [-50, 0, -36], shinR: [74, 0, 0],
    upperArmL: [-112, 0, 34], forearmL: [-96, 0, 0],
    upperArmR: [-52, 0, -32], forearmR: [-88, 0, 0],
    neck: [-16, 0, 0],
  })),
  blueUnderSide: P(R([0, 0.15, 0], [-90, 0, 0]), J({
    ...supineFlat,
    upperArmL: [-68, 0, 40], forearmL: [-100, 0, 0],
    upperArmR: [-50, 0, -38], forearmR: [-94, 0, 0],
    neck: [-12, 0, 0],
  })),
  blueShrimpRecover: P(R([-0.28, 0.16, -0.06], [-90, 0, -18]), J({
    thighL: [68, 0, 30], shinL: [105, 0, 0],
    thighR: [34, 0, -10], shinR: [80, 0, 0],
    upperArmL: [-68, 0, 36], forearmL: [-86, 0, 0],
    upperArmR: [-42, 0, -24], forearmR: [-96, 0, 0],
    neck: [-10, 0, 0],
  })),
  // 攻め版サイド: 青(あなた)が上で抑える / 赤が仰向け下。連続ロールの攻めパートで使用。
  blueSideControl: P(R([0.03, 0.25, -0.02], [92, 88, -12]), J({
    thighL: [-118, 0, 34], shinL: [124, 0, 0],
    thighR: [-50, 0, -36], shinR: [74, 0, 0],
    upperArmL: [-112, 0, 34], forearmL: [-96, 0, 0],
    upperArmR: [-52, 0, -32], forearmR: [-88, 0, 0],
    neck: [-16, 0, 0],
  })),
  redUnderSide: P(R([0, 0.15, 0], [-90, 0, 0]), J({
    ...supineFlat,
    upperArmL: [-68, 0, 40], forearmL: [-100, 0, 0],
    upperArmR: [-50, 0, -38], forearmR: [-94, 0, 0],
    neck: [-12, 0, 0],
  })),
  // 青: サイド→ニーオンベリー→マウントへ上る
  blueClimbToMount: P(R([0, 0.50, -0.1], [12, 180, 0]), J({
    ...kneel,
    upperArmL: [36, 0, 18], forearmL: [52, 0, 0],
    upperArmR: [36, 0, -18], forearmR: [52, 0, 0],
    neck: [6, 0, 0],
  })),
  redUnderMount: P(R([0, 0.16, 0], [-90, 0, 0]), J({
    ...supineBent,
    upperArmL: [-30, 0, 22], forearmL: [-90, 0, 0],
    upperArmR: [-30, 0, -22], forearmR: [-90, 0, 0],
    neck: [-12, 0, 0],
  })),

  // =====================================================================
  // 5) マウントから腕を隔離 → 腕十字 (青=攻 / 赤=下・頭 -Z)
  // =====================================================================
  blueIsolateArm: P(R([-0.04, 0.50, -0.1], [16, 180, 0]), J({
    ...kneel,
    upperArmL: [78, 0, 28], forearmL: [70, 0, 0],
    upperArmR: [36, 0, -16], forearmR: [50, 0, 0],
    neck: [8, 0, 0],
  })),
  redArmIsolated: P(R([0, 0.16, 0], [-90, 0, 0]), J({
    ...supineBent,
    upperArmL: [-90, 0, 10], forearmL: [-26, 0, 0],
    upperArmR: [-30, 0, -22], forearmR: [-60, 0, 0],
    neck: [-12, 0, 0],
  })),
  // 青: アームバー完成(赤の片腕を伸ばし極める)
  blueArmbarFinish: P(R([0.12, 0.34, 0.12], [34, 180, 8]), J({
    thighL: [-92, 12, 10], shinL: [44, 0, 0],
    thighR: [-82, 24, -2], shinR: [58, 0, 0],
    upperArmL: [54, 0, 20], forearmL: [54, 0, 0],
    upperArmR: [40, 0, -16], forearmR: [44, 0, 0],
    neck: [8, 0, 0],
  })),
  redArmbarCaught: P(R([0, 0.16, -0.04], [-90, 0, 0]), J({
    ...supineFlat,
    upperArmL: [-118, 0, 8], forearmL: [-10, 0, 0],
    upperArmR: [-30, 0, -20], forearmR: [-120, 0, 0],
    neck: [-16, 0, 0],
  })),

  // 赤: クローズドガード下から角度を切り、青の片腕を脚で挟んで伸ばす
  redGuardArmbarFinish: P(R([-0.16, 0.16, 0.06], [-90, 180, -66]), J({
    thighL: [130, 4, 72], shinL: [70, -24, -12], footL: [0, -42, 0],
    thighR: [118, -2, -42], shinR: [124, 24, 10], footR: [0, 28, 0],
    upperArmL: [-124, 0, 30], forearmL: [-116, 0, 0],
    upperArmR: [-102, 0, -24], forearmR: [-110, 0, 0],
    neck: [-12, 0, 0],
  })),
  // 青: 姿勢を折られ、片腕を中心線から外されている
  blueGuardArmbarCaught: P(R([0.1, 0.24, 0.01], [78, -44, 8]), J({
    thighL: [-118, 0, 24], shinL: [118, 0, 0],
    thighR: [-112, 0, -18], shinR: [124, 0, 0],
    upperArmL: [112, 0, 26], forearmL: [-4, 0, 0],
    upperArmR: [-84, 0, -22], forearmR: [-106, 0, 0],
    neck: [28, 0, 0],
  })),

  // =====================================================================
  // おまけ: 悪手の結果
  // =====================================================================
  blueGivesBack: P(R([0, 0.26, 0.12], [82, 0, 0]), J({
    ...supineBent,
    upperArmL: [-20, 0, 20], upperArmR: [-20, 0, -20],
    neck: [16, 0, 0],
  })),
  redTriangleFinish: P(R([-0.1, 0.16, 0.1], [-90, 180, -38]), J({
    thighL: [132, 10, 70], shinL: [122, -28, -14], footL: [0, -42, 0],
    thighR: [126, -10, -58], shinR: [122, 28, 14], footR: [0, 36, 0],
    upperArmL: [-56, 0, 20], forearmL: [-100, 0, 0],
    upperArmR: [-60, 0, -18], forearmR: [-102, 0, 0],
    neck: [-16, 0, 0],
  })),
  blueCaughtInTriangle: P(R([0.03, 0.25, 0.04], [82, -18, 2]), J({
    thighL: [-114, 0, 16], shinL: [124, 0, 0],
    thighR: [-106, 0, -14], shinR: [118, 0, 0],
    upperArmL: [112, 0, 12], forearmL: [-12, 0, 0],
    upperArmR: [-84, 0, -30], forearmR: [-116, 0, 0],
    neck: [28, 0, 0],
  })),
};

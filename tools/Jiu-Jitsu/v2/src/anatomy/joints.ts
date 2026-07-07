// 関節の解剖モデル本体。
// rigRangeDeg は旧 anatomy.js の保守的な表示用レンジを継承し、
// anatomicalRangeDeg は標準的なキネシオロジーの参考可動域 (健常成人・他動でない範囲の目安)。
// 「なぜその技でタップするのか」を構造から説明するための教育データでもある。

import type { AnatomyJointId, Axis, JointSpec, RigJointName } from "./types";

export const JOINTS: JointSpec[] = [
  {
    id: "neck",
    jp: "頸椎 (首)",
    en: "Cervical spine",
    kind: "pivot",
    kindJp: "車軸関節 + 椎間関節の連鎖",
    rigJoints: ["neck"],
    axes: [
      {
        axis: "x",
        motion: ["伸展 (上を向く)", "屈曲 (顎を引く)"],
        rigRangeDeg: [-35, 35],
        anatomicalRangeDeg: [-60, 50],
      },
      {
        axis: "y",
        motion: ["左回旋", "右回旋"],
        rigRangeDeg: [-30, 30],
        anatomicalRangeDeg: [-80, 80],
      },
      {
        axis: "z",
        motion: ["左側屈", "右側屈"],
        rigRangeDeg: [-30, 30],
        anatomicalRangeDeg: [-45, 45],
      },
    ],
    limitedBy: "椎骨の形状、項靭帯・翼状靭帯、椎間板",
    failureMode:
      "過屈曲・過伸展で靭帯損傷や椎間板・神経の圧迫。ただし絞め技の主対象は関節ではなく頸動脈の血流",
    submissions: [
      {
        name: "裸絞め / mata-leão",
        how: "関節技ではなく血流の技。前腕で両側の頸動脈を圧迫し、数秒で脳血流を落とす。顎を引いて腕の挿入を防ぐのが第一防御",
      },
      {
        name: "三角絞め / triângulo",
        how: "自分の脚と相手自身の肩で首の両側を挟む。片腕が中に残る構造が絞めの条件",
      },
    ],
    note:
      "「首を守る」は柔術防御の最優先。絞めは関節破壊より速く意識を落とすため、タップは早めに。",
  },
  {
    id: "shoulder",
    jp: "肩関節 (肩甲上腕関節)",
    en: "Glenohumeral joint",
    kind: "ball-socket",
    kindJp: "球関節 — 人体で最も可動域が広く、最も脱臼しやすい",
    rigJoints: ["upperArmL", "upperArmR"],
    axes: [
      {
        axis: "x",
        motion: ["屈曲 (腕を前へ上げる)", "伸展 (腕を後ろへ)"],
        rigRangeDeg: [-125, 125],
        anatomicalRangeDeg: [-180, 60],
      },
      {
        axis: "y",
        motion: ["外旋", "内旋"],
        rigRangeDeg: [-70, 70],
        anatomicalRangeDeg: [-90, 70],
      },
      {
        axis: "z",
        motion: ["内転 (体側へ)", "外転 (横へ上げる)"],
        rigRangeDeg: [-80, 80],
        anatomicalRangeDeg: [-45, 180],
      },
    ],
    limitedBy: "関節唇、関節包、回旋筋腱板 (ローテーターカフ)",
    failureMode:
      "外旋・伸展の限界を超えると関節包前面と腱板が損傷し、亜脱臼・脱臼に至る",
    submissions: [
      {
        name: "キムラ / 腕緘 (うでがらみ)",
        how: "肘を 90° に固定したまま前腕をテコに肩を強制内旋→背面へ回す。サイド下で腕を相手の下へ差すと入口を渡す",
      },
      {
        name: "アメリカーナ",
        how: "キムラの逆方向。腕を頭側へ倒して肩を外旋・伸展させる",
      },
    ],
  },
  {
    id: "elbow",
    jp: "肘関節 (腕尺関節)",
    en: "Elbow (humeroulnar joint)",
    kind: "hinge",
    kindJp: "蝶番 (ヒンジ) 関節 — 曲げ伸ばしの 1 自由度が主",
    rigJoints: ["forearmL", "forearmR"],
    axes: [
      {
        axis: "x",
        motion: ["伸展 (伸ばす)", "屈曲 (曲げる)"],
        rigRangeDeg: [-120, 120],
        anatomicalRangeDeg: [0, 145],
      },
      {
        axis: "y",
        motion: ["回外", "回内"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-85, 80],
      },
      {
        axis: "z",
        motion: ["内反", "外反"],
        rigRangeDeg: [-35, 35],
        anatomicalRangeDeg: [-5, 5],
      },
    ],
    limitedBy: "肘頭が肘頭窩に骨性に当たって伸展 0° で止まる。内外反は側副靭帯が抑える",
    failureMode:
      "伸展 0° を超える過伸展で内側側副靭帯・関節包が損傷し、肘頭の骨性衝突に至る。競技傷害の最多部位 (整形外科的傷害の 38.9% が肘、その大半が腕十字)",
    submissions: [
      {
        name: "腕十字 / 十字固め / juji-gatame",
        how: "腰を支点に相手の腕全体をテコで伸ばし、伸展 0° の骨性ロックを越えさせる。「肘を体から離さない」が防御の核",
      },
    ],
    note: "ヒンジ関節は自由度が少ないぶん、限界方向への力に構造的な逃げ場がない。",
  },
  {
    id: "wrist",
    jp: "手関節 (手首)",
    en: "Wrist (radiocarpal joint)",
    kind: "condyloid",
    kindJp: "顆状関節 — 2 自由度 (掌背屈・橈尺屈)",
    rigJoints: ["handL", "handR"],
    axes: [
      {
        axis: "x",
        motion: ["背屈 (甲側へ)", "掌屈 (手のひら側へ)"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-70, 80],
      },
      {
        axis: "y",
        motion: ["回外方向", "回内方向"],
        rigRangeDeg: [-45, 45],
        anatomicalRangeDeg: [-10, 10],
      },
      {
        axis: "z",
        motion: ["橈屈 (親指側へ)", "尺屈 (小指側へ)"],
        rigRangeDeg: [-45, 45],
        anatomicalRangeDeg: [-20, 30],
      },
    ],
    limitedBy: "手根骨の配列と掌側・背側の靭帯群",
    failureMode: "過度の掌背屈で靭帯損傷。リストロックは小さい力で速く極まる",
    note:
      "回内・回外は本来、前腕の橈尺関節の運動。簡易リグでは手首の y 軸にまとめて表現している。手関節自体の自由度は掌背屈・橈尺屈の 2 つ。",
    submissions: [
      {
        name: "リストロック",
        how: "掌屈または背屈方向へ手首を折り込む。ノーギのハンドファイトで露出しやすい",
      },
    ],
  },
  {
    id: "hip",
    jp: "股関節",
    en: "Hip joint",
    kind: "ball-socket",
    kindJp: "球関節 — 肩より深い臼蓋で安定性重視",
    rigJoints: ["thighL", "thighR"],
    axes: [
      {
        axis: "x",
        motion: ["伸展 (脚を後ろへ)", "屈曲 (膝を胸へ)"],
        rigRangeDeg: [-130, 145],
        anatomicalRangeDeg: [-30, 120],
      },
      {
        axis: "y",
        motion: ["外旋", "内旋"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-45, 45],
      },
      {
        axis: "z",
        motion: ["内転", "外転"],
        rigRangeDeg: [-80, 80],
        anatomicalRangeDeg: [-30, 45],
      },
    ],
    limitedBy: "深い臼蓋 (寛骨臼)、腸骨大腿靭帯 — 人体最強の靭帯",
    failureMode: "単体で極められることは少ないが、腰の角度が消えると上の関節が全て守れなくなる",
    submissions: [
      {
        name: "(直接のサブミッションは少ない)",
        how: "股関節は「攻められる関節」ではなく「技の動力源」。ブリッジ・海老・角度切りはすべて股関節の可動域で作る",
      },
    ],
    note: "ガードワークとは股関節の可動域の使い方のこと。柔術で最初に柔らかくすべき関節。",
  },
  {
    id: "knee",
    jp: "膝関節",
    en: "Knee joint",
    kind: "hinge",
    kindJp: "蝶番 (ヒンジ) 関節 — 屈曲時のみわずかに回旋",
    rigJoints: ["shinL", "shinR"],
    axes: [
      {
        axis: "x",
        motion: ["伸展 (伸ばす)", "屈曲 (曲げる)"],
        rigRangeDeg: [0, 150],
        anatomicalRangeDeg: [0, 135],
      },
      {
        axis: "y",
        motion: ["内旋", "外旋"],
        rigRangeDeg: [-30, 30],
        anatomicalRangeDeg: [-10, 30],
      },
      {
        axis: "z",
        motion: ["内反", "外反"],
        rigRangeDeg: [-25, 25],
        anatomicalRangeDeg: [-5, 5],
      },
    ],
    limitedBy: "前後十字靭帯 (ACL/PCL)、内外側側副靭帯 (MCL/LCL)、半月板",
    failureMode:
      "過伸展や捻りで靭帯・半月板が損傷する。痛みより先に構造が壊れることがあり「痛くなってからタップ」では遅い",
    submissions: [
      {
        name: "膝十字 / ヒールフック (本道場では扱わない)",
        how: "下半身関節技は位置支配がなくても極まりやすく (Spanias 2022)、損傷リスクが高いため上級者の領域。本道場は上半身の攻防に限定",
      },
    ],
    note: "リグの脛 (shin) は x=0..150 のみ許可 — 逆関節に見える負方向屈曲を構造で禁止している。",
  },
  {
    id: "ankle",
    jp: "足関節 (足首)",
    en: "Ankle joint",
    kind: "hinge",
    kindJp: "蝶番 (ヒンジ) 関節 (距腿関節)",
    rigJoints: ["footL", "footR"],
    axes: [
      {
        axis: "x",
        motion: ["背屈 (つま先を上げる)", "底屈 (つま先を伸ばす)"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-20, 50],
      },
      {
        axis: "y",
        motion: ["内転", "外転"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-35, 25],
      },
      {
        axis: "z",
        motion: ["内がえし", "外がえし"],
        rigRangeDeg: [-55, 55],
        anatomicalRangeDeg: [-35, 20],
      },
    ],
    limitedBy: "距骨のほぞ穴構造 (脛骨・腓骨に挟まれる)、三角靭帯・外側靭帯群",
    failureMode: "過底屈 + 内がえしでアキレス腱と靭帯にストレス (アンクルロックの原理)",
    submissions: [
      {
        name: "アキレス固め (本道場では扱わない)",
        how: "底屈方向へ足首を伸ばし込みつつアキレス腱を前腕で圧迫する入門的足関節技",
      },
    ],
  },
];

const JOINT_BY_ID = new Map(JOINTS.map((j) => [j.id, j]));
const SPEC_BY_RIG_JOINT = new Map<RigJointName, JointSpec>();
for (const spec of JOINTS) {
  for (const rig of spec.rigJoints) SPEC_BY_RIG_JOINT.set(rig, spec);
}

export function jointById(id: AnatomyJointId): JointSpec {
  const spec = JOINT_BY_ID.get(id);
  if (!spec) throw new Error(`unknown joint: ${id}`);
  return spec;
}

/** リグ関節名 → 解剖スペック (胴・頭など対象外は undefined) */
export function specForRigJoint(rig: RigJointName): JointSpec | undefined {
  return SPEC_BY_RIG_JOINT.get(rig);
}

const AXES: readonly Axis[] = ["x", "y", "z"];
const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));
export const deg = (d: number): number => (d * Math.PI) / 180;
export const radToDeg = (r: number): number => (r * 180) / Math.PI;

/** リグ関節のオイラー角 (ラジアン) を表示可動域に clamp する */
export function clampJointEuler(
  rig: RigJointName,
  radians: readonly [number, number, number],
): [number, number, number] {
  const spec = SPEC_BY_RIG_JOINT.get(rig);
  if (!spec) return [radians[0], radians[1], radians[2]];
  const out: [number, number, number] = [radians[0], radians[1], radians[2]];
  for (const axisSpec of spec.axes) {
    const i = AXES.indexOf(axisSpec.axis);
    const [min, max] = axisSpec.rigRangeDeg;
    out[i] = deg(clamp(radToDeg(out[i]), min, max));
  }
  return out;
}

/** ポーズ検証用: 可動域外の軸を列挙する */
export function jointLimitViolations(
  poseId: string,
  rig: RigJointName,
  radians: readonly [number, number, number],
): string[] {
  const spec = SPEC_BY_RIG_JOINT.get(rig);
  if (!spec) return [];
  const violations: string[] = [];
  for (const axisSpec of spec.axes) {
    const i = AXES.indexOf(axisSpec.axis);
    const degrees = radToDeg(radians[i]);
    const [min, max] = axisSpec.rigRangeDeg;
    if (degrees < min - 1e-6 || degrees > max + 1e-6) {
      violations.push(
        `${poseId}:${rig}.${axisSpec.axis}=${degrees.toFixed(1)} outside ${min}..${max}`,
      );
    }
  }
  return violations;
}

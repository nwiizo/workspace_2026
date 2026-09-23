import { Vector3 } from "three";
import { practiceLesson } from "../content/practice";
import { bodyFrame, kneelingLeg, solveBody, type BodyPose, type BodyTargets, type Landmark, type LimbTarget } from "./mannequin";

export type Actor = "blue" | "red";
export interface PracticeCue { label: string; point: Vector3; kind: "floor" | "contact" | "space" }
export interface PracticePair {
  blue: BodyPose;
  red: BodyPose;
  supports: { actor: Actor; joint: Landmark; height: number }[];
  cues: PracticeCue[];
}
const v = (x: number, y: number, z: number) => new Vector3(x, y, z);
const limb = (end: Vector3, pole: Vector3): LimbTarget => ({ end, pole });

export function supine(x = 0, z = 0.25, roll = false): BodyTargets {
  return {
    pelvis: v(x, roll ? 0.18 : 0.12, z), up: v(roll ? 0.22 : 0, 0, -1), front: v(roll ? 0.55 : 0, 1, 0),
    arms: {
      L: limb(v(x + 0.32, 0.10, z - 0.07), v(x + 0.42, 0.15, z - 0.20)),
      R: limb(v(x - 0.32, 0.10, z - 0.07), v(x - 0.42, 0.15, z - 0.20)),
    },
    legs: {
      L: limb(v(x + 0.22, 0.055, z + 0.63), v(x + 0.22, 0.47, z + 0.27)),
      R: limb(v(x - 0.22, 0.055, z + 0.63), v(x - 0.22, 0.47, z + 0.27)),
    },
  };
}

export function kneeling(pelvis: Vector3, up = v(0, 0.98, -0.2), front = v(0, -0.2, -1)): BodyTargets {
  const { point: p } = bodyFrame(pelvis, up, front);
  const forward = front.clone().setY(0);
  if (forward.length() < 0.5) forward.copy(up).setY(0);
  forward.normalize();
  const lateral = p(1, 0, 0).sub(pelvis).setY(0).normalize();
  const heels = forward.clone().negate();
  return {
    pelvis, up, front,
    arms: { L: limb(p(0.17, 0.06, 0.22), p(0.32, 0.20, 0.05)), R: limb(p(-0.17, 0.06, 0.22), p(-0.32, 0.20, 0.05)) },
    legs: {
      L: kneelingLeg(p(0.11, 0, 0), lateral.clone().multiplyScalar(0.5).addScaledVector(forward, 0.20), heels),
      R: kneelingLeg(p(-0.11, 0, 0), lateral.clone().multiplyScalar(-0.5).addScaledVector(forward, 0.20), heels),
    },
  };
}

export function pair(top: BodyTargets, bottom: BodyTargets, topActor: Actor): PracticePair {
  const a = solveBody(top), b = solveBody(bottom);
  const bottomActor: Actor = topActor === "red" ? "blue" : "red";
  return {
    blue: topActor === "blue" ? a : b,
    red: topActor === "red" ? a : b,
    supports: [
      { actor: topActor, joint: "kneeL", height: 0.073 }, { actor: topActor, joint: "kneeR", height: 0.073 },
      { actor: topActor, joint: "ankleL", height: 0.055 }, { actor: topActor, joint: "ankleR", height: 0.055 },
      ...(["L", "R"] as const).filter((side) => bottom.legs[side].end.y === 0.055).map((side) => ({ actor: bottomActor, joint: `ankle${side}` as const, height: 0.055 })),
    ], cues: [],
  };
}

export function sidePair(node: string, topActor: Actor): PracticePair {
  const recovering = ["space", "shield", "blocked-path", "open"].includes(node);
  const shield = node === "blocked-path";
  const framed = ["framed", "space", "blocked-path", "shield"].includes(node);
  const bottom = supine(recovering ? -0.23 : 0, 0.25, recovering);
  const top = kneeling(v(0.46, framed ? 0.45 : 0.415, -0.07), v(-1, -0.20, 0), v(0.2, -1, 0));
  // 横から胸を重ね、両膝は相手の外側のマットへ置く。
  top.arms.L = limb(v(-0.21, 0.11, 0.16), v(-0.14, 0.18, 0.32));
  top.arms.R = limb(v(-0.20, 0.11, -0.36), v(-0.12, 0.19, -0.49));
  if (recovering) {
    top.arms.L = limb(v(-0.12, 0.30, 0.24), v(0.04, 0.26, 0.36));
    top.arms.R = limb(v(-0.35, 0.13, -0.37), v(-0.20, 0.22, -0.62));
  }
  if (framed) {
    bottom.arms.L = limb(v(0.33, 0.31, 0.03), v(0.12, 0.18, 0.16));
    bottom.arms.R = limb(v(0.02, 0.30, -0.31), v(-0.38, 0.17, -0.27));
    if (recovering) bottom.arms.R = limb(v(-0.13, 0.40, -0.17), v(-0.45, 0.24, -0.10));
  } else if (node === "pressured") {
    bottom.arms.L = limb(v(0.32, 0.25, 0.04), v(0.44, 0.22, -0.10));
  } else if (node === "heavy") {
    top.arms.L = limb(v(0.12, 0.22, 0.23), v(-0.06, 0.23, 0.35));
  }
  if (shield) bottom.legs.L = limb(v(0.44, 0.27, 0.57), v(0.10, 0.48, 0.17));
  if (["clear-path", "side-held"].includes(node)) top.arms.L = limb(v(0.13, 0.20, 0.25), v(0.05, 0.26, 0.32));
  const result = pair(top, bottom, topActor);
  const b = topActor === "red" ? result.blue : result.red;
  result.cues.push({ label: framed ? "前腕で支える" : "胸の重なり", kind: "contact", point: framed ? b.wristR : b.chest.clone().add(v(0.06, 0.12, 0)) });
  if (recovering) result.cues.push({ label: shield ? "膝が間に入った" : "腰を離した空間", kind: "space", point: shield ? b.kneeL : b.pelvis.clone().add(v(0.23, 0.05, 0)) });
  return result;
}

export function mountPair(node: string, topActor: Actor): PracticePair {
  const high = ["high", "read-high"].includes(node);
  const escaping = node === "knee-space";
  const bottom = supine(escaping ? -0.17 : 0, 0.25, escaping);
  if (escaping) bottom.up = v(0.60, 0, -1);
  const top = kneeling(v(0, 0.40, high || escaping ? -0.09 : 0.20));
  if (["low", "high"].includes(node)) {
    bottom.arms.L = limb(v(0.34, 0.28, -0.32), v(0.38, 0.14, -0.15));
    bottom.arms.R = limb(v(-0.34, 0.28, -0.32), v(-0.38, 0.14, -0.15));
  } else {
    bottom.arms.L = limb(v(0.17, 0.30, -0.08), v(0.22, 0.14, 0.10));
    bottom.arms.R = limb(v(-0.17, 0.30, -0.08), v(-0.22, 0.14, 0.10));
  }
  if (node === "trapped") {
    top.arms.R = limb(v(0.20, 0.34, -0.07), v(0.37, 0.40, 0.06));
    bottom.arms.L = limb(v(0.21, 0.34, -0.06), v(0.23, 0.15, 0.10));
    bottom.legs.L = limb(top.legs.R.end.clone().add(v(0.09, 0, 0.01)), v(0.34, 0.45, 0.47));
  }
  if (node === "isolated") {
    bottom.arms.L = limb(v(0.23, 0.49, -0.10), v(0.32, 0.30, -0.25));
    top.arms.L = limb(v(0.20, 0.49, -0.10), v(-0.20, 0.62, -0.08));
    top.arms.R = limb(v(0.26, 0.49, -0.10), v(0.36, 0.61, -0.01));
  }
  if (node === "mounted") {
    bottom.pelvis.y = 0.18;
    bottom.up = v(-0.13, -0.07, -1);
    top.arms.L = limb(v(-0.42, 0.22, -0.07), v(-0.42, 0.49, 0.14));
    top.arms.R = limb(v(0.42, 0.22, -0.07), v(0.42, 0.49, 0.14));
  }
  if (escaping) {
    bottom.legs.L = limb(v(0.25, 0.19, 0.53), v(0.06, 0.45, 0.20));
    bottom.arms.R.pole = v(-0.42, 0.20, 0.08);
  }
  const result = pair(top, bottom, topActor);
  if (node === "isolated") {
    result.cues.push({ label: "片腕が体から離れる", point: result[topActor === "red" ? "blue" : "red"].wristL, kind: "contact" });
    return result;
  }
  result.cues.push({ label: escaping ? "膝を内側へ戻す" : high ? "膝が胸側へ移動" : node === "trapped" ? "同じ側の手足を止める" : "両膝を床につく", point: escaping ? result.blue.kneeL : node === "trapped" ? result[topActor].wristR : result[topActor].kneeR, kind: escaping || node === "trapped" ? "contact" : "floor" });
  return result;
}

export function guardPair(node: string, topActor: Actor): PracticePair {
  const pulled = ["pull", "base-pull"].includes(node);
  const angled = ["angle", "base-angle"].includes(node);
  const bottom = supine(angled ? -0.14 : 0, 0.24);
  if (angled) bottom.up = v(0.38, 0, -1);
  const top = kneeling(v(0, 0.38, 0.54), pulled ? v(0, 0.40, -0.916) : v(0, 0.99, -0.14));
  bottom.legs.L = limb(v(-0.035, 0.48, 0.74), v(0.60, 0.44, 0.50));
  bottom.legs.R = limb(v(0.035, 0.53, 0.74), v(-0.60, 0.44, 0.50));
  const frame = bodyFrame(top.pelvis, top.up, top.front);
  bottom.arms.L = limb(pulled ? frame.point(0.04, 0.49, 0.07) : v(0.16, 0.37, 0.09), v(0.34, 0.26, -0.09));
  bottom.arms.R = limb(v(-0.13, 0.37, 0.09), v(-0.33, 0.26, -0.09));
  top.arms.L = limb(v(-0.16, 0.30, 0.22), v(-0.26, 0.49, 0.48));
  top.arms.R = limb(v(node === "stable" ? 0.16 : 0.30, 0.30, 0.22), v(node === "stable" ? 0.26 : 0.39, 0.49, 0.48));
  if (["pull", "angle"].includes(node)) top.legs.L = kneelingLeg(frame.point(0.11, 0, 0), v(-0.55, 0, -0.7), v(0, 0, 1));
  const result = pair(top, bottom, topActor);
  const b = topActor === "blue" ? result.red : result.blue;
  result.cues.push({ label: "脚が腰を囲う", kind: "contact", point: b.kneeL });
  result.cues.push({ label: angled ? "腰が横へずれた" : pulled ? "上体が前へ傾く" : "上体を起こす", kind: "space", point: angled ? b.pelvis : result[topActor].chest });
  return result;
}

export function halfPair(node: string, topActor: Actor): PracticePair {
  const shield = node === "shield";
  const bottom = supine(-0.14, 0.24, node !== "flat");
  const top = kneeling(v(0.18, 0.40, 0.51), v(0, 0.65, -0.76));
  bottom.legs.R = limb(v(-0.05, 0.055, 0.82), v(-0.32, 0.20, 0.45));
  bottom.legs.L = limb(v(0.24, shield ? 0.28 : 0.17, shield ? 0.34 : 0.54), v(-0.02, shield ? 0.48 : 0.25, 0.42));
  if (node !== "flat") {
    bottom.arms.L = limb(v(0.16, 0.42, 0.05), v(0.16, 0.22, -0.05));
    bottom.arms.R = limb(v(-0.12, 0.36, -0.10), v(-0.45, 0.20, -0.12));
  }
  top.arms.L = limb(v(-0.25, 0.28, -0.03), v(-0.24, 0.42, 0.13));
  top.arms.R = limb(v(0.23, 0.32, 0.24), v(0.46, 0.46, 0.29));
  const result = pair(top, bottom, topActor);
  const b = result[topActor === "red" ? "blue" : "red"];
  result.cues = [{ label: shield ? "上の膝で距離を保つ" : "膝の壁が低い", point: b.kneeL, kind: "contact" }, { label: "片脚を両脚の間に残す", point: result[topActor].kneeL, kind: "contact" }];
  return result;
}

export function turtlePair(node: string, topActor: Actor): PracticePair {
  const bottom = kneeling(v(0, 0.35, 0.25), v(0, 0, -1), v(0, -1, 0));
  const frame = bodyFrame(bottom.pelvis, bottom.up, bottom.front);
  bottom.legs.L = kneelingLeg(frame.point(0.11, 0, 0), v(-0.12, 0, -0.6), v(0, 0, 1));
  bottom.legs.R = kneelingLeg(frame.point(-0.11, 0, 0), v(0.12, 0, -0.6), v(0, 0, 1));
  bottom.arms.L = limb(v(-0.24, 0.046, -0.43), v(-0.26, 0.16, -0.22));
  bottom.arms.R = limb(v(0.24, 0.046, -0.43), v(0.26, 0.16, -0.22));
  const top = kneeling(v(0.48, 0.40, 0.22), v(-0.35, 0.88, -0.30), v(-1, -0.4, 0));
  top.arms.L = limb(v(0.02, 0.49, node === "control" ? 0.30 : 0.23), v(0.19, 0.55, 0.34));
  top.arms.R = limb(v(0.02, 0.50, node === "reach" ? -0.22 : -0.06), v(0.23, 0.58, -0.25));
  if (node === "protected") bottom.arms.R = limb(v(0.16, 0.36, -0.27), v(0.31, 0.19, -0.16));
  const result = pair(top, bottom, topActor);
  const actor = topActor === "red" ? "blue" : "red";
  result.supports.push({ actor, joint: "kneeL", height: 0.073 }, { actor, joint: "kneeR", height: 0.073 }, { actor, joint: "wristL", height: 0.046 });
  if (node !== "protected") result.supports.push({ actor, joint: "wristR", height: 0.046 });
  result.cues = [{ label: "両膝と手で支える", point: result[actor].kneeL, kind: "floor" }, { label: node === "reach" ? "首へ向かう手を見る" : "腰の横から位置を保つ", point: result[topActor][node === "reach" ? "wristR" : "wristL"], kind: "contact" }];
  return result;
}

export function openPair(node: string, topActor: Actor): PracticePair {
  const offset = node === "square" ? 0 : 0.24;
  const bottom = supine();
  bottom.legs.L = limb(v(0.43, 0.055, 0.89), v(0.44, 0.43, 0.47));
  bottom.legs.R = limb(v(-0.43, 0.055, 0.89), v(-0.44, 0.43, 0.47));
  const top = kneeling(v(offset, 0.38, 0.57));
  top.arms.L = limb(v(offset - 0.31, 0.30, 0.30), v(offset - 0.41, 0.48, 0.51));
  top.arms.R = limb(v(offset + 0.19, 0.28, 0.47), v(offset + 0.31, 0.48, 0.48));
  const result = pair(top, bottom, topActor);
  result.cues = [{ label: "足を交差せず膝の内側を見る", point: result[topActor === "red" ? "blue" : "red"].kneeL, kind: "space" }];
  return result;
}

function seated(x: number, z: number): BodyTargets {
  return {
    pelvis: v(x, 0.115, z), up: v(0, 1, 0), front: v(0, 0, 1),
    arms: { L: limb(v(x + 0.20, 0.27, z + 0.32), v(x + 0.30, 0.35, z + 0.1)), R: limb(v(x - 0.20, 0.27, z + 0.32), v(x - 0.30, 0.35, z + 0.1)) },
    legs: { L: limb(v(x + 0.21, 0.055, z + 0.69), v(x + 0.26, 0.18, z + 0.34)), R: limb(v(x - 0.21, 0.055, z + 0.69), v(x - 0.26, 0.18, z + 0.34)) },
  };
}

export function backPair(node: string): PracticePair {
  const released = node === "safe";
  const blue = seated(released ? -0.55 : 0, 0.12);
  const red = seated(released ? 0.55 : 0, released ? 0.12 : -0.16);
  if (!released) {
    red.legs.L = limb(v(0.15, 0.15, 0.46), v(0.39, 0.20, 0.17));
    red.legs.R = limb(v(-0.15, 0.15, 0.46), v(-0.39, 0.20, 0.17));
    red.arms.L = limb(v(node === "reach" ? 0.16 : -0.08, 0.61, 0.29), v(0.32, 0.52, 0.21));
    red.arms.R = node === "switch" ? limb(v(-0.15, 0.69, 0.24), v(-0.31, 0.43, 0.12)) : limb(v(-0.04, 0.51, 0.33), v(-0.36, 0.42, 0.10));
    if (["switch", "defended"].includes(node)) {
      blue.arms.L = limb(v(0.04, 0.59, 0.30), v(0.29, 0.36, 0.29));
      blue.arms.R = limb(v(-0.12, node === "defended" ? 0.67 : 0.56, 0.29), v(-0.29, 0.36, 0.29));
    }
    if (node === "locked") red.arms.R = limb(v(0.12, 0.65, 0.14), v(-0.27, 0.63, 0.24));
  }
  const b = solveBody(blue), r = solveBody(red);
  return {
    blue: b, red: r,
    supports: [
      { actor: "blue", joint: "pelvis", height: 0.115 }, { actor: "red", joint: "pelvis", height: 0.115 },
      { actor: "blue", joint: "ankleL", height: 0.055 }, { actor: "blue", joint: "ankleR", height: 0.055 },
      ...(released ? [{ actor: "red" as const, joint: "ankleL" as const, height: 0.055 }, { actor: "red" as const, joint: "ankleR" as const, height: 0.055 }] : []),
    ],
    cues: released ? [] : [{ label: node === "locked" ? "タップして止める場面" : node === "reach" ? "首へ向かう腕" : "首の前で手を保つ", point: node === "reach" ? r.wristL : b.wristR, kind: "contact" }],
  };
}

/** 各状態に対応する二人を一組で作る。古い単体ポーズの接地補正を通さない。 */
export function practicePair(lessonId: string, nodeId: string): PracticePair {
  if (nodeId === "stopped") return backPair("safe");
  const lesson = practiceLesson(lessonId);
  if (!lesson.nodes[nodeId]) throw new Error(`Unknown practice node: ${lessonId}/${nodeId}`);
  const visual = lesson.nodes[nodeId]!.visual;
  if (visual) {
    const builders = { mount: mountPair, side: sidePair, guard: guardPair, back: backPair, half: halfPair, turtle: turtlePair, open: openPair };
    const p = builders[visual.family](visual.state, visual.top ?? (visual.family === "guard" ? "blue" : "red"));
    if (!visual.swap) return p;
    return { blue: p.red, red: p.blue, cues: p.cues, supports: p.supports.map((s) => ({ ...s, actor: s.actor === "blue" ? "red" : "blue" })) };
  }
  switch (lessonId) {
    case "side-space": return nodeId === "recovered" ? guardPair("stable", "red") : sidePair(nodeId, "red");
    case "mount-base":
      if (nodeId === "top") return guardPair("stable", "blue");
      if (nodeId === "guard") return guardPair("stable", "red");
      return mountPair(nodeId, "red");
    case "guard-posture": return guardPair(nodeId, "blue");
    case "side-control": return ["mounted", "mount-held"].includes(nodeId) ? mountPair(nodeId, "blue") : sidePair(nodeId, "blue");
    case "back-safety": return backPair(nodeId);
    default: throw new Error(`No practice mannequins for: ${lessonId}`);
  }
}

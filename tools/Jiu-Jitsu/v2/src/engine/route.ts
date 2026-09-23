import { Quaternion, Vector3 } from "three";
import { BODY, solveBody, type BodyPose, type BodyTargets, type Landmark } from "../render/mannequin";
import type { Actor, PracticePair } from "../render/practicePose";

export type RoutePose = Record<Actor, BodyTargets>;
export interface RouteFrame { label: string; seconds: number; pose: RoutePose }
export interface MotionIssue { level: "blocked" | "review"; message: string; actor: Actor; point: Vector3 }

export function bodyTargets(p: BodyPose): BodyTargets {
  return {
    pelvis: p.pelvis.clone(), up: new Vector3(0, 1, 0).applyQuaternion(p.rotation), front: new Vector3(0, 0, 1).applyQuaternion(p.rotation),
    arms: { L: { end: p.wristL.clone(), pole: p.elbowL.clone() }, R: { end: p.wristR.clone(), pole: p.elbowR.clone() } },
    legs: { L: { end: p.ankleL.clone(), pole: p.kneeL.clone() }, R: { end: p.ankleR.clone(), pole: p.kneeR.clone() } },
  };
}
export function routePose(pair: PracticePair): RoutePose { return { blue: bodyTargets(pair.blue), red: bodyTargets(pair.red) }; }
export function cloneRoutePose(pose: RoutePose): RoutePose {
  const copy = (p: BodyTargets): BodyTargets => ({ pelvis: p.pelvis.clone(), up: p.up.clone(), front: p.front.clone(),
    arms: { L: { end: p.arms.L.end.clone(), pole: p.arms.L.pole.clone() }, R: { end: p.arms.R.end.clone(), pole: p.arms.R.pole.clone() } },
    legs: { L: { end: p.legs.L.end.clone(), pole: p.legs.L.pole.clone() }, R: { end: p.legs.R.end.clone(), pole: p.legs.R.pole.clone() } } });
  return { blue: copy(pose.blue), red: copy(pose.red) };
}

export function inspectRoutePose(pose: RoutePose): { pair: PracticePair; issues: MotionIssue[] } {
  const pair: PracticePair = { blue: solveBody(pose.blue), red: solveBody(pose.red), supports: [], cues: [] };
  const issues: MotionIssue[] = [];
  for (const actor of ["blue", "red"] as const) {
    const body = pair[actor], targets = pose[actor], other = pair[actor === "blue" ? "red" : "blue"];
    const add = (level: MotionIssue["level"], message: string, point: Vector3) => {
      if (!issues.some((i) => i.actor === actor && i.message === message)) issues.push({ actor, level, message, point: point.clone() });
    };
    const floor = (minimum: number, label: string, point: Vector3) => {
      if (minimum < -0.006) add("blocked", `${label}が床に入り込んでいます。高さや身体の向きを戻してください。`, point);
    };
    for (const [center, radii, label] of [[body.pelvis, new Vector3(0.165, 0.115, 0.11), "腰"], [body.chest, new Vector3(0.207, 0.205, 0.12), "胸"], [body.head, new Vector3(0.10, 0.125, 0.105), "頭"]] as const) {
      floor(ovalMinimum(center, radii, body.rotation), label, center);
    }
    const supportHeights: Partial<Record<Landmark, number>> = { kneeL: 0.073, kneeR: 0.073, ankleL: 0.055, ankleR: 0.055, wristL: 0.046, wristR: 0.046 };
    for (const [joint, height] of Object.entries(supportHeights) as [Landmark, number][]) {
      if (Math.abs(body[joint].y - height) < 0.012) pair.supports.push({ actor, joint, height: body[joint].y });
    }
    const pelvisFloor = ovalMinimum(body.pelvis, new Vector3(0.165, 0.115, 0.11), body.rotation);
    const chestFloor = ovalMinimum(body.chest, new Vector3(0.207, 0.205, 0.12), body.rotation);
    if (!pair.supports.some((s) => s.actor === actor) && Math.min(pelvisFloor, chestFloor) > 0.025) add("review", "床の支えが見つかりません。相手に支えられているか、支持を確認してください。", body.pelvis);
    for (const side of ["L", "R"] as const) {
      const sideName = side === "L" ? "左" : "右";
      for (const [kind, root, middle, end, a, b] of [
        ["arms", body[`shoulder${side}`], body[`elbow${side}`], body[`wrist${side}`], BODY.upperArm, BODY.forearm],
        ["legs", body[`hip${side}`], body[`knee${side}`], body[`ankle${side}`], BODY.thigh, BODY.shin],
      ] as const) {
        const target = targets[kind][side], limb = `${sideName}${kind === "arms" ? "腕" : "脚"}`;
        if (target.end.distanceTo(end) > 0.006) add("blocked", root.distanceTo(target.end) > a + b
          ? `${limb}の長さでは目標に届きません。手足を近づけるか、腰を動かしてください。`
          : `${limb}を折りたたみすぎています。表示用の屈曲範囲に戻してください。`, target.end);
        floor(target.end.y - (kind === "legs" ? 0.055 : 0.03), `${sideName}${kind === "legs" ? "足" : "手"}の目標`, target.end);
        floor(middle.y - (kind === "legs" ? 0.073 : 0.05), `${sideName}${kind === "legs" ? "膝" : "肘"}`, middle);
        for (const [p, q, r0, r1] of [[root, middle, kind === "arms" ? 0.074 : 0.092, kind === "arms" ? 0.048 : 0.065], [middle, end, kind === "arms" ? 0.05 : 0.06, kind === "arms" ? 0.033 : 0.041]] as const) {
          const vertical = (q.y - p.y) / p.distanceTo(q), horizontal = Math.sqrt(Math.max(0, 1 - vertical * vertical));
          floor(Math.min(p.y - r0 * horizontal, q.y - r1 * horizontal), limb, middle);
          for (const t of [0.2, 0.4, 0.6, 0.8, 1]) {
            const point = p.clone().lerp(q, t);
            if (insideCore(point, other)) add("blocked", `${limb}が相手の身体の内部を通っています。外側へ回す中間姿勢を入れてください。`, point);
            if (p === middle && insideCore(point, body)) add("blocked", `${limb}が自分の身体の内部を通っています。曲げる向きを変えてください。`, point);
          }
        }
        const direction = middle.clone().sub(root).applyQuaternion(body.rotation.clone().invert());
        const flex = Math.atan2(direction.z, -direction.y) * 180 / Math.PI;
        if (kind === "legs" && (flex < -40 || flex > 140)) add("review", `${sideName}股関節の向きが大きくなっています。骨盤の向きと膝の曲がる側を確認してください。`, root);
        if (kind === "arms" && direction.z < -0.10 && direction.y > 0.02) add("review", `${sideName}腕が肩の後ろ上方へ向いています。肩の回旋はこの人形だけでは判定できません。`, root);
      }
    }
    if (body.head.distanceTo(other.head) < 0.18 || body.chest.distanceTo(other.chest) < 0.17 || body.pelvis.distanceTo(other.pelvis) < 0.15) add("blocked", "二人の頭・胸・腰が深く重なっています。身体を離してください。", body.chest);
  }
  pair.cues = issues.slice(0, 6).map((issue) => ({ label: issue.message, point: issue.point, kind: "contact" }));
  return { pair, issues };
}

function ovalMinimum(center: Vector3, radii: Vector3, rotation: Quaternion): number {
  const x = new Vector3(1, 0, 0).applyQuaternion(rotation).y * radii.x;
  const y = new Vector3(0, 1, 0).applyQuaternion(rotation).y * radii.y;
  const z = new Vector3(0, 0, 1).applyQuaternion(rotation).y * radii.z;
  return center.y - Math.hypot(x, y, z);
}
function insideCore(point: Vector3, body: BodyPose): boolean {
  const inverse = body.rotation.clone().invert();
  return ([[body.chest, new Vector3(0.16, 0.15, 0.075)], [body.pelvis, new Vector3(0.12, 0.075, 0.07)], [body.head, new Vector3(0.075, 0.10, 0.08)]] as const)
    .some(([center, radii]) => point.clone().sub(center).applyQuaternion(inverse).divide(radii).length() < 1);
}

export function interpolateRoute(a: RoutePose, b: RoutePose, fraction: number): RoutePose {
  fraction = Math.max(0, Math.min(1, fraction));
  const pose = cloneRoutePose(a);
  for (const actor of ["blue", "red"] as const) {
    const start = a[actor], end = b[actor], p = pose[actor];
    p.pelvis.lerp(end.pelvis, fraction);
    const qa = solveBody(start).rotation, qb = solveBody(end).rotation;
    const rotation = new Quaternion().slerpQuaternions(qa, qb, fraction);
    p.up.set(0, 1, 0).applyQuaternion(rotation); p.front.set(0, 0, 1).applyQuaternion(rotation);
    for (const limbs of ["arms", "legs"] as const) for (const side of ["L", "R"] as const) {
      p[limbs][side].end.lerp(end[limbs][side].end, fraction);
      p[limbs][side].pole.lerp(end[limbs][side].pole, fraction);
    }
  }
  return pose;
}

/** 登録した姿勢の間も検査する。離散サンプルのため連続衝突の保証ではない。 */
export function inspectTransition(a: RoutePose, b: RoutePose): { fraction: number; issues: MotionIssue[] } | null {
  let previous: PracticePair | undefined;
  for (let i = 0; i <= 24; i++) {
    const fraction = i / 24;
    const result = inspectRoutePose(interpolateRoute(a, b, fraction));
    const issues = result.issues.filter((issue) => issue.level === "blocked");
    if (previous) for (const actor of ["blue", "red"] as const) for (const joint of ["elbowL", "elbowR", "kneeL", "kneeR"] as const) {
      if (previous[actor][joint].distanceTo(result.pair[actor][joint]) > 0.12) issues.push({ actor, level: "blocked", point: result.pair[actor][joint], message: "肘・膝の位置が急に切り替わります。曲がる方向をつなぐ中間姿勢を入れてください。" });
    }
    if (issues.length) return { fraction, issues };
    previous = result.pair;
  }
  return null;
}

// 骨格を主役にしたヒューマノイドレンダラ。
// 旧 fighter.js の階層・寸法・floor probe を継承しつつ、服飾ディテールを外し、
// 解剖モデルが定義する関節を常時マーカー表示・ハイライト可能にした。
// ポーズ適用時の clamp は anatomy の同じ可動域データを使う。

import * as THREE from "three";
import { clampJointEuler, specForRigJoint } from "../anatomy/joints";
import type { AnatomyJointId, RigJointName } from "../anatomy/types";
import { DIMS as D, SKELETON, type Pose } from "./rig";

const SKIN = 0xd9a878;
const FLOOR_Y = 0.012;
const UNIT_SCALE = new THREE.Vector3(1, 1, 1);
const IDENTITY_QUAT = new THREE.Quaternion();

// 軽量 grounding solver 用のプローブ点 (完全な物理ではなく、マット抜け/浮きの補正)
const FLOOR_PROBES: readonly [RigJointName, readonly (readonly [number, number, number])[]][] = [
  ["hips", [[0, -D.torsoR * 0.72, 0], [0, 0, -D.torsoR * 0.82], [0, 0, D.torsoR * 0.82]]],
  ["spine", [[0, D.spine * 0.5, -D.torsoR * 0.78], [0, D.spine * 0.5, D.torsoR * 0.78]]],
  ["chest", [[0, D.chest * 0.45, -D.torsoR * 0.78], [0, D.chest * 0.45, D.torsoR * 0.78]]],
  ["head", [[0, D.headR * 0.75, -D.headR * 0.9], [0, D.headR * 0.75, D.headR * 0.9]]],
  ["forearmL", [[0, -D.forearm * 0.52, 0]]],
  ["forearmR", [[0, -D.forearm * 0.52, 0]]],
  ["handL", [[0, -D.hand, 0]]],
  ["handR", [[0, -D.hand, 0]]],
  ["thighL", [[0, -D.thigh * 0.45, 0]]],
  ["thighR", [[0, -D.thigh * 0.45, 0]]],
  ["shinL", [[0, 0, 0], [0, -D.shin * 0.55, 0]]],
  ["shinR", [[0, 0, 0], [0, -D.shin * 0.55, 0]]],
  ["footL", [[0, -0.025, -0.02], [0, -0.025, D.foot]]],
  ["footR", [[0, -0.025, -0.02], [0, -0.025, D.foot]]],
];

export interface FighterOptions {
  color: number;
  accent: number;
}

export class Fighter {
  readonly root = new THREE.Group();
  readonly joints = {} as Record<RigJointName, THREE.Object3D>;

  private readonly bodyMat: THREE.MeshStandardMaterial;
  private readonly jointMat: THREE.MeshStandardMaterial;
  private readonly highlightMat: THREE.MeshStandardMaterial;
  private readonly jointMarkers = new Map<RigJointName, THREE.Mesh>();

  private targets: Partial<Record<RigJointName, THREE.Quaternion>> = {};
  private rootTargetPos = new THREE.Vector3(0, D.hipY, 0);
  private rootTargetQuat = new THREE.Quaternion();

  constructor({ color, accent }: FighterOptions) {
    this.bodyMat = new THREE.MeshStandardMaterial({ color, roughness: 0.8 });
    const skin = new THREE.MeshStandardMaterial({ color: SKIN, roughness: 0.6 });
    this.jointMat = new THREE.MeshStandardMaterial({
      color: accent,
      roughness: 0.5,
      emissive: accent,
      emissiveIntensity: 0.12,
    });
    this.highlightMat = new THREE.MeshStandardMaterial({
      color: 0xffd75e,
      emissive: 0xffb300,
      emissiveIntensity: 0.85,
      roughness: 0.3,
    });

    for (const bone of SKELETON) {
      const j = new THREE.Object3D();
      j.position.set(...bone.localPos);
      this.joints[bone.name] = j;
      if (bone.parent) this.joints[bone.parent].add(j);
      else this.root.add(j);
    }

    this.buildBody(skin);
    this.buildJointMarkers();

    this.root.position.set(0, D.hipY, 0);
    this.applyPose({}, { immediate: true });
  }

  private taperedLimb(
    joint: THREE.Object3D,
    len: number,
    r0: number,
    r1: number,
    mat: THREE.MeshStandardMaterial = this.bodyMat,
  ): void {
    const mesh = new THREE.Mesh(new THREE.CylinderGeometry(r1, r0, len, 16), mat);
    mesh.position.y = -len / 2;
    mesh.castShadow = true;
    joint.add(mesh);
    // 近位・遠位端の丸み — 関節部で肌が途切れて見えないようにする
    const proximal = new THREE.Mesh(new THREE.SphereGeometry(r0, 14, 12), mat);
    proximal.castShadow = true;
    joint.add(proximal);
    const distal = new THREE.Mesh(new THREE.SphereGeometry(r1 * 1.02, 14, 12), mat);
    distal.position.y = -len;
    joint.add(distal);
  }

  private buildBody(skin: THREE.MeshStandardMaterial): void {
    const lr = D.limbR;
    for (const side of ["L", "R"] as const) {
      this.taperedLimb(this.joints[`upperArm${side}`], D.upperArm, lr * 1.15, lr * 0.85);
      this.taperedLimb(this.joints[`forearm${side}`], D.forearm, lr * 0.85, lr * 0.62);
      this.taperedLimb(this.joints[`thigh${side}`], D.thigh, lr * 1.55, lr * 1.05);
      this.taperedLimb(this.joints[`shin${side}`], D.shin, lr * 1.05, lr * 0.72);

      const palm = new THREE.Mesh(new THREE.BoxGeometry(0.07, D.hand, 0.035), skin);
      palm.position.y = -D.hand / 2;
      palm.castShadow = true;
      this.joints[`hand${side}`].add(palm);

      const foot = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.05, D.foot), skin);
      foot.position.set(0, -0.02, D.foot / 2 - 0.02);
      foot.castShadow = true;
      this.joints[`foot${side}`].add(foot);
    }

    // 胴: 骨盤→ウエスト(細)→胸郭(広)→肩のなで肩ヨーク。前後は薄い楕円断面
    const belly = new THREE.Mesh(
      new THREE.CylinderGeometry(D.torsoR * 0.94, D.torsoR * 0.88, D.spine * 1.15, 18),
      this.bodyMat,
    );
    belly.position.y = D.spine / 2;
    belly.scale.set(1, 1, 0.8);
    belly.castShadow = true;
    this.joints.spine.add(belly);

    const rib = new THREE.Mesh(
      new THREE.CylinderGeometry(D.torsoR * 1.16, D.torsoR * 0.92, D.chest, 18),
      this.bodyMat,
    );
    rib.position.y = D.chest / 2;
    rib.scale.set(1, 1, 0.78);
    rib.castShadow = true;
    this.joints.chest.add(rib);

    // 大胸筋・肩甲帯の盛り
    const chestCap = new THREE.Mesh(new THREE.SphereGeometry(D.torsoR * 1.16, 18, 14), this.bodyMat);
    chestCap.position.y = D.chest * 0.98;
    chestCap.scale.set(1, 0.42, 0.78);
    chestCap.castShadow = true;
    this.joints.chest.add(chestCap);

    // 三角筋 (肩の丸み)
    for (const sx of [-1, 1]) {
      const delt = new THREE.Mesh(new THREE.SphereGeometry(D.limbR * 1.22, 16, 12), this.bodyMat);
      delt.position.set(sx * (D.shoulderHalf - 0.012), D.chest - 0.035, 0);
      delt.scale.set(1.05, 1.0, 0.95);
      delt.castShadow = true;
      this.joints.chest.add(delt);
    }

    // 僧帽筋 (首の付け根のなだらかな盛り)
    const yoke = new THREE.Mesh(new THREE.SphereGeometry(D.torsoR * 0.6, 16, 12), this.bodyMat);
    yoke.position.set(0, D.chest - 0.015, 0);
    yoke.scale.set(1.5, 0.5, 0.8);
    this.joints.chest.add(yoke);

    const pelvis = new THREE.Mesh(new THREE.SphereGeometry(D.torsoR * 0.98, 18, 14), this.bodyMat);
    pelvis.scale.set(1.12, 0.8, 0.86);
    pelvis.castShadow = true;
    this.joints.hips.add(pelvis);

    // 首・頭 (鼻と顎で向きを読ませる)
    const neck = new THREE.Mesh(new THREE.CylinderGeometry(0.05, 0.058, D.neck, 12), skin);
    neck.position.y = D.neck / 2;
    this.joints.neck.add(neck);

    const head = new THREE.Mesh(new THREE.SphereGeometry(D.headR * 0.94, 22, 16), skin);
    head.position.y = D.headR * 0.78;
    head.scale.set(0.9, 1.08, 0.98);
    head.castShadow = true;
    this.joints.head.add(head);

    // 耳 (頭の向きの読み取り補助)
    for (const sx of [-1, 1]) {
      const ear = new THREE.Mesh(new THREE.SphereGeometry(0.02, 10, 8), skin);
      ear.position.set(sx * D.headR * 0.86, D.headR * 0.74, 0);
      ear.scale.set(0.55, 1, 0.9);
      this.joints.head.add(ear);
    }

    const jaw = new THREE.Mesh(new THREE.SphereGeometry(D.headR * 0.5, 14, 10), skin);
    jaw.position.set(0, D.headR * 0.4, D.headR * 0.42);
    this.joints.head.add(jaw);

    const nose = new THREE.Mesh(new THREE.SphereGeometry(0.02, 10, 8), skin);
    nose.position.set(0, D.headR * 0.66, D.headR * 0.96);
    this.joints.head.add(nose);

    // 髪キャップ (本体色) — 赤/青の識別を頭でも読めるように
    const cap = new THREE.Mesh(
      new THREE.SphereGeometry(D.headR * 1.04, 22, 16, 0, Math.PI * 2, 0, Math.PI * 0.62),
      this.bodyMat,
    );
    cap.position.y = D.headR * 0.78;
    cap.scale.set(0.94, 1.05, 0.98);
    this.joints.head.add(cap);
  }

  /** 解剖モデルが教育対象とする関節に球マーカーを置く */
  private buildJointMarkers(): void {
    for (const bone of SKELETON) {
      if (!specForRigJoint(bone.name)) continue;
      const marker = new THREE.Mesh(
        new THREE.SphereGeometry(D.limbR * 0.62, 12, 10),
        this.jointMat,
      );
      this.joints[bone.name].add(marker);
      this.jointMarkers.set(bone.name, marker);
    }
  }

  /** 指定した解剖関節をハイライトする (null で解除) */
  highlightJoint(id: AnatomyJointId | null): void {
    for (const [rig, marker] of this.jointMarkers) {
      const spec = specForRigJoint(rig);
      marker.material = spec && spec.id === id ? this.highlightMat : this.jointMat;
      marker.scale.setScalar(spec && spec.id === id ? 1.45 : 1);
    }
  }

  /** リグ関節のワールド座標 (アーク描画・ラベル用) */
  jointWorldPosition(rig: RigJointName, out = new THREE.Vector3()): THREE.Vector3 {
    return this.joints[rig].getWorldPosition(out);
  }

  applyPose(pose: Pose, { immediate = false } = {}): void {
    const joints = pose.joints ?? {};
    const e = new THREE.Euler();
    const q = new THREE.Quaternion();
    for (const bone of SKELETON) {
      const raw = joints[bone.name] ?? ([0, 0, 0] as const);
      const c = clampJointEuler(bone.name, [raw[0], raw[1], raw[2]]);
      e.set(c[0], c[1], c[2], "XYZ");
      q.setFromEuler(e);
      this.targets[bone.name] = q.clone();
      if (immediate) this.joints[bone.name].quaternion.copy(q);
    }

    const r = pose.root;
    if (r?.pos) this.rootTargetPos.set(r.pos[0], r.pos[1], r.pos[2]);
    else this.rootTargetPos.set(0, D.hipY, 0);
    const rot = r?.rot ?? ([0, 0, 0] as const);
    this.rootTargetQuat.setFromEuler(new THREE.Euler(rot[0], rot[1], rot[2], "XYZ"));
    this.solveGroundedRootTarget(Boolean(r?.pos));

    if (immediate) {
      this.root.position.copy(this.rootTargetPos);
      this.root.quaternion.copy(this.rootTargetQuat);
    }
  }

  private targetJointMatrices(rootPos: THREE.Vector3): Map<RigJointName, THREE.Matrix4> {
    const matrices = new Map<RigJointName, THREE.Matrix4>();
    const rootMatrix = new THREE.Matrix4().compose(rootPos, this.rootTargetQuat, UNIT_SCALE);
    for (const bone of SKELETON) {
      const local = new THREE.Matrix4().compose(
        new THREE.Vector3(...bone.localPos),
        this.targets[bone.name] ?? IDENTITY_QUAT,
        UNIT_SCALE,
      );
      const parent = bone.parent ? matrices.get(bone.parent) : rootMatrix;
      matrices.set(bone.name, (parent ?? rootMatrix).clone().multiply(local));
    }
    return matrices;
  }

  private floorProbeMinY(rootPos: THREE.Vector3): number {
    const matrices = this.targetJointMatrices(rootPos);
    const p = new THREE.Vector3();
    let minY = Infinity;
    for (const [joint, points] of FLOOR_PROBES) {
      const matrix = matrices.get(joint);
      if (!matrix) continue;
      for (const point of points) {
        p.set(point[0], point[1], point[2]).applyMatrix4(matrix);
        minY = Math.min(minY, p.y);
      }
    }
    return minY;
  }

  // v1 は補正量を ±0.12/0.05 に制限していたため、手打ちの root 高さの誤りが
  // そのまま残った (座位が浮く・寝技が沈む)。v2 は接地を厳密に解く:
  // ポーズの y は初期値扱いとし、プローブ最下点が常にマットに接するまで root を動かす。
  private solveGroundedRootTarget(hasExplicitRoot: boolean): void {
    if (!hasExplicitRoot || this.rootTargetPos.y >= 0.88) return;
    const minY = this.floorProbeMinY(this.rootTargetPos);
    if (!Number.isFinite(minY)) return;
    this.rootTargetPos.y += FLOOR_Y - minY;
  }

  /** dt 秒で目標へ近づける (フレームレート非依存の指数イージング) */
  update(dt: number, speed = 6): void {
    const a = 1 - Math.exp(-speed * dt);
    for (const bone of SKELETON) {
      const t = this.targets[bone.name];
      if (t) this.joints[bone.name].quaternion.slerp(t, a);
    }
    this.root.position.lerp(this.rootTargetPos, a);
    this.root.quaternion.slerp(this.rootTargetQuat, a);
  }
}

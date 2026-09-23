import * as T from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { footGeometry, headGeometry, HIP_RADII, limbGeometry, torsoGeometry } from "./mannequinShape";

export const BODY = { upperArm: 0.28, forearm: 0.26, thigh: 0.41, shin: 0.4, kneeRadius: 0.073 } as const;
export type Side = "L" | "R";
export type Landmark = "pelvis" | "chest" | "neck" | "head" |
  `shoulder${Side}` | `elbow${Side}` | `wrist${Side}` | `hip${Side}` | `knee${Side}` | `ankle${Side}`;
export type BodyPose = Record<Landmark, T.Vector3> & { rotation: T.Quaternion };
export interface LimbTarget { end: T.Vector3; pole: T.Vector3 }
export interface BodyTargets {
  pelvis: T.Vector3;
  up: T.Vector3;
  front: T.Vector3;
  arms: Record<Side, LimbTarget>;
  legs: Record<Side, LimbTarget>;
}

/** 固定長の二節。pole は曲げる側。表示用の屈曲制限で、人体の安全限界ではない。 */
export function solveLimb(start: T.Vector3, target: LimbTarget, a: number, b: number) {
  const axis = target.end.clone().sub(start);
  const requested = axis.length();
  if (requested < 1e-8) axis.set(0, -1, 0);
  else axis.divideScalar(requested);
  const minimum = Math.sqrt(a * a + b * b + 2 * a * b * Math.cos(150 * Math.PI / 180));
  const distance = T.MathUtils.clamp(requested, minimum, a + b - 1e-6);
  const bend = target.pole.clone().sub(start);
  bend.addScaledVector(axis, -bend.dot(axis));
  if (bend.lengthSq() < 1e-8) {
    bend.set(Math.abs(axis.x) < 0.8 ? 1 : 0, Math.abs(axis.x) < 0.8 ? 0 : 1, 0);
    bend.addScaledVector(axis, -bend.dot(axis));
  }
  bend.normalize();
  const along = (a * a - b * b + distance * distance) / (2 * distance);
  return {
    joint: start.clone().addScaledVector(axis, along).addScaledVector(bend, Math.sqrt(Math.max(0, a * a - along * along))),
    end: start.clone().addScaledVector(axis, distance),
  };
}

export function bodyFrame(pelvis: T.Vector3, up: T.Vector3, front: T.Vector3) {
  const y = up.clone().normalize();
  const x = y.clone().cross(front).normalize();
  const z = x.clone().cross(y).normalize();
  const rotation = new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(x, y, z));
  const point = (px: number, py: number, pz: number) => pelvis.clone().addScaledVector(x, px).addScaledVector(y, py).addScaledVector(z, pz);
  return { rotation, point };
}

export function solveBody(targets: BodyTargets): BodyPose {
  const { point, rotation } = bodyFrame(targets.pelvis, targets.up, targets.front);
  const pose = { pelvis: targets.pelvis.clone(), chest: point(0, 0.32, 0), neck: point(0, 0.50, 0), head: point(0, 0.63, 0), rotation } as BodyPose;
  for (const [side, sign] of [["L", 1], ["R", -1]] as const) {
    pose[`shoulder${side}`] = point(sign * 0.20, 0.43, 0);
    pose[`hip${side}`] = point(sign * 0.11, 0, 0);
    const arm = solveLimb(pose[`shoulder${side}`], targets.arms[side], BODY.upperArm, BODY.forearm);
    const leg = solveLimb(pose[`hip${side}`], targets.legs[side], BODY.thigh, BODY.shin);
    pose[`elbow${side}`] = arm.joint;
    pose[`wrist${side}`] = arm.end;
    pose[`knee${side}`] = leg.joint;
    pose[`ankle${side}`] = leg.end;
  }
  return pose;
}

/** 膝の表面を y=0 へ固定。脛はマットに沿い、脚長は変えない。 */
export function kneelingLeg(hip: T.Vector3, kneeDirection: T.Vector3, heelDirection: T.Vector3): LimbTarget {
  const drop = hip.y - BODY.kneeRadius;
  if (drop < 0 || drop >= BODY.thigh) throw new Error("Pelvis cannot reach a kneeling support");
  const knee = kneeDirection.clone().setY(0).normalize().multiplyScalar(Math.sqrt(BODY.thigh ** 2 - drop ** 2)).add(hip);
  knee.y = BODY.kneeRadius;
  const end = heelDirection.clone().setY(0).normalize().multiplyScalar(Math.sqrt(BODY.shin ** 2 - (BODY.kneeRadius - 0.055) ** 2)).add(knee);
  end.y = 0.055;
  return { end, pole: knee };
}

const Y = new T.Vector3(0, 1, 0);

/** 骨の位置と身体の厚みを同じ座標から描く。衣装の部品や角度リグには依存しない。 */
export class Mannequin {
  readonly root = new T.Group();
  private readonly flesh = new T.Group();
  private readonly skeleton = new T.Group();
  private readonly shell: T.MeshStandardMaterial;
  private readonly bone: T.MeshStandardMaterial;
  private readonly face: T.MeshStandardMaterial;
  private readonly meshes: T.Mesh[] = [];
  private readonly rigid: T.Mesh[] = [];
  private readonly resources = new Set<T.BufferGeometry>();
  private readonly shared = new Map<string, T.BufferGeometry>();
  private cursor = 0;

  constructor(public pose: BodyPose, color: number) {
    this.shell = new T.MeshStandardMaterial({ color, roughness: 0.67, metalness: 0.04 });
    this.bone = new T.MeshStandardMaterial({ color: new T.Color(color).lerp(new T.Color(0xfff4de), 0.58), roughness: 0.8 });
    this.face = new T.MeshStandardMaterial({ color: 0x25354a, roughness: 0.8 });
    this.root.add(this.flesh, this.skeleton);
    this.buildRigidParts(pose);
    this.updatePose(pose);
    this.setDisplay(false, false);
  }

  private buildRigidParts(p: BodyPose): void {
    this.shaped(torsoGeometry, p.pelvis, p.rotation);
    this.link(this.flesh, p.neck, p.head, 0.047, 0.05, this.shell);
    this.shaped(headGeometry, p.head, p.rotation);
    const local = (x: number, y: number, z: number) => new T.Vector3(x, y, z).applyQuaternion(p.rotation).add(p.head);
    this.oval(this.flesh, local(0, -0.010, 0.097), [0.014, 0.020, 0.021], this.shell, p.rotation);
    for (const sign of [-1, 1]) this.oval(this.flesh, local(sign * 0.035, 0.025, 0.084), [0.012, 0.007, 0.005], this.face, p.rotation);

    for (const side of ["L", "R"] as const) {
      this.oval(this.flesh, p[`hip${side}`], HIP_RADII, this.shell, p.rotation);
      this.link(this.skeleton, p.neck, p[`shoulder${side}`], 0.015, 0.018, this.bone);
    }
    const torsoPoint = (x: number, y: number, z: number) => new T.Vector3(x, y, z).applyQuaternion(p.rotation).add(p.pelvis);
    for (let i = 0; i <= 10; i++) this.oval(this.skeleton, torsoPoint(0, i * 0.046, -0.05), [0.026, 0.023, 0.024], this.bone, p.rotation);
    for (let i = 0; i < 5; i++) {
      const y = 0.23 + i * 0.045;
      const width = 0.13 + Math.sin(i / 4 * Math.PI) * 0.035;
      for (const sign of [-1, 1]) this.curve([
        torsoPoint(0, y + 0.02, -0.065), torsoPoint(sign * width, y, -0.02),
        torsoPoint(sign * width * 0.65, y - 0.01, 0.095), torsoPoint(0, y, 0.10),
      ]);
    }
    this.link(this.skeleton, torsoPoint(0, 0.23, 0.10), torsoPoint(0, 0.44, 0.10), 0.013, 0.016, this.bone);
    for (const sign of [-1, 1]) this.curve([torsoPoint(0, 0.07, -0.07), torsoPoint(sign * 0.13, 0.07, 0), torsoPoint(sign * 0.13, -0.025, 0.065), torsoPoint(0, -0.065, 0.06)]);
    this.link(this.skeleton, p.neck, p.head, 0.024, 0.027, this.bone);
    this.oval(this.skeleton, p.head, [0.09, 0.11, 0.095], this.bone, p.rotation);

    // 胴体・頭・肋骨はこのリグでは同じ剛体。材質ごとに結合し、骨盤の座標へ戻す。
    const inverse = new T.Matrix4().compose(p.pelvis, p.rotation, new T.Vector3(1, 1, 1)).invert();
    for (const [parent, material, name] of [[this.flesh, this.shell, "body-core"], [this.flesh, this.face, "face"], [this.skeleton, this.bone, "skeleton-core"]] as const) {
      const parts = this.meshes.filter((mesh) => mesh.parent === parent && mesh.material === material);
      const pieces = parts.map((mesh) => {
        mesh.updateMatrix();
        const geometry = mesh.geometry.clone().applyMatrix4(mesh.matrix).applyMatrix4(inverse);
        for (const key of Object.keys(geometry.attributes)) if (key !== "position" && key !== "normal") geometry.deleteAttribute(key);
        parent.remove(mesh);
        return geometry;
      });
      const merged = mergeGeometries(pieces);
      pieces.forEach((geometry) => geometry.dispose());
      if (!merged) throw new Error("Could not merge mannequin body parts");
      const mesh = new T.Mesh(merged, material); mesh.name = name;
      mesh.castShadow = parent === this.flesh; mesh.receiveShadow = true;
      parent.add(mesh); this.rigid.push(mesh);
    }
    for (const geometry of this.resources) geometry.dispose();
    this.resources.clear(); this.shared.clear(); this.meshes.length = 0; this.cursor = 0;
    for (const mesh of this.rigid) this.resources.add(mesh.geometry);
  }

  updatePose(pose: BodyPose): void {
    this.pose = pose; this.cursor = 0;
    const p = pose;
    for (const mesh of this.rigid) { mesh.position.copy(p.pelvis); mesh.quaternion.copy(p.rotation); }

    for (const side of ["L", "R"] as const) {
      const shoulder = p[`shoulder${side}`], elbow = p[`elbow${side}`], wrist = p[`wrist${side}`];
      const hip = p[`hip${side}`], knee = p[`knee${side}`], ankle = p[`ankle${side}`];
      this.link(this.flesh, shoulder, elbow, 0.074, 0.048, this.shell);
      this.link(this.flesh, elbow, wrist, 0.05, 0.033, this.shell);
      this.link(this.flesh, hip, knee, 0.092, 0.065, this.shell);
      this.link(this.flesh, knee, ankle, 0.060, 0.041, this.shell);
      const upperDirection = elbow.clone().sub(shoulder).normalize();
      this.oval(this.flesh, shoulder.clone().addScaledVector(upperDirection, 0.025), [0.073, 0.092, 0.071], this.shell, new T.Quaternion().setFromUnitVectors(Y, upperDirection));
      for (const [at, r] of [[elbow, 0.050], [knee, BODY.kneeRadius]] as const) this.oval(this.flesh, at, [r, r, r], this.shell);
      const handDirection = wrist.clone().sub(elbow).normalize();
      if (wrist.y < 0.12) {
        this.oval(this.flesh, wrist.clone().add(new T.Vector3(0, -0.018, 0)), [0.043, 0.028, 0.065], this.shell);
      } else {
        const handRotation = new T.Quaternion().setFromUnitVectors(Y, handDirection);
        this.oval(this.flesh, wrist.clone().addScaledVector(handDirection, 0.026), [0.038, 0.060, 0.025], this.shell, handRotation);
      }
      const footDirection = ankle.clone().sub(knee).setY(0).normalize();
      // すねが立つと、足先はすねの傾斜の逆向きではなく身体の前を向く。
      if (knee.y - ankle.y > 0.20 && new T.Vector3(0, 1, 0).applyQuaternion(p.rotation).y > 0.6) {
        footDirection.set(0, 0, 1).applyQuaternion(p.rotation).setY(0).normalize();
      }
      if (footDirection.lengthSq() < 1e-8) footDirection.set(0, 0, 1);
      const footRotation = new T.Quaternion().setFromUnitVectors(new T.Vector3(0, 0, 1), footDirection);
      const foot = this.mesh(this.flesh, footGeometry, this.shell, "foot");
      foot.position.copy(ankle).addScaledVector(footDirection, 0.047).add(new T.Vector3(0, -0.015, 0));
      foot.scale.set(0.054, 0.040, 0.105); foot.quaternion.copy(footRotation);
      for (const [a, b] of [[shoulder, elbow], [elbow, wrist], [hip, knee], [knee, ankle]]) this.link(this.skeleton, a!, b!, 0.019, 0.016, this.bone);
      for (const at of [shoulder, elbow, wrist, hip, knee, ankle]) this.oval(this.skeleton, at, [0.031, 0.031, 0.031], this.bone);
    }
  }

  private mesh(parent: T.Group, geometry: () => T.BufferGeometry, material: T.Material, sharedKey?: string): T.Mesh {
    const index = this.cursor++;
    if (!this.meshes[index]) {
      const shape = sharedKey ? this.shared.get(sharedKey) ?? geometry() : geometry();
      if (sharedKey) this.shared.set(sharedKey, shape);
      this.resources.add(shape);
      const mesh = new T.Mesh(shape, material);
      mesh.name = `body-part-${index}`; mesh.castShadow = parent === this.flesh; mesh.receiveShadow = true;
      parent.add(mesh); this.meshes[index] = mesh;
    }
    return this.meshes[index]!;
  }

  private oval(parent: T.Group, at: T.Vector3, scale: readonly [number, number, number], material: T.Material, rotation?: T.Quaternion): void {
    const detail = parent === this.skeleton || material === this.face ? "small" : "body";
    const mesh = this.mesh(parent, () => detail === "small" ? new T.SphereGeometry(1, 12, 8) : new T.SphereGeometry(1, 16, 12), material, `sphere-${detail}`);
    mesh.position.copy(at); mesh.scale.set(...scale);
    mesh.quaternion.copy(rotation ?? new T.Quaternion());
  }

  private shaped(geometry: () => T.BufferGeometry, at: T.Vector3, rotation: T.Quaternion): void {
    const mesh = this.mesh(this.flesh, geometry, this.shell);
    mesh.position.copy(at); mesh.quaternion.copy(rotation);
  }

  private link(parent: T.Group, a: T.Vector3, b: T.Vector3, r0: number, r1: number, material: T.Material): void {
    const length = a.distanceTo(b);
    const mesh = this.mesh(parent, () => parent === this.flesh ? limbGeometry(r0, r1) : new T.CylinderGeometry(r1, r0, 1, 12), material, `${parent === this.flesh ? "limb" : "bone"}-${r0}-${r1}`);
    mesh.scale.y = length;
    mesh.position.copy(a).lerp(b, 0.5);
    mesh.quaternion.setFromUnitVectors(Y, b.clone().sub(a).normalize());
  }

  private curve(points: T.Vector3[]): void {
    this.mesh(this.skeleton, () => new T.TubeGeometry(new T.CatmullRomCurve3(points), 12, 0.008, 6, false), this.bone);
  }

  setDisplay(skeleton: boolean, faded: boolean): void {
    const shellWasTransparent = this.shell.transparent;
    const boneWasTransparent = this.bone.transparent;
    this.skeleton.visible = skeleton;
    this.shell.transparent = skeleton || faded;
    this.shell.opacity = skeleton ? (faded ? 0.045 : 0.10) : faded ? 0.20 : 1;
    this.shell.depthWrite = !this.shell.transparent;
    this.bone.transparent = faded; this.bone.opacity = faded ? 0.24 : 1; this.bone.depthWrite = !faded;
    // 不透明用シェーダーは alpha を 1 に固定するため、切替時は再コンパイルが必要。
    if (shellWasTransparent !== this.shell.transparent) this.shell.needsUpdate = true;
    if (boneWasTransparent !== this.bone.transparent) this.bone.needsUpdate = true;
    this.face.visible = !faded;
    this.flesh.traverse((obj) => { if (obj instanceof T.Mesh) obj.castShadow = !skeleton && !faded; });
  }

  dispose(): void {
    for (const geometry of this.resources) geometry.dispose();
    this.resources.clear(); this.shared.clear();
    this.shell.dispose(); this.bone.dispose(); this.face.dispose();
  }
}

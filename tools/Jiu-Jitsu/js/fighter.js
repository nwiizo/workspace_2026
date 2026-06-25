// fighter.js
// プリミティブで組んだヒューマノイド。関節 (Object3D) の階層を持ち、
// 名前付きポーズ (関節オイラー角 + ルート変換) をクォータニオン補間で滑らかに遷移させる。
//
// 設計方針:
//   - 立ち姿 (全関節 identity) を基準。腕・脚は -Y (下) に伸び、胴は +Y (上) に伸びる。
//   - グラップリングの「相対位置」が一番大事なので、各フェーズで両者の root を含めて指定する。
//   - 四肢はテーパー (近位太・遠位細)、胴/肩/首を造形し、手足にディテールを足して人体精度を上げる。
//   - gi / no-gi を切替: gi=襟・ラペル・帯, no-gi=ラッシュガード風 (襟なし・袖短め)。

import * as THREE from "three";
import { clampJointEuler } from "./anatomy.js";

export const deg = (d) => (d * Math.PI) / 180;

const SKIN = 0xd9a878; // 肌色 (赤青で共通)

// 体格 (おおよそ身長 1.8 相当)
const D = {
  hipY: 0.92, // 立位での骨盤の高さ
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
};

// 関節の親子関係と「親原点からのローカル位置」。
// 子関節は親セグメントの遠位端に置く。
const SKELETON = [
  // name,        parent,     localPos
  ["hips", null, [0, 0, 0]],
  ["spine", "hips", [0, 0, 0]],
  ["chest", "spine", [0, D.spine, 0]],
  ["neck", "chest", [0, D.chest, 0]],
  ["head", "neck", [0, D.neck, 0]],

  ["upperArmL", "chest", [D.shoulderHalf, D.chest - 0.03, 0]],
  ["forearmL", "upperArmL", [0, -D.upperArm, 0]],
  ["handL", "forearmL", [0, -D.forearm, 0]],

  ["upperArmR", "chest", [-D.shoulderHalf, D.chest - 0.03, 0]],
  ["forearmR", "upperArmR", [0, -D.upperArm, 0]],
  ["handR", "forearmR", [0, -D.forearm, 0]],

  ["thighL", "hips", [D.hipHalf, 0, 0]],
  ["shinL", "thighL", [0, -D.thigh, 0]],
  ["footL", "shinL", [0, -D.shin, 0]],

  ["thighR", "hips", [-D.hipHalf, 0, 0]],
  ["shinR", "thighR", [0, -D.thigh, 0]],
  ["footR", "shinR", [0, -D.shin, 0]],
];

const FLOOR_Y = 0.012;
const FLOATING_FLOOR_Y = 0.08;
const MAX_LIFT = 0.12;
const MAX_DROP = 0.05;
const UNIT_SCALE = new THREE.Vector3(1, 1, 1);
const IDENTITY_QUAT = new THREE.Quaternion();

// Probe points used by the lightweight grounding solver. This is not full
// physics; it keeps the current primitive body from visually floating or
// penetrating the mat after anatomical joint clamping.
const FLOOR_PROBES = [
  ["hips", [[0, -D.torsoR * 0.72, 0], [0, 0, -D.torsoR * 0.82], [0, 0, D.torsoR * 0.82]]],
  ["spine", [[0, D.spine * 0.5, -D.torsoR * 0.78], [0, D.spine * 0.5, D.torsoR * 0.78]]],
  ["chest", [[0, D.chest * 0.45, -D.torsoR * 0.78], [0, D.chest * 0.45, D.torsoR * 0.78]]],
  ["head", [[0, D.headR * 0.75, -D.headR * 0.9], [0, D.headR * 0.75, D.headR * 0.9]]],
  ["forearmL", [[0, -D.forearm * 0.52, 0]]],
  ["forearmR", [[0, -D.forearm * 0.52, 0]]],
  ["handL", [[0, -D.hand, 0], [0, -D.hand * 0.55, 0.02]]],
  ["handR", [[0, -D.hand, 0], [0, -D.hand * 0.55, 0.02]]],
  ["thighL", [[0, -D.thigh * 0.45, 0]]],
  ["thighR", [[0, -D.thigh * 0.45, 0]]],
  ["shinL", [[0, 0, 0], [0, -D.shin * 0.55, 0]]],
  ["shinR", [[0, 0, 0], [0, -D.shin * 0.55, 0]]],
  ["footL", [[0, -0.025, -0.02], [0, -0.025, D.foot]]],
  ["footR", [[0, -0.025, -0.02], [0, -0.025, D.foot]]],
];

export class Fighter {
  /**
   * @param {object} opts
   * @param {number} opts.color  道着/ラッシュガードの色
   * @param {number} opts.accent 帯・襟・髪のアクセント色
   * @param {"gi"|"nogi"} [opts.mode] 初期モード
   */
  constructor({ color, accent, mode = "gi" }) {
    this.root = new THREE.Group();
    this.joints = {};
    this.mode = mode;
    this._color = color;
    this._accent = accent;

    const gi = new THREE.MeshStandardMaterial({
      color,
      roughness: 0.8,
      metalness: 0.0,
    });
    const giDark = new THREE.MeshStandardMaterial({
      color: new THREE.Color(color).multiplyScalar(0.6),
      roughness: 0.82,
      metalness: 0.0,
    });
    // ラッシュガード: 少し光沢のある別マテリアル (no-gi 胴・上腕)
    const rash = new THREE.MeshStandardMaterial({
      color: new THREE.Color(color).multiplyScalar(0.92),
      roughness: 0.45,
      metalness: 0.05,
    });
    const skin = new THREE.MeshStandardMaterial({
      color: SKIN,
      roughness: 0.6,
      metalness: 0.0,
    });
    const accentMat = new THREE.MeshStandardMaterial({
      color: accent,
      roughness: 0.7,
      metalness: 0.0,
    });
    this.bodyMat = gi;
    this._mats = { gi, giDark, rash, skin, accentMat };

    // gi/no-gi で出し入れするメッシュ群
    this._giOnly = []; // 襟・ラペル・袖口など gi 専用
    this._nogiOnly = []; // ラッシュガードのトリムなど no-gi 専用
    this._sleeveLowerL = null; // 前腕の道着スリーブ (gi=覆う / nogi=肌)
    this._sleeveLowerR = null;
    this._forearmSkinL = null;
    this._forearmSkinR = null;

    // 関節階層を構築
    for (const [name, parent, pos] of SKELETON) {
      const j = new THREE.Object3D();
      j.position.set(pos[0], pos[1], pos[2]);
      this.joints[name] = j;
      if (parent) this.joints[parent].add(j);
      else this.root.add(j);
    }

    this._buildLimbs();
    this._buildTorso();
    this._buildHead();
    this._buildHandsFeet();
    this._buildGiDetails();
    this._buildPelvis();

    this.setMode(mode);

    // 立ち姿を初期ポーズに
    this.root.position.set(0, D.hipY, 0);
    this._targets = {};
    this._rootTargetPos = this.root.position.clone();
    this._rootTargetQuat = this.root.quaternion.clone();
    this.applyPose({}, { immediate: true });
  }

  // --- テーパー付きセグメント (近位 r0 → 遠位 r1) を -Y / +Y 方向に生やす -----
  _taperedLimb(joint, len, r0, r1, mat, dir = "down") {
    const geo = new THREE.CylinderGeometry(r1, r0, len, 16, 1, false);
    // CylinderGeometry は +Y が上端(r1=top)。中心が原点なので半分ずらす。
    const mesh = new THREE.Mesh(geo, mat);
    mesh.castShadow = true;
    if (dir === "down") mesh.position.y = -len / 2;
    else mesh.position.y = len / 2;
    joint.add(mesh);
    // 関節の丸み (近位端に球)
    const ball = new THREE.Mesh(new THREE.SphereGeometry(r0 * 1.02, 14, 12), mat);
    ball.castShadow = true;
    joint.add(ball);
    return mesh;
  }

  _buildLimbs() {
    const { gi, skin } = this._mats;
    const lr = D.limbR;
    // 腕: 上腕 (肩太→肘細), 前腕 (肘→手首細)。前腕は gi/nogi で素材差し替え対象。
    for (const side of ["L", "R"]) {
      this._taperedLimb(this.joints["upperArm" + side], D.upperArm, lr * 1.15, lr * 0.85, gi);
      // 前腕: gi スリーブ (道着) と 肌 の二重メッシュを用意し、表示を切替
      const sleeve = this._taperedLimb(
        this.joints["forearm" + side], D.forearm, lr * 0.85, lr * 0.62, gi,
      );
      const bare = new THREE.Mesh(
        new THREE.CylinderGeometry(lr * 0.6, lr * 0.82, D.forearm, 14),
        skin,
      );
      bare.position.y = -D.forearm / 2;
      bare.castShadow = true;
      this.joints["forearm" + side].add(bare);
      this["_sleeveLower" + side] = sleeve;
      this["_forearmSkin" + side] = bare;
    }
    // 脚: 腿 (太), 脛 (やや細)。常に道着 (gi=ズボン, nogi=ファイトショーツ風で同色)。
    for (const side of ["L", "R"]) {
      this._taperedLimb(this.joints["thigh" + side], D.thigh, lr * 1.55, lr * 1.05, gi);
      this._taperedLimb(this.joints["shin" + side], D.shin, lr * 1.05, lr * 0.72, gi);
    }
  }

  _buildTorso() {
    const { gi, giDark } = this._mats;
    // 腹 (spine): 下が細く上が広い樽
    const belly = new THREE.Mesh(
      new THREE.CylinderGeometry(D.torsoR * 1.02, D.torsoR * 0.9, D.spine, 18),
      gi,
    );
    belly.position.y = D.spine / 2;
    belly.castShadow = true;
    this.joints.spine.add(belly);

    // 胸郭 (chest): 樽 + 肩へ向けて広がる。少し前後に平たく。
    const ribGeo = new THREE.CylinderGeometry(D.torsoR * 1.18, D.torsoR * 1.0, D.chest, 18);
    const rib = new THREE.Mesh(ribGeo, gi);
    rib.position.y = D.chest / 2;
    rib.scale.set(1.0, 1.0, 0.82); // 前後に薄く
    rib.castShadow = true;
    this.joints.chest.add(rib);

    // 肩の盛り上がり (左右の三角筋) — 控えめに
    for (const sx of [-1, 1]) {
      const delt = new THREE.Mesh(new THREE.SphereGeometry(D.limbR * 1.18, 16, 12), gi);
      delt.position.set(sx * (D.shoulderHalf - 0.015), D.chest - 0.04, 0);
      delt.scale.set(1.05, 0.95, 1.0);
      delt.castShadow = true;
      this.joints.chest.add(delt);
    }
    // 鎖骨〜首の付け根 (なだらかな盛り)
    const yoke = new THREE.Mesh(new THREE.SphereGeometry(D.torsoR * 0.62, 16, 12), gi);
    yoke.position.set(0, D.chest - 0.02, 0);
    yoke.scale.set(1.55, 0.5, 0.85);
    this.joints.chest.add(yoke);
  }

  _buildHead() {
    const { skin, giDark, accentMat } = this._mats;
    // 首 (肌)
    const neck = new THREE.Mesh(
      new THREE.CylinderGeometry(0.05, 0.058, D.neck, 14),
      skin,
    );
    neck.position.y = D.neck / 2;
    neck.castShadow = true;
    this.joints.neck.add(neck);

    // 頭 (やや縦長)
    const head = new THREE.Mesh(new THREE.SphereGeometry(D.headR, 24, 18), skin);
    head.position.y = D.headR * 0.78;
    head.scale.set(0.92, 1.05, 0.96);
    head.castShadow = true;
    this.joints.head.add(head);

    // 顎 (前下に小球)
    const jaw = new THREE.Mesh(new THREE.SphereGeometry(D.headR * 0.5, 16, 12), skin);
    jaw.position.set(0, D.headR * 0.4, D.headR * 0.42);
    this.joints.head.add(jaw);

    // 髪 (上半分を覆うキャップ + 後頭部)
    const cap = new THREE.Mesh(
      new THREE.SphereGeometry(D.headR * 1.04, 24, 18, 0, Math.PI * 2, 0, Math.PI * 0.62),
      giDark,
    );
    cap.position.y = D.headR * 0.78;
    cap.scale.set(0.94, 1.05, 0.98);
    this.joints.head.add(cap);

    // 耳
    for (const sx of [-1, 1]) {
      const ear = new THREE.Mesh(new THREE.SphereGeometry(0.022, 10, 8), skin);
      ear.position.set(sx * D.headR * 0.92, D.headR * 0.72, 0);
      ear.scale.set(0.6, 1, 1);
      this.joints.head.add(ear);
    }

    // 鼻 (向き確認用) + 目
    const nose = new THREE.Mesh(new THREE.SphereGeometry(0.02, 10, 8), skin);
    nose.position.set(0, D.headR * 0.66, D.headR * 0.96);
    this.joints.head.add(nose);
    const eyeGeo = new THREE.SphereGeometry(0.015, 8, 8);
    const eyeMat = new THREE.MeshStandardMaterial({ color: 0x1a1a1a, roughness: 0.4 });
    for (const sx of [-1, 1]) {
      const eye = new THREE.Mesh(eyeGeo, eyeMat);
      eye.position.set(sx * 0.045, D.headR * 0.82, D.headR * 0.85);
      this.joints.head.add(eye);
    }
  }

  _buildHandsFeet() {
    const { skin } = this._mats;
    // 手のひら (扁平な箱) + 親指の盛り
    for (const side of ["L", "R"]) {
      const palm = new THREE.Mesh(new THREE.BoxGeometry(0.07, D.hand, 0.035), skin);
      palm.position.y = -D.hand / 2;
      palm.castShadow = true;
      this.joints["hand" + side].add(palm);
      const sx = side === "L" ? 1 : -1;
      const thumb = new THREE.Mesh(new THREE.SphereGeometry(0.026, 10, 8), skin);
      thumb.position.set(sx * 0.045, -D.hand * 0.45, 0.018);
      thumb.scale.set(0.7, 1.2, 0.75);
      thumb.castShadow = true;
      this.joints["hand" + side].add(thumb);
      // 指の塊 (先端の丸み)
      const fingers = new THREE.Mesh(new THREE.SphereGeometry(0.04, 12, 10), skin);
      fingers.position.y = -D.hand;
      fingers.scale.set(0.9, 0.6, 0.45);
      this.joints["hand" + side].add(fingers);
    }
    // 足 (前方へ伸びる扁平な楔)
    for (const side of ["L", "R"]) {
      const foot = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.05, D.foot), skin);
      foot.position.set(0, -0.02, D.foot / 2 - 0.02);
      foot.castShadow = true;
      this.joints["foot" + side].add(foot);
      // 踵
      const heel = new THREE.Mesh(new THREE.SphereGeometry(0.04, 12, 10), skin);
      heel.position.set(0, -0.01, -0.01);
      this.joints["foot" + side].add(heel);
      // つま先の丸み
      const toe = new THREE.Mesh(new THREE.SphereGeometry(0.038, 12, 10), skin);
      toe.position.set(0, -0.015, D.foot - 0.02);
      toe.scale.set(1, 0.7, 0.8);
      this.joints["foot" + side].add(toe);
      for (const tx of [-0.026, 0, 0.026]) {
        const smallToe = new THREE.Mesh(new THREE.SphereGeometry(0.012, 8, 6), skin);
        smallToe.position.set(tx, -0.015, D.foot + 0.012);
        smallToe.castShadow = true;
        this.joints["foot" + side].add(smallToe);
      }
    }
  }

  _buildGiDetails() {
    const { giDark } = this._mats;
    // 道着の襟 (V字ラペル) — 胸の前面に左右の細板。gi 専用。
    const lapelGeo = new THREE.BoxGeometry(0.05, D.chest * 1.0, 0.025);
    for (const sx of [-1, 1]) {
      const lap = new THREE.Mesh(lapelGeo, giDark);
      lap.position.set(sx * 0.055, D.chest * 0.52, D.torsoR * 0.78);
      lap.rotation.z = sx * deg(16);
      this.joints.chest.add(lap);
      this._giOnly.push(lap);
    }
    // 首回りの襟 (リング状)
    const collar = new THREE.Mesh(
      new THREE.TorusGeometry(0.075, 0.018, 8, 20, Math.PI * 1.3),
      giDark,
    );
    collar.rotation.x = Math.PI / 2;
    collar.rotation.z = Math.PI * 0.85;
    collar.position.set(0, D.chest - 0.005, 0.02);
    this.joints.chest.add(collar);
    this._giOnly.push(collar);

    // 袖口 (前腕遠位の帯)。gi 専用。
    for (const side of ["L", "R"]) {
      const cuff = new THREE.Mesh(
        new THREE.CylinderGeometry(D.limbR * 0.72, D.limbR * 0.72, 0.04, 14),
        giDark,
      );
      cuff.position.y = -D.forearm * 0.78;
      this.joints["forearm" + side].add(cuff);
      this._giOnly.push(cuff);
    }
  }

  _buildPelvis() {
    const { gi, accentMat } = this._mats;
    const pelvis = new THREE.Mesh(new THREE.SphereGeometry(D.torsoR * 1.02, 18, 14), gi);
    pelvis.scale.set(1.2, 0.82, 0.9);
    pelvis.castShadow = true;
    this.joints.hips.add(pelvis);

    // 帯 (アクセント色) + 結び目。gi 専用 (no-gi はファイトショーツのウエスト)。
    const belt = new THREE.Mesh(
      new THREE.TorusGeometry(D.torsoR * 1.06, 0.03, 10, 28),
      accentMat,
    );
    belt.rotation.x = Math.PI / 2;
    belt.position.y = 0.03;
    this.joints.hips.add(belt);
    this._giOnly.push(belt);
    const knot = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.055, 0.05), accentMat);
    knot.position.set(0, 0.03, D.torsoR * 1.0);
    this.joints.hips.add(knot);
    this._giOnly.push(knot);

    // no-gi: ウエストバンド (暗色リング)
    const band = new THREE.Mesh(
      new THREE.TorusGeometry(D.torsoR * 1.04, 0.022, 10, 28),
      this._mats.giDark,
    );
    band.rotation.x = Math.PI / 2;
    band.position.y = 0.02;
    this.joints.hips.add(band);
    this._nogiOnly.push(band);
  }

  /** gi / no-gi を切替えて見た目を更新する */
  setMode(mode) {
    this.mode = mode;
    const isGi = mode === "gi";
    for (const m of this._giOnly) m.visible = isGi;
    for (const m of this._nogiOnly) m.visible = !isGi;
    // 胴・上腕の素材: gi=道着 / nogi=ラッシュガード
    const torsoMat = isGi ? this._mats.gi : this._mats.rash;
    for (const name of ["spine", "chest"]) {
      this.joints[name].traverse((o) => {
        if (o.isMesh && (o.material === this._mats.gi || o.material === this._mats.rash))
          o.material = torsoMat;
      });
    }
    for (const side of ["L", "R"]) {
      this.joints["upperArm" + side].traverse((o) => {
        if (o.isMesh && (o.material === this._mats.gi || o.material === this._mats.rash))
          o.material = torsoMat;
      });
      // 前腕: gi=スリーブ表示・肌非表示 / nogi=逆
      if (this["_sleeveLower" + side]) this["_sleeveLower" + side].visible = isGi;
      if (this["_forearmSkin" + side]) this["_forearmSkin" + side].visible = !isGi;
    }
  }

  /**
   * ポーズを目標値としてセット。update() で滑らかに補間する。
   * @param {object} pose { root?:{pos,rot}, joints?:{name:[rx,ry,rz]} }
   * @param {object} [o]  { immediate?:boolean }
   */
  applyPose(pose = {}, { immediate = false } = {}) {
    const joints = pose.joints || {};
    const _e = new THREE.Euler();
    const _q = new THREE.Quaternion();
    for (const name of Object.keys(this.joints)) {
      const e = clampJointEuler(name, joints[name] || [0, 0, 0]);
      _e.set(e[0], e[1], e[2], "XYZ");
      _q.setFromEuler(_e);
      this._targets[name] = _q.clone();
      if (immediate) this.joints[name].quaternion.copy(_q);
    }

    const r = pose.root;
    const baseY = D.hipY;
    if (r && r.pos) {
      this._rootTargetPos.set(r.pos[0], r.pos[1], r.pos[2]);
    } else {
      this._rootTargetPos.set(0, baseY, 0);
    }
    const rot = (r && r.rot) || [0, 0, 0];
    this._rootTargetQuat.setFromEuler(new THREE.Euler(rot[0], rot[1], rot[2], "XYZ"));
    this._solveGroundedRootTarget(Boolean(r?.pos));

    if (immediate) {
      this.root.position.copy(this._rootTargetPos);
      this.root.quaternion.copy(this._rootTargetQuat);
    }
  }

  _targetJointMatrices(rootPos) {
    const matrices = {};
    const rootMatrix = new THREE.Matrix4().compose(rootPos, this._rootTargetQuat, UNIT_SCALE);
    for (const [name, parent, pos] of SKELETON) {
      const localMatrix = new THREE.Matrix4().compose(
        new THREE.Vector3(pos[0], pos[1], pos[2]),
        this._targets[name] || IDENTITY_QUAT,
        UNIT_SCALE,
      );
      matrices[name] = parent
        ? matrices[parent].clone().multiply(localMatrix)
        : rootMatrix.clone().multiply(localMatrix);
    }
    return matrices;
  }

  _floorProbeMinY(rootPos) {
    const matrices = this._targetJointMatrices(rootPos);
    const p = new THREE.Vector3();
    let minY = Infinity;
    for (const [joint, points] of FLOOR_PROBES) {
      const matrix = matrices[joint];
      if (!matrix) continue;
      for (const point of points) {
        p.set(point[0], point[1], point[2]).applyMatrix4(matrix);
        minY = Math.min(minY, p.y);
      }
    }
    return minY;
  }

  _solveGroundedRootTarget(hasExplicitRoot) {
    if (!hasExplicitRoot || this._rootTargetPos.y >= 0.68) return;
    const minY = this._floorProbeMinY(this._rootTargetPos);
    if (!Number.isFinite(minY)) return;

    if (minY < FLOOR_Y) {
      this._rootTargetPos.y += Math.min(FLOOR_Y - minY, MAX_LIFT);
      return;
    }

    if (this._rootTargetPos.y <= 0.26 && minY > FLOATING_FLOOR_Y) {
      this._rootTargetPos.y -= Math.min(minY - FLOATING_FLOOR_Y, MAX_DROP);
    }
  }

  /** dt 秒で目標へ近づける。speed が大きいほど速い。 */
  update(dt, speed = 6) {
    const a = 1 - Math.exp(-speed * dt); // フレームレート非依存の指数イージング
    for (const [name, joint] of Object.entries(this.joints)) {
      const t = this._targets[name];
      if (t) joint.quaternion.slerp(t, a);
    }
    this.root.position.lerp(this._rootTargetPos, a);
    this.root.quaternion.slerp(this._rootTargetQuat, a);
  }
}

export const FIGHTER_DIMS = D;

// 関節ラボ — 単体骨格で関節構造・可動域・「なぜタップするか」を学ぶモード。
// 左: 3D ビュー (青ファイター単体 + 可動域アーク)、右: 関節リストと詳細パネル。
// スタイルは UI 担当が `lab-` クラスへ書く。ここでは構造とクラス名のみ整える。

import * as THREE from "three";
import { deg, JOINTS } from "../anatomy/joints";
import type { Axis, AxisSpec, JointSpec, RigJointName } from "../anatomy/types";
import { DojoScene } from "../render/scene";

export interface JointLabHandle {
  /** タブ切替などレイアウト変更後に呼ぶ */
  refreshSize(): void;
  dispose(): void;
}

const ARC_RADIUS = 0.3;
const ARC_SAFE_COLOR = 0x3ecf7a;
const ARC_DANGER_COLOR = 0xe0453a;
const AXIS_INDEX: Record<Axis, number> = { x: 0, y: 1, z: 2 };

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

interface AxisRow {
  spec: AxisSpec;
  index: number;
  slider: HTMLInputElement;
  valueEl: HTMLElement;
  zoneEl: HTMLElement;
  rootEl: HTMLElement;
}

class JointLab implements JointLabHandle {
  private readonly scene: DojoScene;
  private readonly container: HTMLElement;
  private readonly listEl: HTMLElement;
  private readonly detailEl: HTMLElement;
  private readonly jointButtons = new Map<string, HTMLButtonElement>();

  private selected: JointSpec;
  /** 選択関節の現在オイラー角 (度)。常に [x, y, z]。 */
  private angles: [number, number, number] = [0, 0, 0];
  private selectedAxisIdx = 0;
  private axisRows: AxisRow[] = [];
  private arcLines: THREE.Line[] = [];
  private tapBanner: HTMLElement | null = null;
  private animHandle: number | null = null;
  private disposed = false;

  constructor(container: HTMLElement) {
    this.container = container;
    const { canvas, listEl, detailEl } = this.buildDom();
    this.listEl = listEl;
    this.detailEl = detailEl;
    this.scene = new DojoScene({ canvas, pair: false });
    this.selected = JOINTS[0]!;
    this.buildJointList();
    this.selectJoint(this.selected);
  }

  refreshSize(): void {
    this.scene.refreshSize();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.stopAnimation();
    this.clearArcs();
    this.scene.dispose();
    this.container.replaceChildren();
  }

  private buildDom(): { canvas: HTMLCanvasElement; listEl: HTMLElement; detailEl: HTMLElement } {
    const root = el("div", "lab-root");
    const viewport = el("div", "lab-viewport");
    const canvas = document.createElement("canvas");
    canvas.className = "lab-canvas";
    // レンダラは親要素サイズへ setSize(…, false) するため、canvas 自身の追従指定は動作に必須
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    canvas.style.display = "block";
    viewport.appendChild(canvas);

    const panel = el("div", "lab-panel");
    const listEl = el("div", "lab-joint-list");
    const detailEl = el("div", "lab-detail");
    panel.append(listEl, detailEl);
    root.append(viewport, panel);
    this.container.replaceChildren(root);
    return { canvas, listEl, detailEl };
  }

  private buildJointList(): void {
    for (const spec of JOINTS) {
      const btn = el("button", "lab-joint-btn", spec.jp);
      btn.type = "button";
      btn.addEventListener("click", () => this.selectJoint(spec));
      this.jointButtons.set(spec.id, btn);
      this.listEl.appendChild(btn);
    }
  }

  private selectJoint(spec: JointSpec): void {
    this.stopAnimation();
    this.selected = spec;
    this.angles = [0, 0, 0];
    this.selectedAxisIdx = 0;
    for (const [id, btn] of this.jointButtons) {
      btn.className = id === spec.id ? "lab-joint-btn is-active" : "lab-joint-btn";
    }
    this.scene.blue.highlightJoint(spec.id);
    // アークは即時描画のため、切替時は補間なしで立位へ戻して位置ズレを防ぐ
    this.applyAngles(true);
    this.renderDetail();
    this.redrawArcs();
  }

  private renderDetail(): void {
    const s = this.selected;
    const title = el("h2", "lab-joint-title", s.jp);
    title.appendChild(el("span", "lab-joint-en", ` ${s.en}`));
    const kind = el("p", "lab-joint-kind", s.kindJp);

    const axesEl = el("div", "lab-axes");
    this.axisRows = s.axes.map((axisSpec, i) => this.buildAxisRow(axisSpec, i));
    for (const row of this.axisRows) axesEl.appendChild(row.rootEl);

    const subBtn = el("button", "lab-submission-btn", "サブミッション再現");
    subBtn.type = "button";
    subBtn.addEventListener("click", () => this.playSubmission());

    this.tapBanner = el("div", "lab-tap-banner");
    this.tapBanner.hidden = true;

    const info = el("dl", "lab-info");
    const addTerm = (term: string, desc: string): void => {
      info.appendChild(el("dt", "lab-info-term", term));
      info.appendChild(el("dd", "lab-info-desc", desc));
    };
    addTerm("可動域を制限する構造", s.limitedBy);
    addTerm("限界を超えると", s.failureMode);

    const subs = el("div", "lab-submissions");
    subs.appendChild(el("h3", "lab-submissions-title", "この関節を攻める技"));
    for (const sub of s.submissions) {
      const item = el("div", "lab-submission");
      item.appendChild(el("div", "lab-submission-name", sub.name));
      item.appendChild(el("p", "lab-submission-how", sub.how));
      subs.appendChild(item);
    }

    const children: HTMLElement[] = [title, kind, axesEl, subBtn, this.tapBanner, info, subs];
    if (s.note) children.push(el("p", "lab-joint-note", s.note));
    this.detailEl.replaceChildren(...children);
    for (const row of this.axisRows) this.updateAxisRow(row);
  }

  private buildAxisRow(spec: AxisSpec, index: number): AxisRow {
    const rootEl = el("div", "lab-axis");
    const head = el("div", "lab-axis-head");
    head.appendChild(
      el("span", "lab-axis-motion", `${spec.motion[0]} ⟷ ${spec.motion[1]}`),
    );
    head.appendChild(el("span", "lab-axis-name", `${spec.axis} 軸`));

    const slider = document.createElement("input");
    slider.type = "range";
    slider.className = "lab-axis-slider";
    slider.min = String(spec.rigRangeDeg[0]);
    slider.max = String(spec.rigRangeDeg[1]);
    slider.step = "1";
    slider.value = "0";

    const readout = el("div", "lab-axis-readout");
    const valueEl = el("span", "lab-axis-value");
    const zoneEl = el("span", "lab-axis-zone");
    readout.append(valueEl, zoneEl);

    const row: AxisRow = { spec, index, slider, valueEl, zoneEl, rootEl };
    const onTouch = (): void => {
      if (this.selectedAxisIdx !== index) {
        this.selectedAxisIdx = index;
        this.redrawArcs();
      }
    };
    slider.addEventListener("pointerdown", onTouch);
    slider.addEventListener("focus", onTouch);
    slider.addEventListener("input", () => {
      this.stopAnimation();
      if (this.tapBanner) this.tapBanner.hidden = true;
      onTouch();
      this.angles[AXIS_INDEX[spec.axis]] = Number(slider.value);
      this.applyAngles();
      this.updateAxisRow(row);
    });

    rootEl.append(head, slider, readout);
    return row;
  }

  private updateAxisRow(row: AxisRow): void {
    const value = this.angles[AXIS_INDEX[row.spec.axis]]!;
    row.slider.value = String(Math.round(value));
    row.valueEl.textContent = `${Math.round(value)}°`;
    const [aMin, aMax] = row.spec.anatomicalRangeDeg;
    const safe = value >= aMin && value <= aMax;
    row.zoneEl.textContent = safe ? "安全域" : "限界域 (ここでタップ)";
    row.zoneEl.className = safe ? "lab-axis-zone lab-zone-safe" : "lab-axis-zone lab-zone-danger";
    row.rootEl.className = this.selectedAxisIdx === row.index ? "lab-axis is-active" : "lab-axis";
  }

  private rigJoint(): RigJointName {
    return this.selected.rigJoints[0]!;
  }

  private applyAngles(immediate = false): void {
    // ベースポーズ = 立位 (全関節 identity)。選択関節のオイラー角だけ差し替える。
    this.scene.blue.applyPose(
      {
        joints: {
          [this.rigJoint()]: [deg(this.angles[0]), deg(this.angles[1]), deg(this.angles[2])],
        },
      },
      { immediate },
    );
  }

  // 選択軸の可動域を関節位置中心の円弧で示す。緑=解剖学的安全域、赤=リグ限界までの危険域。
  // 厳密な回転平面の追従より「安全域と危険域が見える」ことを優先し、親フレーム基準で静的に描く。
  private redrawArcs(): void {
    this.clearArcs();
    const axisSpec = this.selected.axes[this.selectedAxisIdx];
    if (!axisSpec) return;

    const jointObj = this.scene.blue.joints[this.rigJoint()];
    this.scene.blue.root.updateWorldMatrix(true, true);
    const center = jointObj.getWorldPosition(new THREE.Vector3());
    const parentQuat = (jointObj.parent ?? this.scene.blue.root).getWorldQuaternion(
      new THREE.Quaternion(),
    );

    const axisLocal: Record<Axis, THREE.Vector3> = {
      x: new THREE.Vector3(1, 0, 0),
      y: new THREE.Vector3(0, 1, 0),
      z: new THREE.Vector3(0, 0, 1),
    };
    // 基準方向: 四肢は静止時ローカル -Y に伸びる。y 軸回転は -Y と平行になるため +Z を使う
    const refLocal = axisSpec.axis === "y" ? new THREE.Vector3(0, 0, 1) : new THREE.Vector3(0, -1, 0);
    const a = axisLocal[axisSpec.axis].applyQuaternion(parentQuat).normalize();
    const r = refLocal.applyQuaternion(parentQuat).normalize();
    const b = new THREE.Vector3().crossVectors(a, r).normalize();

    const addArc = (fromDeg: number, toDeg: number, color: number): void => {
      if (toDeg - fromDeg < 1e-3) return;
      const steps = Math.max(8, Math.ceil((toDeg - fromDeg) / 4));
      const points: THREE.Vector3[] = [];
      for (let i = 0; i <= steps; i++) {
        const t = deg(fromDeg + ((toDeg - fromDeg) * i) / steps);
        points.push(
          new THREE.Vector3()
            .addScaledVector(r, Math.cos(t) * ARC_RADIUS)
            .addScaledVector(b, Math.sin(t) * ARC_RADIUS)
            .add(center),
        );
      }
      const line = new THREE.Line(
        new THREE.BufferGeometry().setFromPoints(points),
        new THREE.LineBasicMaterial({ color }),
      );
      this.scene.scene.add(line);
      this.arcLines.push(line);
    };

    const [rigMin, rigMax] = axisSpec.rigRangeDeg;
    const safeMin = Math.max(axisSpec.anatomicalRangeDeg[0], rigMin);
    const safeMax = Math.min(axisSpec.anatomicalRangeDeg[1], rigMax);
    addArc(safeMin, safeMax, ARC_SAFE_COLOR);
    addArc(rigMin, safeMin, ARC_DANGER_COLOR);
    addArc(safeMax, rigMax, ARC_DANGER_COLOR);

    for (const row of this.axisRows) this.updateAxisRow(row);
  }

  private clearArcs(): void {
    for (const line of this.arcLines) {
      this.scene.scene.remove(line);
      line.geometry.dispose();
      (line.material as THREE.Material).dispose();
    }
    this.arcLines = [];
  }

  /** リグ限界のうち anatomicalRange を超える側があればその方向を優先して選ぶ */
  private submissionTarget(): { axisIdx: number; targetDeg: number } {
    for (let i = 0; i < this.selected.axes.length; i++) {
      const axis = this.selected.axes[i]!;
      const [rigMin, rigMax] = axis.rigRangeDeg;
      const [aMin, aMax] = axis.anatomicalRangeDeg;
      if (rigMin < aMin) return { axisIdx: i, targetDeg: rigMin };
      if (rigMax > aMax) return { axisIdx: i, targetDeg: rigMax };
    }
    const first = this.selected.axes[0]!;
    return { axisIdx: 0, targetDeg: first.rigRangeDeg[1] };
  }

  private playSubmission(): void {
    this.stopAnimation();
    if (this.tapBanner) this.tapBanner.hidden = true;

    const { axisIdx, targetDeg } = this.submissionTarget();
    const axisSpec = this.selected.axes[axisIdx]!;
    if (this.selectedAxisIdx !== axisIdx) {
      this.selectedAxisIdx = axisIdx;
      this.redrawArcs();
    }

    const angleIdx = AXIS_INDEX[axisSpec.axis];
    const startDeg = this.angles[angleIdx]!;
    const durationMs = 1400;
    const startTime = performance.now();
    const failureMode = this.selected.failureMode;

    const step = (now: number): void => {
      const t = Math.min((now - startTime) / durationMs, 1);
      const eased = 1 - Math.pow(1 - t, 3);
      this.angles[angleIdx] = startDeg + (targetDeg - startDeg) * eased;
      this.applyAngles();
      const row = this.axisRows[axisIdx];
      if (row) this.updateAxisRow(row);
      if (t < 1) {
        this.animHandle = requestAnimationFrame(step);
        return;
      }
      this.animHandle = null;
      if (this.tapBanner) {
        this.tapBanner.textContent = `ここでタップ — ${failureMode}`;
        this.tapBanner.hidden = false;
      }
    };
    this.animHandle = requestAnimationFrame(step);
  }

  private stopAnimation(): void {
    if (this.animHandle !== null) {
      cancelAnimationFrame(this.animHandle);
      this.animHandle = null;
    }
  }
}

export function createJointLab(container: HTMLElement): JointLabHandle {
  return new JointLab(container);
}

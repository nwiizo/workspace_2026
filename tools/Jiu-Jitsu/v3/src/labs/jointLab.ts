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
  setActive(active: boolean): void;
  dispose(): void;
}

const ARC_RADIUS = 0.3;
const ARC_COLOR = 0x87b7ee;
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

  setActive(active: boolean): void { this.scene.setActive(active); }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
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

    const subBtn = el("button", "lab-submission-btn", "表示を元に戻す");
    subBtn.type = "button";
    subBtn.addEventListener("click", () => this.selectJoint(this.selected));
    const modelNote = el("p", "lab-joint-note", "角度と円弧は簡易リグを動かすための表示値です。人体の可動域や安全限界とは一致しません。タップは表示の限界や痛みを待たずに行います。");

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

    const children: HTMLElement[] = [title, kind, modelNote, axesEl, subBtn, info, subs];
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
    slider.setAttribute("aria-label", `${this.selected.jp} ${spec.motion[0]}・${spec.motion[1]}`);

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
    row.zoneEl.textContent = "表示用の角度・安全性は判定しません";
    row.zoneEl.className = "lab-axis-zone";
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

  // リグの表示範囲を円弧で示す。人体の安全域とは対応させない。
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
    addArc(rigMin, rigMax, ARC_COLOR);

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

}

export function createJointLab(container: HTMLElement): JointLabHandle {
  return new JointLab(container);
}

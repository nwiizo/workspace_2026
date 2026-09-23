import { legPoints, momentArm, supportMargin, movementTarget, type BodyExperiment } from "../anatomy/mechanics";
import { loadProgress, saveProgress, type KeyValueStore } from "../engine/storage";
import { recordResult } from "../engine/srs";
import { h } from "../ui/dom";
import { createJointLab, type JointLabHandle } from "./jointLab";

export type { BodyExperiment } from "../anatomy/mechanics";
type Layer = "bones" | "muscles" | "both";

const EXPERIMENTS = {
  lever: {
    title: "肘からの距離で、作用はどう変わる？",
    short: "腕とテコ", task: "金色の点か手首を動かし、下向きの力の線を青い帯へ近づけよう。",
    goal: "力の線と肘の水平距離を12cm以下にする。位置を近づけても、腕を傾けてもよい。",
    initial: [20, 0],
    controls: [["力を受ける位置（肘から）", 10, 40, "cm"], ["前腕の傾き（水平から）", 0, 60, "°"]],
    bones: "骨は力を伝える支柱にも、関節を中心に回るテコにもなる。図は上腕骨と前腕の2本の骨（橈骨・尺骨）を簡略化している。",
    muscles: "肘を曲げるときは上腕二頭筋・上腕筋など、伸ばすときは上腕三頭筋などが働く。動かさず支えるときにも筋肉は張力を出す。",
    bjj: "フレームでは骨の並びと力の向きが大切。骨だけで支えているわけではなく、筋肉が関節の位置を保っている。腕を遠くへ出すと、相手に回される作用も変わる。",
    limit: "肘を固定し、一定の下向きの力だけを扱う2Dモデル。筋力・接触圧・肩の動きは計算しない。回転作用が小さくても、関節に力がかからないわけではない。",
    source: "https://openstax.org/books/college-physics-2e/pages/9-6-forces-and-torques-in-muscles-and-joints",
  },
  base: {
    title: "支えの幅と重心を、別々に変える",
    short: "支えとバランス", task: "重心の点か左右の支えを動かし、重心の投影を支えの中へ戻そう。",
    goal: "重心の投影を支持範囲の内側へ入れ、端まで10cmの余裕を作る。支えと重心のどちらを動かしてもよい。",
    initial: [20, 20],
    controls: [["左右の支えの幅", 20, 80, "cm"], ["重心の投影の位置", -50, 50, "cm"]],
    bones: "床についた膝や足などが支持点になる。その間の範囲に、重さの中心（重心）から下ろした線があるかを見る。",
    muscles: "腹部・背部・股関節まわりの筋肉は、体が流れないよう姿勢を調整する。静止して見えても筋肉が休んでいるとは限らない。",
    bjj: "相手を返すときは、体重を動かすことと、手足の支えを使わせないことが組み合わさる。膝を広げても、別方向の崩しまで防げるとは限らない。",
    limit: "支持範囲を左右の一本の線として扱う静止モデル。重心位置は入力値で、体の絵から推定していない。摩擦・相手の力・前後方向・手をつく反応は計算しない。",
    source: "https://openstax.org/books/college-physics-2e/pages/9-3-stability",
  },
  leg: {
    title: "膝を曲げることと、膝を戻すこと",
    short: "股関節と膝", task: "膝と足首の点を動かして、脚を折りたたみながら胸の方へ戻そう。",
    goal: "膝を小さい青枠へ、足首を大きい青枠へ同時に収める。股関節と膝を別々に動かしてみよう。",
    initial: [0, 0],
    controls: [["股関節を曲げる", 0, 110, "°"], ["膝を曲げる", 0, 130, "°"]],
    bones: "骨盤と大腿骨は股関節でつながり、大腿骨とすねは膝でつながる。膝だけを曲げても、大腿骨の先にある膝頭の位置は変わらない。",
    muscles: "股関節を曲げる動きには腸腰筋など、膝を曲げる動きにはハムストリングスなどが関わる。大腿四頭筋は膝を伸ばす。図の色は筋肉の位置の目安で、活動量ではない。",
    bjj: "ガードへ膝を戻すには、膝を折りたたむだけでなく股関節や骨盤を動かす必要がある。相手との間に空間があるかは、ポジション練習で確かめよう。",
    limit: "骨盤を固定した平面内の2本リンク。大腿とすねは各40cmの例示値。床・相手との衝突、骨盤の回転や個人差は再現していない。実際の関節の安全角度を示す図ではない。",
    source: "https://openstax.org/books/anatomy-and-physiology-2e/pages/9-5-types-of-body-movements",
  },
} as const;

const SVG_NS = "http://www.w3.org/2000/svg";
function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>, text?: string): SVGElementTagNameMap[K] {
  const element = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) element.setAttribute(key, String(value));
  if (text) element.textContent = text;
  return element;
}
const line = (x1: number, y1: number, x2: number, y2: number, color = "#e5dec7", width = 9): SVGLineElement =>
  svg("line", { x1, y1, x2, y2, stroke: color, "stroke-width": width, "stroke-linecap": "round" });
const label = (x: number, y: number, text: string, color = "#d9e4f6"): SVGTextElement =>
  svg("text", { x, y, fill: color, "font-size": 13 }, text);
const joint = (x: number, y: number): SVGCircleElement => svg("circle", { cx: x, cy: y, r: 8, fill: "#92bdfb", stroke: "#233a5b", "stroke-width": 3 });

export class BodyLab {
  private selected: BodyExperiment = "lever";
  private layer: Layer = "both";
  private values: [number, number] = [20, 0];
  private before: [number, number] = [20, 0];
  private completed = false;
  private dragging: number | null = null;
  private readonly diagram = svg("svg", { viewBox: "0 0 600 380", role: "img", "aria-label": "操作に応じて骨格と作用が変わる模式図" });
  private readonly result = h("output", { class: "body-result", attrs: { "aria-live": "polite" } });
  private readonly compare = h("p", { class: "body-compare" });
  private readonly feedback = h("p", { class: "practice-hint" });
  private readonly controls = h("div", { class: "body-controls" });
  private readonly lesson = h("section", { class: "body-explanation" });
  private readonly sliderRows: { slider: HTMLInputElement; readout: HTMLOutputElement }[] = [];
  private jointLab: JointLabHandle | null = null;
  private jointDetails: HTMLDetailsElement | null = null;
  private active = true;

  constructor(private readonly root: HTMLElement, private readonly returnToPractice: (experiment: BodyExperiment) => void, private readonly store: KeyValueStore, private readonly changed: () => void) {
    this.diagram.addEventListener("pointerdown", (event) => {
      const control = (event.target as SVGElement).closest("[data-control]")?.getAttribute("data-control");
      if (control === undefined || control === null) return;
      this.dragging = Number(control);
      this.diagram.setPointerCapture(event.pointerId);
      this.drag(event);
    });
    this.diagram.addEventListener("pointermove", (event) => this.drag(event));
    const release = (): void => { this.dragging = null; };
    this.diagram.addEventListener("pointerup", release);
    this.diagram.addEventListener("pointercancel", release);
    this.diagram.addEventListener("lostpointercapture", release);
    this.render();
  }

  open(experiment: BodyExperiment): void {
    this.selected = experiment;
    this.reset();
    this.render();
  }

  setActive(active: boolean): void {
    this.active = active;
    this.jointLab?.setActive(active && !!this.jointDetails?.open);
  }

  refreshSize(): void { this.jointLab?.refreshSize(); }

  private reset(): void {
    const initial = EXPERIMENTS[this.selected].initial;
    this.values = [initial[0], initial[1]];
    this.before = [...this.values];
    this.completed = false;
    this.dragging = null;
  }

  private render(): void {
    this.jointLab?.dispose();
    this.jointLab = null;
    const exp = EXPERIMENTS[this.selected];
    const tabs = h("nav", { class: "body-tabs", attrs: { "aria-label": "体の実験" } });
    for (const [id, experiment] of Object.entries(EXPERIMENTS)) tabs.append(h("button", {
      class: "btn", text: experiment.short, attrs: { "aria-pressed": String(this.selected === id) },
      onClick: () => this.open(id as BodyExperiment),
    }));
    const layers = h("div", { class: "body-layers", attrs: { role: "group", "aria-label": "図に表示するもの" } });
    for (const [id, text] of [["bones", "骨格"], ["muscles", "筋肉の位置"], ["both", "重ねる"]] as const) {
      layers.append(h("button", { class: "btn", text, attrs: { "aria-pressed": String(this.layer === id) }, onClick: () => {
        this.layer = id;
        for (const [index, button] of Array.from(layers.querySelectorAll("button")).entries()) button.setAttribute("aria-pressed", String(["bones", "muscles", "both"][index] === id));
        this.draw();
      } }));
    }
    this.buildControls();
    this.lesson.replaceChildren(
      h("h3", { text: "骨格は何をしている？" }), h("p", { text: exp.bones }),
      h("h3", { text: "筋肉は何をしている？" }), h("p", { text: exp.muscles }),
      h("h3", { text: "柔術では、どこを見る？" }), h("p", { text: exp.bjj }),
      h("button", { class: "btn btn-start", text: "この動きをポジション練習で使う", onClick: () => this.returnToPractice(this.selected) }),
      h("p", { class: "body-limit", text: exp.limit }),
      h("a", { text: "しくみの参考：OpenStax", attrs: { href: exp.source, target: "_blank", rel: "noopener noreferrer" } }),
      h("p", {}, h("a", { text: "筋肉と骨の関係：OpenStax", attrs: { href: "https://openstax.org/books/anatomy-and-physiology-2e/pages/11-1-interactions-of-skeletal-muscles-their-fascicle-arrangement-and-their-lever-systems", target: "_blank", rel: "noopener noreferrer" } })),
    );
    const detailBody = h("div", { class: "body-joint-detail" });
    this.jointDetails = h("details", { class: "body-joint-reference" }, h("summary", { text: "関節ごとの構造と動きを調べる" }), detailBody);
    this.jointDetails.addEventListener("toggle", () => {
      if (this.jointDetails?.open && !this.jointLab) this.jointLab = createJointLab(detailBody);
      this.jointLab?.setActive(this.active && !!this.jointDetails?.open);
      if (this.jointDetails?.open) requestAnimationFrame(() => this.jointLab?.refreshSize());
    });
    this.root.replaceChildren(h("div", { class: "body-lab" },
      h("p", { class: "eyebrow", text: "MOVEMENT DOJO / 動かして、柔術をつかむ" }), h("h2", { text: exp.title }),
      h("p", { class: "body-lead", text: "丸い点をドラッグして動かそう。支えが変わる。バランスが変わる。脚の届く位置が変わる。スライダーと矢印キーでも操作できます。" }), tabs,
      h("div", { class: "body-layout" },
        h("section", { class: "body-workbench" }, layers, this.diagram, this.result, this.compare, this.controls,
          h("div", { class: "body-tools" }, h("button", { class: "btn", text: "今の状態を比較元にする", onClick: () => { this.before = [...this.values]; this.draw(); } }),
            h("button", { class: "btn", text: "最初の状態へ", onClick: () => this.open(this.selected) }))),
        h("div", {}, h("section", { class: "body-task" }, h("h3", { text: "動かしてみよう" }), h("p", { text: exp.task }), h("p", { class: "body-goal", text: exp.goal }), this.feedback,
          h("p", { class: "body-task-note", text: "青枠と数値はこの練習の目標です。人体の安全限界や、実技の成功率を示すものではありません。" }),
          h("button", { class: "btn", text: "次の動きを試す →", onClick: () => this.open(this.selected === "lever" ? "base" : this.selected === "base" ? "leg" : "lever") })), this.lesson)),
      this.jointDetails,
    ));
    this.draw();
  }

  private buildControls(): void {
    this.controls.replaceChildren();
    this.sliderRows.length = 0;
    EXPERIMENTS[this.selected].controls.forEach(([text, min, max, unit], index) => {
      const slider = h("input", { attrs: { type: "range", min: String(min), max: String(max), step: "1", value: String(this.values[index]), "aria-label": text } });
      const readout = h("output");
      const row = h("label", { class: "body-slider" }, h("span", { text }), slider, readout);
      this.sliderRows.push({ slider, readout });
      slider.addEventListener("input", () => { this.values[index] = Number(slider.value); this.updateMovement(); });
      readout.dataset.unit = unit;
      this.controls.append(row);
    });
  }

  private describe(values: readonly [number, number]): string {
    if (this.selected === "lever") return `肘を回そうとする作用：${(momentArm(values[0], values[1]) / 20).toFixed(2)}倍（水平・20cmを1とする）`;
    if (this.selected === "base") {
      const margin = supportMargin(values[0], values[1]);
      return Math.abs(margin) < 0.01 ? "重心の投影：支えの境界上" : margin > 0 ? `重心の投影：支えの内側 / 端まで${margin.toFixed(0)}cm` : `重心の投影：支えの外側 / ${(-margin).toFixed(0)}cm外れた`;
    }
    const points = legPoints(values[0], values[1]);
    return `骨盤から見た膝：横${points.knee.x.toFixed(0)}cm / 上${points.knee.y.toFixed(0)}cm`;
  }

  private draw(): void {
    this.sliderRows.forEach(({ slider, readout }, index) => { slider.value = String(this.values[index]); readout.textContent = `${this.values[index]}${readout.dataset.unit}`; });
    this.diagram.replaceChildren();
    this.diagram.append(svg("rect", { width: 600, height: 380, rx: 12, fill: "#172334" }));
    if (this.selected === "lever") this.drawLever();
    else if (this.selected === "base") this.drawBase();
    else this.drawLeg();
    this.result.textContent = this.describe(this.values);
    this.compare.textContent = `比較元：${this.describe(this.before)}`;
    this.diagram.setAttribute("aria-label", `${EXPERIMENTS[this.selected].title}。${this.describe(this.values)}`);
    const reached = movementTarget(this.selected, this.values);
    this.feedback.textContent = reached
      ? `目標に届いた！ ${this.selected === "lever" ? "力の線が肘へ近づき、回そうとする作用が小さくなった。別の角度や距離でも試してみよう。" : this.selected === "base" ? "重心の投影が支えの内側へ戻り、余裕ができた。どちらを動かしても同じ結果になるか試してみよう。" : "股関節で膝の位置を変え、膝を曲げて脚を短く収められた。二つの関節が違う役割をしている。"}`
      : this.completed ? "今は目標の外へ動きました。達成した記録は残っています。前の形と比べて、何が変わったか見てみよう。" : "動かす点を変えて、作用を確かめよう。やり直しや時間制限はありません。";
    this.feedback.setAttribute("role", "status");
    this.feedback.classList.toggle("body-achieved", reached);
  }

  private updateMovement(): void {
    if (!this.completed && movementTarget(this.selected, this.values)) {
      this.completed = true;
      const progress = loadProgress(this.store);
      const key = `body:${this.selected}`;
      if (!progress.srs[key]) {
        saveProgress(this.store, { ...progress, srs: recordResult(progress.srs, key, true, Date.now()) });
        this.changed();
      }
    }
    this.draw();
  }

  private drag(event: PointerEvent): void {
    if (this.dragging === null) return;
    const matrix = this.diagram.getScreenCTM();
    if (!matrix) return;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
    let value: number;
    if (this.selected === "lever") {
      const rad = this.values[1] * Math.PI / 180;
      value = this.dragging === 0 ? ((point.x - 220) * Math.cos(rad) + (230 - point.y) * Math.sin(rad)) / 3.5 : Math.atan2(230 - point.y, point.x - 220) * 180 / Math.PI;
    } else if (this.selected === "base") {
      value = this.dragging === 0 ? Math.abs(point.x - 300) / 1.25 : (point.x - 300) / 2.5;
    } else {
      const knee = legPoints(this.values[0], this.values[1]).knee;
      value = this.dragging === 0 ? Math.atan2(210 - point.y, point.x - 280) * 180 / Math.PI : this.values[0] - Math.atan2(210 - knee.y * 2.2 - point.y, point.x - 280 - knee.x * 2.2) * 180 / Math.PI;
    }
    const [, min, max] = EXPERIMENTS[this.selected].controls[this.dragging]!;
    this.values[this.dragging] = Math.round(Math.max(min, Math.min(max, value)));
    this.updateMovement();
  }

  private handle(x: number, y: number, control: number, color = "#a9d0ff"): void {
    this.diagram.append(svg("circle", { cx: x, cy: y, r: 15, fill: color, stroke: "#25466d", "stroke-width": 3, "data-control": control, class: "body-handle" }));
  }

  private drawLever(): void {
    const [distance, angle] = this.values;
    const rad = angle * Math.PI / 180;
    const elbow = { x: 220, y: 230 };
    const hand = { x: 220 + 150 * Math.cos(rad), y: 230 - 150 * Math.sin(rad) };
    const contact = { x: 220 + distance * 3.5 * Math.cos(rad), y: 230 - distance * 3.5 * Math.sin(rad) };
    this.diagram.append(svg("rect", { x: 220, y: 60, width: 42, height: 230, fill: "#79b6f8", opacity: .12 }));
    const bones = svg("g", { opacity: this.layer === "muscles" ? 0.2 : 1 });
    bones.append(line(220, 90, elbow.x, elbow.y), line(elbow.x, elbow.y - 5, hand.x, hand.y - 5, "#e5dec7", 6), line(elbow.x, elbow.y + 5, hand.x, hand.y + 5, "#b8c9df", 6), joint(220, 90), joint(elbow.x, elbow.y), joint(hand.x, hand.y));
    this.diagram.append(bones);
    if (this.layer !== "bones") {
      this.diagram.append(line(240, 105, 240, 183 - angle * .45, "#ee9c91", 20), line(240, 183 - angle * .45, 220 + 45 * Math.cos(rad), 230 - 45 * Math.sin(rad), "#eac4b3", 3),
        line(199, 105, 199, 210, "#b296d6", 16), label(55, 142, "上腕三頭筋", "#d5b7f6"), label(280, 104, "上腕二頭筋など", "#ffbbb1"));
    }
    this.diagram.append(label(170, 262, "肘：支点"), label(142, 73, "肩側は固定"), label(390, 251, "前腕の骨"),
      line(contact.x, contact.y - 65, contact.x, contact.y - 14, "#edb573", 4), svg("path", { d: `M ${contact.x - 7} ${contact.y - 23} L ${contact.x} ${contact.y - 10} L ${contact.x + 7} ${contact.y - 23}`, stroke: "#edb573", fill: "none", "stroke-width": 4 }),
      label(385, 155, "同じ下向きの力", "#f6c88f"), line(elbow.x, 290, contact.x, 290, "#7fb4fc", 3), label(210, 320, `力の線までの水平距離：${momentArm(distance, angle).toFixed(1)}cm`), label(30, 358, "筋肉は力を出して骨を引く。腱が骨にその力を伝える。"));
    this.handle(hand.x, hand.y, 1);
    this.handle(contact.x, contact.y, 0, "#f6c88f");
  }

  private drawBase(): void {
    const [width, offset] = this.values;
    const left = 300 - width * 2.5;
    const right = 300 + width * 2.5;
    const center = 300 + offset * 2.5;
    const margin = supportMargin(width, offset);
    const color = margin > 0 ? "#8ed2b0" : "#f6be85";
    this.diagram.append(line(35, 300, 565, 300, "#52647b", 2), line(left, 300, right, 300, "#74b3d2", 12),
      svg("rect", { x: 255, y: 170, width: 90, height: 32, rx: 14, fill: "#7194c4", opacity: .3 }),
      svg("path", { d: `M ${left} 287 L 280 199 L 320 199 L ${right} 287`, stroke: "#e5dec7", "stroke-width": 10, fill: "none", opacity: this.layer === "muscles" ? .2 : 1 }));
    if (this.layer !== "bones") this.diagram.append(svg("ellipse", { cx: 300, cy: 172, rx: 32, ry: 22, fill: "#e4a396", opacity: .65 }), label(346, 187, "体幹・股関節まわり", "#f1b4a8"));
    this.diagram.append(joint(left, 287), joint(right, 287), line(center, 82, center, 307, color, 2),
      svg("circle", { cx: center, cy: 105, r: 12, fill: color }), svg("circle", { cx: center, cy: 300, r: 7, fill: color }),
      label(Math.min(center + 18, 400), 94, "重心（入力した位置）", color), label(left - 20, 336, "左の支え"), label(right - 30, 336, "右の支え"), label(25, 365, "青い線＝左右の支持範囲。縦線＝重心から下ろした線。"));
    this.handle(left, 287, 0); this.handle(right, 287, 0); this.handle(center, 105, 1, color);
  }

  private drawLeg(): void {
    const p = legPoints(this.values[0], this.values[1]);
    const initial = legPoints(this.before[0], this.before[1]);
    const hip = { x: 280, y: 210 };
    const project = (point: { x: number; y: number }) => ({ x: hip.x + point.x * 2.2, y: hip.y - point.y * 2.2 });
    const knee = project(p.knee); const ankle = project(p.ankle);
    const ghostKnee = project(initial.knee); const ghostAnkle = project(initial.ankle);
    this.diagram.append(svg("rect", { x: 236, y: 100, width: 88, height: 44, rx: 5, fill: "#7fbbff", "fill-opacity": .14, stroke: "#85b8f3", "stroke-dasharray": "4 5" }),
      svg("rect", { x: 280, y: 100, width: 121, height: 77, rx: 5, fill: "#7fbbff", "fill-opacity": .07, stroke: "#85b8f3", "stroke-dasharray": "4 5" }));
    const ghost = svg("g", { opacity: .23, "stroke-dasharray": "5 6" });
    ghost.append(line(hip.x, hip.y, ghostKnee.x, ghostKnee.y, "#abc6eb", 5), line(ghostKnee.x, ghostKnee.y, ghostAnkle.x, ghostAnkle.y, "#abc6eb", 5));
    this.diagram.append(ghost, label(28, 38, "横から見た片脚 / 骨盤は固定"), svg("ellipse", { cx: 244, cy: 210, rx: 35, ry: 28, fill: "#759bc8", opacity: .5 }), line(120, 210, 224, 210, "#87a6cc", 22), svg("circle", { cx: 87, cy: 205, r: 23, fill: "#9bb7d9" }));
    const bones = svg("g", { opacity: this.layer === "muscles" ? .2 : 1 });
    bones.append(line(hip.x, hip.y, knee.x, knee.y), line(knee.x, knee.y, ankle.x, ankle.y), joint(hip.x, hip.y), joint(knee.x, knee.y), joint(ankle.x, ankle.y));
    this.diagram.append(bones);
    if (this.layer !== "bones") {
      this.diagram.append(line(250, 190, hip.x + (knee.x - hip.x) * .3, hip.y + (knee.y - hip.y) * .3 - 10, "#f0aa99", 14),
        line(hip.x + 8, hip.y + 16, knee.x - 5, knee.y + 14, "#baa0de", 11),
        label(26, 64, "腸腰筋など：股関節を曲げる", "#ffc0b1"), label(26, 86, "ハムストリングスなど：膝を曲げる", "#d9bffa"));
    }
    this.diagram.append(label(222, 264, "骨盤"), label(hip.x - 35, hip.y - 34, "股関節"), label(knee.x + 10, knee.y - 12, "膝"), label(ankle.x + 10, ankle.y + 18, "足首"), label(26, 348, "薄い点線＝比較元。膝と足首の位置を見比べよう。"));
    this.handle(knee.x, knee.y, 0); this.handle(ankle.x, ankle.y, 1);
  }
}

import { DUEL_ACTIONS, DuelEngine, type DuelAction, type DuelState } from "../engine/duel";
import { BodyScene } from "../render/bodyScene";
import { duelPair } from "../render/duelPose";
import { h } from "./dom";
import type { KeyValueStore } from "../engine/storage";
import type { DojoDeps } from "./dojoTab";

const POSITIONS = { guard: "ガード", side: "サイド", mount: "マウント", back: "バック" };
const situation = (s: DuelState) => `${s.top === "blue" ? "青" : "赤"}が${s.position === "back" ? "バックを保持" : `${POSITIONS[s.position]}の上`}`;

export class DuelTab {
  readonly root: HTMLElement;
  private readonly scene: BodyScene;
  private game = new DuelEngine();
  private readonly hud = h("div", { class: "duel-hud", attrs: { "aria-live": "polite" } });
  private readonly panel = h("div", { class: "dojo-panel" });
  private readonly log = h("ol", { class: "duel-history", attrs: { "aria-label": "動きの変遷" } });
  private readonly caption = h("p", { class: "practice-caption" });
  private readonly message = h("p", { class: "duel-message", attrs: { role: "status" } });
  private readonly start = h("select", { attrs: { "aria-label": "対戦の開始位置" } },
    ...[["guard-bottom", "ガードの下から"], ["guard-top", "ガードの上から"], ["mount-bottom", "マウントの下から"], ["side-top", "サイドの上から"]].map(([value, text]) => h("option", { text, attrs: { value: value! } })));
  private readonly style = h("select", { attrs: { "aria-label": "相手の戦い方" } }, h("option", { text: "前へ出る相手", attrs: { value: "pressure" } }), h("option", { text: "返しを狙う相手", attrs: { value: "counter" } }));
  private active = false;
  private busy = false;
  private historyView = false;
  private pending: { state: DuelState; text: string }[] = [];
  private timer: number | null = null;
  private bones = false;
  private faded = false;

  constructor(_store: KeyValueStore, _deps: DojoDeps) {
    const canvas = h("canvas", { class: "dojo-canvas", attrs: { "aria-label": "対戦の二人。青が自分、赤が相手。ドラッグで視点変更。" } });
    const pins = h("div", { class: "practice-pins" });
    const cameras = h("div", { class: "practice-cameras" });
    for (const [label, view] of [["斜め", "diagonal"], ["横", "side"], ["真上", "top"]] as const) cameras.append(h("button", { class: "btn", text: label, onClick: () => this.scene.setCamera(view) }));
    const display = h("div", { class: "practice-display" });
    const bones = h("input", { attrs: { type: "checkbox" } }); bones.addEventListener("change", () => { this.bones = bones.checked; this.scene.setDisplay(this.bones, this.faded, true); });
    const faded = h("input", { attrs: { type: "checkbox" } }); faded.addEventListener("change", () => { this.faded = faded.checked; this.scene.setDisplay(this.bones, this.faded, true); });
    display.append(h("label", {}, bones, "骨格を見る"), h("label", {}, faded, "相手を透かす"));
    this.root = h("div", { class: "duel" },
      h("div", { class: "duel-settings" }, h("h2", { text: "応用ロール — 青 vs 赤" }), this.start, this.style,
        h("button", { class: "btn btn-start", text: "新しい対戦", onClick: () => this.newGame() })),
      this.hud,
      h("div", { class: "dojo-stage-wrap" }, h("div", { class: "roll-observation" }, display,
        h("div", { class: "duel-view" }, canvas, pins, cameras, this.caption), this.message), this.panel),
      h("section", { class: "duel-review" },
        h("div", { class: "duel-history-actions" }, h("h3", { text: "動きの変遷" }), h("button", { class: "btn", text: "攻防を順番に再生", onClick: () => this.replay() }),
          h("button", { class: "btn", text: "対戦の現在へ戻る", onClick: () => this.live() })), this.log),
      h("p", { class: "practice-model-note", text: "12手で区切る戦術シミュレーション。支配・準備・余力はゲーム用の数値です。実際の強さや大会の採点ではありません。返し・パス・極めへの移行は局面表示で、物理演算による連続動作ではありません。" }));
    this.scene = new BodyScene(canvas, pins); this.render(); this.show(this.game.state, "青が自分、赤が相手。行動すると相手も反応します。");
  }
  refreshSize(): void { this.scene.refreshSize(); }
  setActive(active: boolean): void {
    this.active = active; this.scene.setActive(active);
    if (!active && this.timer !== null) { clearTimeout(this.timer); this.timer = null; }
    if (active && this.busy && this.timer === null) this.nextBeat();
  }
  handleKey(event: KeyboardEvent): void {
    if (event.repeat || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || event.target instanceof HTMLElement && event.target.closest("input,select,textarea,[contenteditable=true]")) return;
    if (/^[1-9]$/.test(event.key)) { const action = this.game.available()[Number(event.key) - 1]; if (action) { event.preventDefault(); this.act(action); } }
  }
  private newGame(): void {
    if (this.game.state.turn && !this.game.state.ended && !window.confirm("進行中の対戦を終了して、新しく始めますか？")) return;
    this.cancel(); this.game = new DuelEngine(Math.floor(Math.random() * 2 ** 31), this.style.value as "pressure" | "counter", this.start.value as "guard-bottom" | "guard-top" | "mount-bottom" | "side-top");
    this.historyView = false; this.render(); this.show(this.game.state, "新しい対戦を始めました。");
  }
  private act(action: DuelAction): void {
    if (this.busy || this.historyView || !this.game.available().includes(action)) return;
    const beats = this.game.turn(action);
    this.pending = beats.map((beat) => ({ state: beat.after, text: beat.message }));
    this.busy = true; this.render(); this.nextBeat();
  }
  private nextBeat(): void {
    if (!this.active) return;
    const next = this.pending.shift();
    if (!next) { this.busy = false; this.render(); return; }
    this.show(next.state, next.text);
    this.timer = window.setTimeout(() => { this.timer = null; this.nextBeat(); }, 850);
  }
  private cancel(): void { if (this.timer !== null) clearTimeout(this.timer); this.timer = null; this.pending = []; this.busy = false; }
  private live(): void { this.cancel(); this.historyView = false; this.render(); this.show(this.game.state, "対戦の現在の局面です。"); }
  private replay(): void {
    if (this.busy || !this.game.history.length) return;
    this.cancel(); this.historyView = true; this.busy = true;
    this.pending = this.game.history.map((beat) => ({ state: beat.after, text: `振り返り：${beat.message}` }));
    this.render(); this.nextBeat();
  }
  private show(state: DuelState, text: string): void {
    this.scene.show(duelPair(state)); this.scene.setDisplay(this.bones, this.faded, true);
    this.caption.textContent = state.ended ? `対戦終了 · ${situation(state)}` : situation(state); this.message.textContent = text;
    this.hud.replaceChildren(...(["blue", "red"] as const).map((actor) => h("div", { class: `duel-fighter duel-${actor}` },
      h("strong", { text: actor === "blue" ? "青 · あなた" : "赤 · 相手" }), h("span", { text: `展開 ${state.points[actor]} · ${state.top === actor ? "極め" : "返し"}の準備 ${state.preparation[actor]} / 2` }),
      h("label", {}, `余力 ${state.energy[actor]}`, h("meter", { attrs: { min: "0", max: "100", value: String(state.energy[actor]), "aria-label": `${actor === "blue" ? "青" : "赤"}の余力` } })))),
      h("p", { text: `${state.turn} / 12手 · ${situation(state)} · 支配 ${state.control} / 3` }));
  }
  private render(): void {
    const s = this.game.state;
    this.panel.replaceChildren(h("h3", { text: this.historyView ? "攻防の振り返り" : s.ended ? s.winner ? `${s.winner === "blue" ? "あなた" : "相手"}の一本` : "12手終了" : this.busy ? "相手の反応を見る" : "あなたの行動" }));
    if (s.ended) this.panel.append(h("p", { text: s.winner ? "タップを確認して終了。動きの変遷から前の局面を見直せます。" : `展開を進めた回数：青 ${s.points.blue}、赤 ${s.points.red}。一本なしで区切りました。` }));
    else this.panel.append(h("p", { text: `相手の狙い：${DUEL_ACTIONS[this.game.opponentPlan].label}。上下が変わると相手も行動を切り替えます。` }));
    for (const [index, action] of this.game.available().entries()) {
      const definition = DUEL_ACTIONS[action];
      const button = h("button", { class: "duel-action", onClick: () => this.act(action) }, h("strong", { text: `${index + 1}. ${definition.label}` }), h("span", { text: `${definition.description}（余力 −${definition.cost}）` }));
      button.disabled = this.busy || this.historyView; this.panel.append(button);
    }
    if (!s.ended) this.panel.append(h("button", { class: "btn", text: "タップして対戦を止める", onClick: () => { this.cancel(); this.game.tap(); this.historyView = false; this.render(); this.show(this.game.state, "タップを確認して対戦を止めました。"); } }));
    this.log.replaceChildren(...this.game.history.map((beat, index) => h("li", {}, h("button", { class: "btn", text: `${index + 1}. ${beat.actor === "blue" ? "青" : "赤"}：${DUEL_ACTIONS[beat.action].label} → ${situation(beat.after)}`, onClick: () => {
      this.cancel(); this.historyView = true; this.render(); this.show(beat.after, `振り返り：${beat.message}`);
    } }))));
  }
}

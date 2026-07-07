// タブ1: 道場 — 連続ロールの状態機械 UI。
// setup(相手の初動は伏せる) → attack(判断) → feedback(採点) → 次局面 or レビュー。

import { DojoScene } from "../render/scene";
import { poseByName } from "../render/poses";
import { RollEngine, type Difficulty, type Focus, type Outcome } from "../engine/roll";
import { recordResult } from "../engine/srs";
import { loadProgress, saveProgress, type KeyValueStore, type ProgressData } from "../engine/storage";
import { scenarioById } from "../content/scenarios";
import { jointById } from "../anatomy/joints";
import type { JointKind } from "../anatomy/types";
import type { Stage, Uniform } from "../content/types";
import { chipRow, h } from "./dom";

type Phase = "idle" | "setup" | "attack" | "feedback" | "review";

interface TouchRec {
  positionJp: string;
  actionLabel: string;
  before: number;
  after: number;
}

export interface DojoDeps {
  switchToLab: () => void;
  onProgressChange: () => void;
}

const KIND_JP: Record<JointKind, string> = {
  hinge: "ヒンジ",
  "ball-socket": "球関節",
  pivot: "車軸",
  condyloid: "顆状",
};

const SETUP_MS = 1200;

export class DojoTab {
  readonly root: HTMLElement;
  private readonly scene: DojoScene;
  private readonly captionEl: HTMLElement;
  private readonly settingsEl: HTMLElement;
  private readonly panelEl: HTMLElement;
  private readonly store: KeyValueStore;
  private readonly deps: DojoDeps;

  private focus: Focus = "mixed";
  private uniform: Uniform = "gi";
  private difficulty: Difficulty = "beginner";

  private engine: RollEngine | null = null;
  private phase: Phase = "idle";
  private progress: ProgressData;
  private outcome: Outcome | null = null;
  private readonly touched = new Map<string, TouchRec>();
  private rollCounted = false;

  private setupTimer: number | null = null;
  private raf: number | null = null;
  private timerFill: HTMLElement | null = null;
  private pressureEl: HTMLElement | null = null;

  constructor(store: KeyValueStore, deps: DojoDeps) {
    this.store = store;
    this.deps = deps;
    this.progress = loadProgress(store);

    const canvas = h("canvas", { class: "dojo-canvas" });
    const canvasWrap = h("div", { class: "dojo-canvas-wrap" }, canvas);
    this.captionEl = h("div", { class: "dojo-caption" });
    canvasWrap.append(this.captionEl);

    this.panelEl = h("div", { class: "dojo-panel" });
    this.settingsEl = h("div", { class: "dojo-settings" });

    this.root = h(
      "div",
      { class: "dojo" },
      this.settingsEl,
      h("div", { class: "dojo-stage-wrap" }, canvasWrap, this.panelEl),
    );

    this.scene = new DojoScene({ canvas, pair: true });
    this.applyStage({ red: "standingRed", blue: "standingBlue", badge: "礼 — 設定を選んでロールを開始" });
    this.renderSettings();
    this.renderPanel();
  }

  refreshSize(): void {
    this.scene.refreshSize();
  }

  // --- 設定バー ---------------------------------------------------------------
  private get locked(): boolean {
    return this.phase !== "idle" && this.phase !== "review";
  }

  private renderSettings(): void {
    this.settingsEl.replaceChildren();
    this.settingsEl.append(
      this.segmented<Focus>("フォーカス", this.focus, [
        ["mixed", "混合"],
        ["defense", "防御"],
        ["offense", "攻撃"],
      ], (v) => (this.focus = v)),
      this.segmented<Uniform>("ギ/ノーギ", this.uniform, [
        ["gi", "ギ"],
        ["nogi", "ノーギ"],
      ], (v) => (this.uniform = v)),
      this.segmented<Difficulty>("難度", this.difficulty, [
        ["beginner", "入門"],
        ["live", "実戦"],
      ], (v) => (this.difficulty = v)),
    );
    if (!this.locked) {
      this.settingsEl.append(
        h("button", { class: "btn btn-start", text: "ロール開始", onClick: () => this.startRoll() }),
      );
    } else {
      this.settingsEl.append(h("span", { class: "roll-live-tag", text: "ロール中" }));
    }
  }

  private segmented<T extends string>(
    label: string,
    current: T,
    options: readonly (readonly [T, string])[],
    onSelect: (v: T) => void,
  ): HTMLElement {
    const group = h("div", { class: "seg" }, h("span", { class: "seg-label", text: label }));
    const btns = h("div", { class: "seg-btns" });
    for (const [value, text] of options) {
      const active = value === current;
      const btn = h("button", {
        class: `seg-btn${active ? " seg-btn-on" : ""}`,
        text,
        onClick: () => {
          if (this.locked) return;
          onSelect(value);
          this.renderSettings();
        },
      });
      if (this.locked) btn.setAttribute("disabled", "true");
      btns.append(btn);
    }
    group.append(btns);
    return group;
  }

  // --- ロール制御 -------------------------------------------------------------
  private startRoll(startId?: Parameters<RollEngine["start"]>[0]): void {
    this.cancelTimers();
    this.progress = loadProgress(this.store);
    this.engine = new RollEngine(
      { focus: this.focus, uniform: this.uniform, difficulty: this.difficulty },
      this.progress.srs,
      Date.now(),
    );
    this.touched.clear();
    this.rollCounted = false;
    this.outcome = null;
    this.engine.start(startId);
    this.enterSetup();
  }

  private enterSetup(): void {
    this.cancelTimers();
    this.phase = "setup";
    const step = this.engine?.step;
    if (!step) return;
    this.applyStage(step.scenario.setup);
    this.scene.blue.highlightJoint(null);
    this.scene.red?.highlightJoint(null);
    this.renderSettings();
    this.renderPanel();
    this.setupTimer = window.setTimeout(() => this.enterAttack(), SETUP_MS);
  }

  private enterAttack(): void {
    this.cancelTimers();
    this.phase = "attack";
    const step = this.engine?.step;
    if (!step) return;
    this.applyStage(step.action.attack);
    this.renderPanel();
    if (step.timeLimitSec !== null) this.startCountdown(step.timeLimitSec, step.action.pressure);
  }

  private selectOption(index: number): void {
    if (this.phase !== "attack" || !this.engine) return;
    const step = this.engine.step;
    if (!step || index >= step.options.length) return;
    this.cancelTimers();
    this.applyOutcome(this.engine.answer(index));
  }

  private timeOut(): void {
    if (this.phase !== "attack" || !this.engine) return;
    this.cancelTimers();
    this.applyOutcome(this.engine.timeout());
  }

  private applyOutcome(outcome: Outcome): void {
    this.phase = "feedback";
    this.outcome = outcome;
    const step = this.engine?.step;
    if (!step) return;
    this.applyStage(outcome.choice.result);

    const now = Date.now();
    const key = outcome.srsKey;
    const before = this.progress.srs[key]?.box ?? -1;
    this.progress = { ...this.progress, srs: recordResult(this.progress.srs, key, outcome.correct, now) };
    const after = this.progress.srs[key]?.box ?? before;
    const prior = this.touched.get(key);
    this.touched.set(key, {
      positionJp: step.scenario.positionJp,
      actionLabel: step.action.label,
      before: prior?.before ?? before,
      after,
    });
    saveProgress(this.store, this.progress);
    this.deps.onProgressChange();

    this.renderPanel();
  }

  private advance(): void {
    if (this.phase !== "feedback" || !this.engine) return;
    const next = this.engine.advance();
    if (next) this.enterSetup();
    else this.enterReview();
  }

  private enterReview(): void {
    this.cancelTimers();
    this.phase = "review";
    if (!this.rollCounted) {
      this.progress = { ...this.progress, rollsCompleted: this.progress.rollsCompleted + 1 };
      saveProgress(this.store, this.progress);
      this.rollCounted = true;
      this.deps.onProgressChange();
    }
    this.renderSettings();
    this.renderPanel();
  }

  // --- タイマー ---------------------------------------------------------------
  private startCountdown(limitSec: number, pressure: { early: string; urgent: string }): void {
    const total = limitSec * 1000;
    const start = performance.now();
    let earlyShown = false;
    let urgentShown = false;
    const tick = (t: number): void => {
      const remain = Math.max(0, total - (t - start));
      const frac = remain / total;
      if (this.timerFill) this.timerFill.style.width = `${frac * 100}%`;
      if (frac <= 0.4 && !earlyShown) {
        earlyShown = true;
        this.showPressure(pressure.early, false);
      }
      if (frac <= 0.2 && !urgentShown) {
        urgentShown = true;
        this.showPressure(pressure.urgent, true);
        this.timerFill?.classList.add("timer-fill-urgent");
      }
      if (remain <= 0) {
        this.raf = null;
        this.timeOut();
        return;
      }
      this.raf = requestAnimationFrame(tick);
    };
    this.raf = requestAnimationFrame(tick);
  }

  private showPressure(text: string, urgent: boolean): void {
    if (!this.pressureEl) return;
    this.pressureEl.textContent = text;
    this.pressureEl.className = `pressure${urgent ? " pressure-urgent" : ""}`;
  }

  private cancelTimers(): void {
    if (this.setupTimer !== null) {
      clearTimeout(this.setupTimer);
      this.setupTimer = null;
    }
    if (this.raf !== null) {
      cancelAnimationFrame(this.raf);
      this.raf = null;
    }
    this.timerFill = null;
    this.pressureEl = null;
  }

  private applyStage(stage: Stage): void {
    this.scene.red?.applyPose(poseByName(stage.red));
    this.scene.blue.applyPose(poseByName(stage.blue));
    this.captionEl.innerHTML = stage.badge;
  }

  // --- キーボード -------------------------------------------------------------
  handleKey(e: KeyboardEvent): void {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const el = document.activeElement;
    if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT")) return;
    if (this.phase === "attack" && e.key >= "1" && e.key <= "4") {
      e.preventDefault();
      this.selectOption(Number(e.key) - 1);
    } else if (this.phase === "feedback" && e.key === "Enter") {
      e.preventDefault();
      this.advance();
    }
  }

  // --- パネル描画 -------------------------------------------------------------
  private renderPanel(): void {
    this.panelEl.replaceChildren();
    switch (this.phase) {
      case "idle":
        this.renderIdle();
        break;
      case "setup":
        this.renderSetup();
        break;
      case "attack":
        this.renderAttack();
        break;
      case "feedback":
        this.renderFeedback();
        break;
      case "review":
        this.renderReview();
        break;
    }
  }

  private renderIdle(): void {
    this.panelEl.append(
      h("h2", { class: "panel-title", text: "稽古を始める" }),
      h("p", {
        class: "panel-lead",
        text: "フォーカス・ギ/ノーギ・難度を選び「ロール開始」。一本の連続ロールとして局面が動的に繋がります。",
      }),
      h("ul", { class: "how-list" },
        h("li", { text: "setup で状況を読み、相手の初動を待つ" }),
        h("li", { text: "初動が出たら 1〜4 で最善手を選ぶ (実戦は制限時間あり)" }),
        h("li", { text: "回答後 Enter で次へ。ロール終了後にレビューから再ロール" }),
      ),
    );
  }

  private renderSetup(): void {
    const step = this.engine?.step;
    if (!step) return;
    const s = step.scenario;
    this.panelEl.append(this.positionHeader());
    this.panelEl.append(h("div", { class: "badge-line", html: s.setup.badge }));
    this.panelEl.append(h("p", { class: "situation", text: s.situation }));
    this.panelEl.append(h("h4", { class: "cue-title", text: "基本の読む線" }));
    this.panelEl.append(chipRow(s.readCues, "base"));
    this.panelEl.append(h("div", { class: "waiting", text: "相手が動く…" }));
  }

  private renderAttack(): void {
    const step = this.engine?.step;
    if (!step) return;
    this.panelEl.append(this.positionHeader());
    this.panelEl.append(
      h("div", { class: "action-line" },
        h("span", { class: "action-tag", text: "相手の初動" }),
        h("span", { class: "action-label", text: step.action.label }),
      ),
    );
    this.panelEl.append(h("h4", { class: "cue-title", text: "この初動で読む線" }));
    this.panelEl.append(chipRow(step.action.readCues, "read"));

    if (step.timeLimitSec !== null) {
      this.pressureEl = h("div", { class: "pressure" });
      const timerTrack = h("div", { class: "timer-track" });
      this.timerFill = h("div", { class: "timer-fill" });
      timerTrack.append(this.timerFill);
      this.panelEl.append(timerTrack, this.pressureEl);
    }

    this.panelEl.append(h("p", { class: "prompt", text: step.scenario.prompt }));

    const list = h("div", { class: "options" });
    step.options.forEach((opt, i) => {
      const btn = h("button", { class: "option", onClick: () => this.selectOption(i) },
        h("span", { class: "option-num", text: String(i + 1) }),
        h("span", { class: "option-body" },
          h("span", { class: "option-jp", text: opt.jp }),
          h("span", { class: "option-en", text: opt.en }),
        ),
      );
      list.append(btn);
    });
    this.panelEl.append(list);
  }

  private renderFeedback(): void {
    const step = this.engine?.step;
    const o = this.outcome;
    if (!step || !o) return;
    const verdict = o.timedOut
      ? { text: "考えている間に極められた", cls: "verdict-timeout" }
      : o.correct
        ? { text: "良い判断", cls: "verdict-good" }
        : { text: "捕まった", cls: "verdict-bad" };

    this.panelEl.append(h("div", { class: `verdict ${verdict.cls}`, text: verdict.text }));
    this.panelEl.append(h("div", { class: "badge-line", html: o.choice.result.badge }));
    this.panelEl.append(h("p", { class: "feedback", html: o.choice.feedback }));

    const followUp = o.correct ? o.choice.reaction : o.choice.consequence;
    if (followUp) {
      this.panelEl.append(
        h("p", { class: `follow ${o.correct ? "follow-reaction" : "follow-consequence"}`, text: followUp }),
      );
    }

    if (o.readCues.length > 0) {
      this.panelEl.append(h("h4", { class: "cue-title", text: "読めた線" }));
      this.panelEl.append(chipRow(o.readCues, "read"));
    }
    if (o.missedCues.length > 0) {
      this.panelEl.append(h("h4", { class: "cue-title", text: "見落とした線" }));
      this.panelEl.append(chipRow(o.missedCues, "missed"));
    }

    this.panelEl.append(
      h("p", { class: "action-cue" },
        h("strong", { text: "初動の読み: " }),
        document.createTextNode(step.action.cue),
      ),
    );
    this.panelEl.append(h("p", { class: "principle", html: step.scenario.principle }));

    if (step.scenario.focusJoints.length > 0) {
      const danger = h("div", { class: "danger-struct" });
      danger.append(h("h4", { class: "cue-title", text: "この局面の危険構造" }));
      const names = step.scenario.focusJoints
        .map((id) => {
          const j = jointById(id);
          return `${j.jp}（${KIND_JP[j.kind]}）`;
        })
        .join(" ・ ");
      danger.append(h("p", { class: "danger-names", text: names }));
      danger.append(
        h("button", { class: "btn btn-lab", text: "関節ラボで構造を見る", onClick: () => this.deps.switchToLab() }),
      );
      this.panelEl.append(danger);
    }

    this.panelEl.append(
      h("button", { class: "btn btn-next", text: "次へ (Enter)", onClick: () => this.advance() }),
    );
  }

  private renderReview(): void {
    if (!this.engine) return;
    const history = this.engine.history;
    const correct = history.filter((r) => r.correct).length;

    this.panelEl.append(h("h2", { class: "panel-title", text: "ロール終了 — 振り返り" }));
    this.panelEl.append(
      h("p", { class: "review-score" },
        h("strong", { text: `${correct} / ${history.length}` }),
        document.createTextNode(" 局面で最善手"),
      ),
    );

    const changes = [...this.touched.values()].filter((t) => t.after !== t.before);
    if (changes.length > 0) {
      const up = changes.filter((c) => c.after > c.before);
      const down = changes.filter((c) => c.after < c.before);
      const box = h("div", { class: "review-changes" });
      if (up.length > 0) {
        box.append(h("h4", { class: "cue-title", text: "習熟が上がった" }));
        for (const c of up) box.append(h("div", { class: "change change-up", text: `${c.positionJp} / ${c.actionLabel}` }));
      }
      if (down.length > 0) {
        box.append(h("h4", { class: "cue-title", text: "習熟が下がった" }));
        for (const c of down) box.append(h("div", { class: "change change-down", text: `${c.positionJp} / ${c.actionLabel}` }));
      }
      this.panelEl.append(box);
    }

    const log = h("ol", { class: "review-log" });
    for (const r of history) {
      const nextName = r.nextId ? scenarioById(r.nextId).positionJp : "ロール終了";
      const mark = r.timedOut ? "時間切れ" : r.correct ? "○" : "×";
      const follow = r.correct ? r.reaction : r.consequence;
      const item = h("li", { class: `log-item ${r.correct ? "log-ok" : "log-ng"}` },
        h("div", { class: "log-head" },
          h("span", { class: "log-mark", text: mark }),
          h("span", { class: "log-pos", text: r.positionJp }),
        ),
        h("div", { class: "log-line", text: `相手: ${r.actionLabel}` }),
        h("div", { class: "log-line", text: `選択: ${r.chosenJp}` }),
      );
      if (follow) item.append(h("div", { class: "log-follow", text: follow }));
      item.append(h("div", { class: "log-next", text: `▸ ${nextName}` }));
      log.append(item);
    }
    this.panelEl.append(log);

    const actions = h("div", { class: "review-actions" });
    actions.append(h("button", { class: "btn btn-start", text: "もう一度ロール", onClick: () => this.startRoll() }));
    const firstMiss = history.find((r) => !r.correct);
    if (firstMiss) {
      actions.append(
        h("button", { class: "btn btn-secondary", text: "苦手局面から再ロール", onClick: () => this.startRoll(firstMiss.scenarioId) }),
      );
    }
    this.panelEl.append(actions);
  }

  private positionHeader(): HTMLElement {
    const s = this.engine?.step?.scenario;
    if (!s) return h("div");
    return h("div", { class: "position-head" },
      h("span", { class: "belt-tag", text: s.belt }),
      h("div", { class: "position-names" },
        h("span", { class: "position-jp", text: s.positionJp }),
        h("span", { class: "position-en", text: `${s.positionEn} · ${s.term}` }),
      ),
    );
  }
}

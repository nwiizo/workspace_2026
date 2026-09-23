import { PRACTICE_LESSONS, practiceLesson, type PracticeLesson } from "../content/practice";
import { checkedPracticeCount, nextPractice, practiceKey, PracticeRun, type PracticeMode } from "../engine/practice";
import { recordResult } from "../engine/srs";
import { loadProgress, saveProgress, type KeyValueStore } from "../engine/storage";
import { BodyScene } from "../render/bodyScene";
import { practicePair } from "../render/practicePose";
import { h } from "./dom";
import type { BodyExperiment } from "../labs/bodyLab";

export class PracticeTab {
  readonly root: HTMLElement;
  private readonly menu = h("nav", { class: "practice-menu", attrs: { "aria-label": "基礎課題" } });
  private readonly heading = h("div", { class: "practice-heading" });
  private readonly panel = h("section", { class: "practice-panel", attrs: { "aria-label": "操作と振り返り" } });
  private readonly facts = h("ul", { class: "practice-facts", attrs: { "aria-label": "今の状態" } });
  private readonly path = h("ol", { class: "practice-path", attrs: { "aria-label": "課題の進み具合" } });
  private readonly caption = h("p", { class: "practice-caption" });
  private readonly live = h("div", { class: "sr-only", attrs: { role: "status", "aria-live": "polite" } });
  private readonly scene: BodyScene | null;
  private readonly cues = h("div", { class: "practice-cues" });
  private readonly comparison = h("div", { class: "practice-comparison", attrs: { role: "group", "aria-label": "一手の前後を比較" } });
  private bones = false;
  private faded = false;
  private markers = true;
  private showBefore = false;
  private selected = PRACTICE_LESSONS[0]!.id;
  private run: PracticeRun | null = null;
  private variant = 0;
  private showHint = false;

  constructor(private readonly store: KeyValueStore, private readonly changed: () => void, private readonly openBody: (experiment: BodyExperiment) => void, private readonly openCourse: (id: string) => void) {
    const canvas = h("canvas", { class: "practice-canvas", attrs: { "aria-label": "赤が相手、青が自分。ドラッグで視点を回転。ボタンでも視点を変更できます。" } });
    const cameras = h("div", { class: "practice-cameras", attrs: { role: "group", "aria-label": "3Dの視点" } });
    for (const [label, view] of [
      ["斜め", "diagonal"], ["接地を横から", "side"], ["真上", "top"],
    ] as const) {
      cameras.append(h("button", { class: "btn", text: label, onClick: () => {
        this.scene?.setCamera(view);
      } }));
    }
    const pins = h("div", { class: "practice-pins", attrs: { "aria-hidden": "true" } });
    const display = h("div", { class: "practice-display", attrs: { role: "group", "aria-label": "人形の表示" } });
    for (const [label, bones] of [["身体", false], ["骨格", true]] as const) {
      display.append(h("button", { class: "btn", text: label, attrs: { "aria-pressed": String(!bones), "data-layer": String(bones) }, onClick: () => {
        this.bones = bones;
        display.querySelectorAll<HTMLButtonElement>("[data-layer]").forEach((button) => button.setAttribute("aria-pressed", String(button.dataset.layer === String(bones))));
        this.scene?.setDisplay(this.bones, this.faded, this.markers);
      } }));
    }
    for (const [label, key, checked] of [["相手を透かす", "faded", false], ["接点を表示", "markers", true]] as const) {
      const input = h("input", { attrs: { type: "checkbox", ...(checked ? { checked: "" } : {}) } });
      input.addEventListener("change", () => {
        this[key] = input.checked;
        this.cues.hidden = !this.markers;
        this.scene?.setDisplay(this.bones, this.faded, this.markers);
      });
      display.append(h("label", {}, input, document.createTextNode(label)));
    }
    const view = h("div", { class: "practice-view" }, canvas,
      h("div", { class: "practice-legend" }, h("span", { class: "you", text: "● 青＝あなた" }), h("span", { class: "opponent", text: "● 赤＝相手" })),
      pins, cameras, this.caption,
    );
    this.root = h("div", { class: "practice" },
      h("aside", { class: "practice-sidebar" },
        h("p", { class: "eyebrow", text: `POSITION LAB / ${PRACTICE_LESSONS.length}課題` }),
        h("h2", { text: "一手ずつ、柔術。" }),
        h("p", { class: "practice-intro", text: "相手を見る。自分を動かす。変化を確かめる。" }), this.menu,
        h("h3", { text: "ポジションの次の攻防" }),
        h("button", { class: "btn", text: "ポジションから極め技への例を見る", onClick: () => this.openCourse("position-armbar") }),
        h("button", { class: "btn", text: "トップの取り合いを見る", onClick: () => this.openCourse("top-exchange") }),
        h("p", { class: "practice-safety", text: "実技は指導者と。タップはいつでも使えます。痛みや苦しさを待つ必要はありません。" }),
      ),
      h("div", { class: "practice-main" }, this.heading,
        h("div", { class: "practice-board" },
          h("div", { class: "practice-observation" }, display, view, this.cues, this.comparison, this.facts, this.path,
            h("p", { class: "practice-model-note", text: "骨格と身体の厚みを重ねた模式図。丸印は床につく場所です。一手の前後を切り替えて観察できます。接触の力や筋肉の働きは計算していません。" })), this.panel),
      ), this.live,
    );
    try {
      this.scene = new BodyScene(canvas, pins);
    } catch {
      this.scene = null;
      canvas.remove();
      view.prepend(h("p", { class: "practice-fallback", text: "この環境では3Dを表示できません。「今の状態」を読んで練習を続けられます。" }));
      cameras.remove();
      display.remove();
    }
    this.render();
  }

  setActive(active: boolean): void {
    this.scene?.setActive(active);
    if (active) { this.renderMenu(); requestAnimationFrame(() => this.scene?.refreshSize()); }
  }

  private get lesson(): PracticeLesson { return practiceLesson(this.selected); }

  openLesson(id: string): void {
    this.selected = practiceLesson(id).id;
    this.run = null;
    this.render();
  }

  private start(mode: PracticeMode, sameVariant = false): void {
    if (!sameVariant) {
      const attempts = loadProgress(this.store).srs[practiceKey(this.selected, "check")]?.attempts ?? 0;
      this.variant = mode === "learn" ? 0 : (attempts + 1) % 2;
    }
    this.run = new PracticeRun(this.selected, mode, this.variant);
    this.showHint = mode === "learn";
    this.render();
    this.focusPanel();
  }

  private act(id: string): void {
    if (!this.run) return;
    this.run.act(id);
    this.render();
    this.focusPanel();
  }

  private advance(): void {
    if (!this.run) return;
    this.run.continue();
    const result = this.run.takeResult();
    if (result) {
      const progress = loadProgress(this.store);
      saveProgress(this.store, { ...progress, srs: recordResult(progress.srs, result.key, result.correct, Date.now()) });
      this.changed();
    }
    this.showHint = this.run.mode === "learn";
    this.render();
    this.focusPanel();
  }

  handleKey(e: KeyboardEvent): void {
    if (e.repeat || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (target?.closest("input, textarea, select, [contenteditable=true]")) return;
    if (e.key.toLowerCase() === "t" && this.run && ["acting", "feedback"].includes(this.run.phase)) {
      e.preventDefault(); this.act("tap");
    } else if (this.run?.phase === "acting" && /^[1-4]$/.test(e.key)) {
      e.preventDefault(); this.act(this.lesson.actions[Number(e.key) - 1]!.id);
    } else if (this.run?.phase === "feedback" && e.key === "Enter" && target?.tagName !== "BUTTON") {
      e.preventDefault(); this.advance();
    }
  }

  private focusPanel(): void { this.panel.querySelector<HTMLElement>("h2")?.focus({ preventScroll: true }); }

  private renderMenu(): void {
    const srs = loadProgress(this.store).srs;
    this.menu.replaceChildren();
    for (const [i, lesson] of PRACTICE_LESSONS.entries()) {
      const check = srs[practiceKey(lesson.id, "check")];
      const status = check?.box > 0 ? "確認済み" : check ? "もう一度確認" : srs[practiceKey(lesson.id, "learn")] ? "次は確認" : "未練習";
      this.menu.append(h("button", {
        class: `lesson-card${lesson.id === this.selected ? " lesson-selected" : ""}`,
        attrs: { "aria-current": lesson.id === this.selected ? "step" : "false" },
        onClick: () => { this.selected = lesson.id; this.run = null; this.render(); },
      }, h("span", { class: "lesson-number", text: String(i + 1).padStart(2, "0") }),
      h("span", {}, h("strong", { text: lesson.title }), h("small", { text: `${lesson.position} · ${status}` }))));
    }
  }

  private render(): void {
    const lesson = this.lesson;
    const run = this.run;
    const node = run?.node ?? lesson.nodes[lesson.starts[0]]!;
    this.renderMenu();
    this.heading.replaceChildren(
      h("div", {}, h("p", { class: "eyebrow", text: run ? (run.mode === "learn" ? "LEARN / ヒントを見ながら" : "CHECK / 相手の反応を読んで") : "POSITION PRACTICE / ポジション練習" }), h("h2", { text: lesson.title })),
      h("span", { class: "practice-tempo", text: "時間制限なし" }),
    );
    const isStopped = run?.phase === "stopped";
    this.showBefore = false;
    this.renderObservation();
    this.path.replaceChildren(...lesson.steps.map((step, i) => h("li", {
      class: i < node.progress ? "path-done" : i === node.progress ? "path-current" : "",
      text: `${i < node.progress ? "✓" : i + 1} ${step}`,
    })));
    this.panel.replaceChildren();
    if (!run) this.renderIntro();
    else if (isStopped) this.renderStopped();
    else if (run.phase === "complete") this.renderComplete();
    else if (run.phase === "feedback") this.renderFeedback();
    else this.renderActions();
    if (!isStopped) this.panel.append(this.button(run?.mode === "check" && run.phase === "acting" ? "体のしくみを試す（ヒント扱い）" : "この場面の体のしくみを試す", () => {
      const experiment: BodyExperiment = node.focus === "knee" || node.focus === "hip" && this.selected === "side-space" ? "leg" : this.selected === "mount-base" || this.selected === "guard-posture" ? "base" : "lever";
      if (run?.mode === "check" && run.phase === "acting") run.hint();
      this.openBody(experiment);
    }));
    if (lesson.source) {
      const source = lesson.source;
      this.panel.append(h("details", { class: "practice-source" }, h("summary", { text: "この練習の英語資料" }),
        h("a", { text: source.title, attrs: { href: source.url, target: "_blank", rel: "noopener noreferrer" } }),
        h("p", { text: `${source.section} · 確認 ${source.checked}` }), h("p", { text: source.scope })));
    }
  }

  private renderObservation(): void {
    const run = this.run;
    const stopped = run?.phase === "stopped";
    const turn = run?.phase === "feedback" ? run.history.at(-1) : undefined;
    const nodeId = this.showBefore && turn ? turn.before : run?.nodeId ?? this.lesson.starts[0];
    const node = this.lesson.nodes[nodeId]!;
    const pair = practicePair(this.selected, stopped ? "stopped" : nodeId);
    this.scene?.show(pair);
    this.caption.textContent = stopped ? "タップを確認し、相手が離した" : `${turn ? this.showBefore ? "一手の前 ｜ " : "一手の後 ｜ " : ""}${node.title}`;
    this.facts.replaceChildren(...(stopped ? ["練習：中断", "タップ：いつでも使える", "減点：なし"] : node.facts).map((fact) => h("li", { text: fact })));
    this.cues.replaceChildren(h("span", { text: "○ 床につく場所" }), ...pair.cues.map((cue, i) => h("span", { text: `${i + 1} ${cue.label}` })));
    this.comparison.replaceChildren();
    if (turn && turn.before !== turn.after) {
      for (const [label, before] of [["一手の前", true], ["一手の後", false]] as const) this.comparison.append(h("button", {
        class: "btn", text: label, attrs: { "aria-pressed": String(this.showBefore === before) },
        onClick: () => {
          this.showBefore = before;
          this.renderObservation();
          this.comparison.querySelector<HTMLButtonElement>('[aria-pressed="true"]')?.focus({ preventScroll: true });
        },
      }));
    }
  }

  private title(text: string): HTMLElement { return h("h2", { text, attrs: { tabindex: "-1" } }); }
  private button(text: string, onClick: () => void, primary = false): HTMLButtonElement {
    return h("button", { class: `btn${primary ? " btn-start" : ""}`, text, onClick });
  }

  private renderIntro(): void {
    this.panel.append(h("p", { class: "eyebrow", text: "今回の目標" }), this.title(this.lesson.goal),
      h("p", { text: "操作はこの課題の間ずっと同じ。今の状態に合う一手を選ぶと、相手が反応します。" }),
      h("ol", { class: "practice-how" }, h("li", { text: "青が自分。相手との位置と支えを見る。" }), h("li", { text: "ボタンか数字キー1〜4で一手を選ぶ。" }), h("li", { text: "変化と理由を読んで、次の一手へ。" })),
      this.button("ヒントを見ながら練習", () => this.start("learn"), true),
      this.button("相手を変えて確認", () => this.start("check")),
      h("p", { class: "practice-fine", text: "確認練習では相手の反応が変わります。ヒントなしで目標に届くと「確認済み」。実技の習得や帯を判定するものではありません。" }),
    );
  }

  private renderActions(): void {
    const run = this.run!;
    this.panel.append(h("p", { class: "eyebrow", text: "OBSERVE → MOVE / 次の一手" }), this.title(run.node.title), h("p", { text: run.node.situation }));
    if (this.showHint) this.panel.append(h("p", { class: "practice-hint", text: `見るところ：${run.node.hint}` }));
    const actions = h("div", { class: "practice-actions" });
    this.lesson.actions.forEach((action, index) => actions.append(h("button", { class: "practice-action", attrs: { "data-action": action.id }, onClick: () => this.act(action.id) },
      h("span", { class: "action-key", text: String(index + 1) }), h("span", {}, h("strong", { text: action.label }), h("small", { text: action.description })))));
    this.panel.append(actions,
      h("div", { class: "practice-tools" }, this.button("タップして止める [T]", () => this.act("tap")),
        ...(!this.showHint ? [this.button("ヒントを見る", () => { run.hint(); this.showHint = true; this.render(); this.live.textContent = run.node.hint; })] : [])),
    );
  }

  private renderFeedback(): void {
    const run = this.run!;
    const turn = run.history[run.history.length - 1]!;
    const before = this.lesson.nodes[turn.before]!;
    const after = run.node;
    this.panel.append(h("p", { class: "eyebrow", text: "MOVE → NOTICE / 一手の結果" }), this.title(turn.progressed ? "局面が変わった" : "その一手では進めなかった"),
      h("p", { class: "practice-chosen", text: `選んだ一手：${turn.action}` }),
      h("p", { class: turn.progressed ? "practice-feedback good" : "practice-feedback retry", text: turn.feedback }),
      h("div", { class: "practice-change" }, h("small", { text: "前" }), h("span", { text: before.title }), h("small", { text: "後" }), h("strong", { text: after.title })),
      this.button(after.end ? "振り返る [Enter]" : turn.progressed ? "変化を見て、次の一手へ [Enter]" : "同じ場面でもう一手 [Enter]", () => this.advance(), true),
    );
    this.live.textContent = turn.feedback;
  }

  private renderStopped(): void {
    this.panel.append(this.title("タップで練習を止めました"), h("p", { text: "相手が離れました。中断に減点はありません。落ち着いて状況を見直し、また始められます。" }),
      this.button("同じ場面からやり直す", () => this.start(this.run!.mode, true), true),
      this.button("課題の説明に戻る", () => { this.run = null; this.render(); }));
    this.live.textContent = "タップで練習を中断。相手が離れました。";
  }

  private renderComplete(): void {
    const run = this.run!;
    const passed = run.mistakes === 0 && !run.hintUsed;
    const title = run.mode === "learn" ? "流れを体験できました" : passed ? "相手を見て判断できました" : "もう一度、相手を見てみよう";
    this.panel.append(h("p", { class: "eyebrow", text: "REFLECT / 今回の振り返り" }), this.title(title),
      h("p", { class: "practice-feedback good", text: run.node.title }),
      h("p", { text: this.lesson.principle }),
      h("p", { class: "practice-fine", text: run.mode === "learn" ? "次は相手の反応を変えて、ヒントなしで確認できます。" : `やり直した一手：${run.mistakes}回 / ヒント：${run.hintUsed ? "使用" : "なし"}。${passed ? "確認結果を記録しました。" : "復習する課題として記録しました。"}` }),
    );
    const log = h("ol", { class: "practice-log" });
    for (const turn of run.history) log.append(h("li", {}, h("strong", { text: `${turn.progressed ? "✓" : "↻"} ${turn.action}` }), h("small", { text: turn.feedback })));
    this.panel.append(log, h("p", { class: "practice-transfer", text: this.lesson.transfer }));
    if (run.mode === "learn") this.panel.append(this.button("相手を変えて確認する", () => this.start("check"), true));
    else if (!passed) this.panel.append(this.button("同じ反応でもう一度", () => this.start("check", true), true));
    else {
      const srs = loadProgress(this.store).srs;
      const next = nextPractice(srs, Date.now());
      const allDone = checkedPracticeCount(srs) === PRACTICE_LESSONS.length;
      if (allDone) this.panel.append(h("p", { class: "practice-hint", text: `${PRACTICE_LESSONS.length}課題を確認できました。少し時間をあけて、相手の反応をもう一度読んでみよう。` }));
      this.panel.append(this.button(allDone ? "復習の課題へ" : `次へ：${practiceLesson(next.lessonId).title}`, () => { this.selected = next.lessonId; this.start(next.mode); }, true));
    }
    this.panel.append(this.button("課題一覧へ戻る", () => { this.run = null; this.render(); }));
    this.live.textContent = title;
  }
}

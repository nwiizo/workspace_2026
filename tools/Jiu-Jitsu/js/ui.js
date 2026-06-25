// ui.js
// DOM レンダリング層。ゲーム状態を受け取りパネルを描画する。ロジックは持たない。

import { DOJO_NOTES } from "./techniques.js";

export class UI {
  constructor() {
    this.lesson = document.getElementById("lesson");
    this.badge = document.getElementById("position-badge");
    this.scoreEl = document.getElementById("score");
    this.flowEl = document.getElementById("flow");
    this.beltEl = document.getElementById("belt");
    this.streakEl = document.getElementById("streak");
    this.giBtn = document.getElementById("uniform-gi");
    this.nogiBtn = document.getElementById("uniform-nogi");
    this.mixBtn = document.getElementById("mode-mixed");
    this.defBtn = document.getElementById("mode-defense");
    this.offBtn = document.getElementById("mode-offense");
    this.beginnerBtn = document.getElementById("difficulty-beginner");
    this.liveBtn = document.getElementById("difficulty-live");
    this.styleRandomBtn = document.getElementById("style-random");
    this.stylePressureBtn = document.getElementById("style-pressure-passer");
    this.styleChokeBtn = document.getElementById("style-choke-hunter");
    this.styleGuardBtn = document.getElementById("style-guard-player");
    this._setupDojoNotes();
  }

  renderBadge(html) {
    this.badge.innerHTML = safeInlineHtml(html);
  }

  renderScore(score, belt, streak = 0, flow = 0) {
    this.scoreEl.textContent = `道場ポイント: ${score}`;
    this.beltEl.textContent = `${belt.icon} ${belt.name}`;
    if (this.flowEl) {
      const state = flowState(flow);
      this.flowEl.textContent = `流れ: ${state.label}`;
      this.flowEl.className = `flow flow--${state.tone}`;
    }
    if (this.streakEl) {
      if (streak >= 2) {
        this.streakEl.textContent = `🔥 ${streak} 連続`;
        this.streakEl.hidden = false;
      } else {
        this.streakEl.hidden = true;
      }
    }
  }

  renderLesson(scenario, onChoose, meta = {}) {
    const s = scenario;
    const offense = s.role === "offense";
    const roleTag = offense
      ? `<span class="tag tag--offense">攻撃ロール</span><span class="tag tag--player">自分: 赤</span>`
      : `<span class="tag tag--defense">防御ロール</span><span class="tag tag--player">自分: 青</span>`;
    const progress = meta.total
      ? `<span class="tag tag--prog">局面 ${meta.index + 1} / ${meta.total}</span>`
      : "";
    const uniform = meta.uniformMode === "nogi" ? "ノーギ" : "ギ";
    const difficulty = meta.difficultyMode === "live" ? "実戦" : "入門";
    const focus = meta.rollMode === "offense"
      ? "攻撃フォーカス"
      : meta.rollMode === "defense"
        ? "防御フォーカス"
        : "混合ロール";
    const opponentStyle = meta.opponentStyle
      ? `<span class="tag tag--focus">相手: ${escapeHtml(meta.opponentStyle.label)}</span>`
      : "";
    const opponentAction = s.opponentAction?.label
      ? `<span class="tag tag--focus">初動: ${escapeHtml(s.opponentAction.label)}</span>`
      : "";
    const rollState = Array.isArray(meta.rollState) && meta.rollState.length
      ? `<span class="tag tag--focus">引き継ぎ: ${meta.rollState.map(escapeHtml).join(" / ")}</span>`
      : "";
    const decisionOpen = meta.decisionOpen !== false;
    const mission = meta.mission
      ? `<div class="mission"><span>今回の狙い</span><b>${escapeHtml(meta.mission.label)}</b>${escapeHtml(meta.mission.text)}</div>`
      : "";
    const tactic = meta.tactic
      ? `<div class="mission mission--tactic"><span>今回の制約</span><b>${escapeHtml(meta.tactic.label)}</b>${escapeHtml(meta.tactic.text)}</div>`
      : "";
    const adaptiveFocus = meta.adaptiveFocus
      ? `<div class="mission mission--adaptive"><span>今回の補正</span><b>${escapeHtml(meta.adaptiveFocus.label)}</b>${escapeHtml(meta.adaptiveFocus.text)}</div>`
      : "";
    const flowNote = offense
      ? "一本のロールとして、良い位置を作ってから攻めを継続します。"
      : "一本のロールとして、守る・逃げる・次の局面へ進む判断を続けます。";

    const opts = s.options
      .map(
        (o, i) => `
        <button class="option ${decisionOpen ? "" : "option--locked"}" data-i="${i}" type="button" aria-keyshortcuts="${i + 1}" aria-label="${i + 1}: ${escapeHtml(o.jp)}"${decisionOpen ? "" : " disabled"}>
          <span class="opt-key">${i + 1}</span>
          <span class="opt-jp">${escapeHtml(o.jp)}</span>
          <span class="opt-en">${escapeHtml(o.en)}</span>
        </button>`,
      )
      .join("");
    const readCues = Array.isArray(s.readCues) && s.readCues.length
      ? `<div class="read-cues"><span>読む線</span>${s.readCues
          .map((cue) => `<b>${escapeHtml(cue)}</b>`)
          .join("")}</div>`
      : "";

    this.lesson.innerHTML = `
      <div class="tag-row">
        <span class="tag tag--belt">${escapeHtml(s.belt)}</span>
        ${roleTag}
        ${progress}
        <span class="tag tag--focus">${focus}</span>
        ${opponentStyle}
        ${opponentAction}
        ${rollState}
        <span class="tag">${uniform}</span>
        <span class="tag">${difficulty}</span>
      </div>
      <h2>${escapeHtml(s.positionJp)}</h2>
      <p class="romaji">${escapeHtml(s.positionEn)} ・ ${escapeHtml(s.term)}</p>
      <p class="flow-note">${escapeHtml(flowNote)}</p>
      ${mission}
      ${tactic}
      ${adaptiveFocus}
      <div class="roll-meter">
        <span id="decision-clock">${decisionOpen ? "判断待ち" : "構え"}</span>
        <span id="opponent-intent">${decisionOpen ? "相手の動きを読む" : "相手の初動を見る"}</span>
      </div>
      ${readCues}
      <div class="situation ${offense ? "situation--offense" : ""}">${escapeHtml(s.situation)}</div>
      <p class="prompt ${offense ? "prompt--offense" : ""}">${escapeHtml(s.prompt)}</p>
      <div class="options">${opts}</div>
    `;

    this.lesson.querySelectorAll(".option").forEach((btn) => {
      btn.addEventListener("click", () => onChoose(Number(btn.dataset.i)));
    });
    this._bindNumberKeys(onChoose);
    this._unbindFeedbackKeys();
    this.lesson.scrollTop = 0;
  }

  renderFeedback(scenario, chosenIndex, opts) {
    const { isLast, autoAdvance, advanceMs, timedOut, tempo, onNext, onReplay } = opts;
    const s = scenario;
    const chosen = s.options[chosenIndex];
    const good = chosen.correct;

    this.lesson.querySelectorAll(".option").forEach((btn) => {
      const i = Number(btn.dataset.i);
      btn.disabled = true;
      const o = s.options[i];
      if (o.correct) {
        btn.classList.add("correct");
        btn.insertAdjacentHTML("afterbegin", `<span class="verdict">✓</span>`);
      } else if (i === chosenIndex) {
        btn.classList.add("wrong");
        btn.insertAdjacentHTML("afterbegin", `<span class="verdict">✕</span>`);
      }
    });

    const nextLabel = isLast ? "結果を見る ▸" : "次の局面へ ▸";
    const advanceNote =
      autoAdvance
        ? `<p class="auto-note">流れは止まらない — まもなく次の局面へ自動で進みます</p>`
        : "";
    const continuation = good ? chosen.reaction : chosen.consequence;
    const continuationLabel = good ? "相手の反応" : timedOut ? "時間切れの展開" : "相手の追撃";
    const continuationHtml = continuation
      ? `<div class="reaction">${continuationLabel}: ${escapeHtml(continuation)}</div>`
      : "";
    const readLineHtml = Array.isArray(s.readCues) && s.readCues.length
      ? `<div class="read-line ${good ? "read-line--good" : "read-line--bad"}">
          <span>${good ? "読めた線" : timedOut ? "遅れた線" : "見落とした線"}</span>
          ${s.readCues.map((cue) => `<b>${escapeHtml(cue)}</b>`).join("")}
        </div>`
      : "";
    const actionCueHtml = s.opponentAction?.cue
      ? `<div class="reaction reaction--cue">初動の読み: ${escapeHtml(s.opponentAction.cue)}</div>`
      : "";
    const tempoHtml = tempo?.id && tempo.id !== "none"
      ? `<div class="tempo-badge ${tempo.bonus ? "tempo-badge--bonus" : ""}">
          <span>${escapeHtml(tempo.label)}</span>
          ${escapeHtml(tempo.text)}${tempo.bonus ? ` ・ +${tempo.bonus} pt` : ""}
        </div>`
      : "";
    const stateHtml = opts.stateEffects?.added?.length || opts.stateEffects?.removed?.length
      ? `<div class="reaction reaction--cue">引き継ぎ状態: ${
          [
            ...(opts.stateEffects.added || []).map((label) => `+${escapeHtml(label)}`),
            ...(opts.stateEffects.removed || []).map((label) => `-${escapeHtml(label)}`),
          ].join(" / ")
        }</div>`
      : "";

    const fb = document.createElement("div");
    fb.className = `feedback ${good ? "feedback--good" : "feedback--bad"}`;
    fb.innerHTML = `
      <h3>${good ? "✓ 良い柔術です" : timedOut ? "✕ 相手に先手を取られました" : "✕ 危険でした"}</h3>
      <div>${escapeHtml(chosen.feedback)}</div>
      ${readLineHtml}
      ${actionCueHtml}
      ${tempoHtml}
      ${stateHtml}
      ${continuationHtml}
      <div class="principle">原則: ${safeInlineHtml(s.principle)}</div>
      ${advanceNote}
      <button class="next-btn" type="button">${nextLabel}</button>
      <button class="next-btn next-btn--replay" type="button">この局面をもう一度</button>
    `;
    this.lesson.appendChild(fb);

    const [nextBtn, replayBtn] = fb.querySelectorAll(".next-btn");
    nextBtn.addEventListener("click", onNext);
    replayBtn.addEventListener("click", onReplay);
    this._bindFeedbackKeys(onNext, onReplay);

    if (autoAdvance && advanceMs) {
      // プログレスバーで自動前進を可視化
      const bar = document.createElement("div");
      bar.className = "advance-bar";
      bar.innerHTML = `<span style="animation-duration:${advanceMs}ms"></span>`;
      fb.insertBefore(bar, nextBtn);
    }
    fb.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }

  renderPressure(secondsLeft, text) {
    const clock = document.getElementById("decision-clock");
    const intent = document.getElementById("opponent-intent");
    if (clock) {
      clock.textContent = Number.isFinite(secondsLeft)
        ? secondsLeft > 0 ? `判断 ${secondsLeft}s` : "相手が動いた"
        : "時間制限なし";
    }
    if (intent) intent.textContent = text;
  }

  renderComplete(
    score,
    correct,
    total,
    belt,
    rollMode,
    history = [],
    flow = 0,
    missionResult,
    tactic,
    coachReview,
    onRestart,
    weaknessFocus,
  ) {
    this._unbindFeedbackKeys();
    const modeText = rollMode === "offense" ? "攻め" : rollMode === "defense" ? "守り" : "攻守ミックス";
    const finalFlow = flowState(flow);
    const review = history
      .filter(Boolean)
      .map((item, i) => {
        const result = item.correct ? "正解" : item.timedOut ? "時間切れ" : "不正解";
        const resultClass = item.correct ? "review__result--good" : "review__result--bad";
        const nextLabel = item.nextRole === "offense" ? "攻めへ" : item.nextRole === "defense" ? "守りへ" : "次へ";
        const nextLine = item.nextPositionJp
          ? `<div class="review__choice">次: ${escapeHtml(nextLabel)} ${escapeHtml(item.nextPositionJp)}${item.nextSelectedByGraph ? "（反応から分岐）" : ""}</div>`
          : "";
        const actionLine = item.opponentAction?.label
          ? `<div class="review__choice">相手初動: ${escapeHtml(item.opponentAction.label)}</div>`
          : "";
        const actionCueLine = item.opponentAction?.cue
          ? `<div class="review__choice">初動の読み: ${escapeHtml(item.opponentAction.cue)}</div>`
          : "";
        const nextReason = item.nextReason
          ? `<div class="review__choice">理由: ${escapeHtml(item.nextReason)}</div>`
          : "";
        const tempoLine = item.tempo?.id && item.tempo.id !== "none"
          ? `<div class="review__choice">テンポ: ${escapeHtml(item.tempo.label)}${item.tempo.bonus ? ` (+${item.tempo.bonus} pt)` : ""}</div>`
          : "";
        const stateLine = item.rollState?.length
          ? `<div class="review__choice">引き継ぎ状態: ${item.rollState.map(escapeHtml).join(" / ")}</div>`
          : "";
        const readLine = Array.isArray(item.readCues) && item.readCues.length
          ? `<div class="review__choice">読む線: ${item.readCues.map(escapeHtml).join(" / ")}</div>`
          : "";
        return `
          <li class="review__item">
            <div class="review__head">
              <span>${i + 1}. ${escapeHtml(item.positionJp)}</span>
              <span class="review__result ${resultClass}">${result}</span>
            </div>
            <div class="review__choice">選択: ${escapeHtml(item.chosenJp)}</div>
            ${actionLine}
            ${actionCueLine}
            ${readLine}
            ${item.continuation ? `<div class="review__choice">展開: ${escapeHtml(item.continuation)}</div>` : ""}
            ${tempoLine}
            ${stateLine}
            ${nextLine}
            ${nextReason}
            <div class="review__choice">流れ: ${escapeHtml(flowState(item.flow || 0).label)}</div>
            <div class="review__principle">${safeInlineHtml(item.principle)}</div>
          </li>`;
      })
      .join("");
    const mission = missionResult
      ? `<div class="mission mission--result">
          <span>今回の狙い</span>
          <b>${escapeHtml(missionResult.label)} ${missionResult.achieved ? "達成" : "未達"}</b>
          ${escapeHtml(missionResult.progress)}
          ${missionResult.awarded ? ` ・ +${missionResult.awarded} pt` : ""}
        </div>`
      : "";
    const tacticBlock = tactic
      ? `<div class="mission mission--tactic mission--result">
          <span>今回の制約</span>
          <b>${escapeHtml(tactic.label)}</b>
          ${escapeHtml(tactic.text)}
        </div>`
      : "";
    const coachBlock = coachReview
      ? `<div class="coach-card">
          <span>次の稽古</span>
          <b>${escapeHtml(coachReview.headline)}</b>
          <p>${escapeHtml(coachReview.focus)}</p>
          <p>${escapeHtml(coachReview.drill)}</p>
        </div>`
      : "";
    const weaknessButton = weaknessFocus
      ? `<button class="next-btn next-btn--drill" type="button" data-action="drill">
          苦手局面から再ロール
          <span>${escapeHtml(weaknessFocus.label)}${weaknessFocus.actionLabel ? ` / ${escapeHtml(weaknessFocus.actionLabel)}` : ""}</span>
        </button>`
      : "";
    this.lesson.innerHTML = `
      <div class="complete">
        <h2>ロール終了 — 黙想</h2>
        <div class="belt-big">${belt.icon}</div>
        <p>到達した帯: <b>${escapeHtml(belt.name)}</b></p>
        <p>正解 ${correct} / ${total} ・ 道場ポイント ${score} ・ 流れ ${escapeHtml(finalFlow.label)}</p>
        ${mission}
        ${tacticBlock}
        ${coachBlock}
        <div class="situation" style="text-align:left;margin-top:18px">
          一本のロールを ${escapeHtml(modeText)} の視点で通しました。柔術は「負けて学ぶ」武術です。
          タップした局面こそ伸びしろ。間違えた判断を「もう一度」で復習し、原則を身体で覚えましょう。
          <br><br>覚えておく背骨: <b>① 首を守る ② 良い位置を取る (position before submission) ③ 力でなくテコ</b>。
        </div>
        ${review ? `<ol class="review">${review}</ol>` : ""}
        ${weaknessButton}
        <button class="next-btn" type="button">もう一度 ロールする</button>
      </div>
    `;
    this.lesson.querySelector('[data-action="drill"]')?.addEventListener("click", weaknessFocus?.onStart);
    this.lesson.querySelector(".next-btn:not(.next-btn--drill)").addEventListener("click", onRestart);
    this.lesson.scrollTop = 0;
  }

  bindControls({ onRollMode, onUniformMode, onDifficultyMode, onOpponentStyleMode }) {
    if (this._controlsBound) return;
    this.mixBtn?.addEventListener("click", () => onRollMode("mixed"));
    this.defBtn?.addEventListener("click", () => onRollMode("defense"));
    this.offBtn?.addEventListener("click", () => onRollMode("offense"));
    this.giBtn?.addEventListener("click", () => onUniformMode("gi"));
    this.nogiBtn?.addEventListener("click", () => onUniformMode("nogi"));
    this.beginnerBtn?.addEventListener("click", () => onDifficultyMode("beginner"));
    this.liveBtn?.addEventListener("click", () => onDifficultyMode("live"));
    this.styleRandomBtn?.addEventListener("click", () => onOpponentStyleMode("random"));
    this.stylePressureBtn?.addEventListener("click", () => onOpponentStyleMode("pressure-passer"));
    this.styleChokeBtn?.addEventListener("click", () => onOpponentStyleMode("choke-hunter"));
    this.styleGuardBtn?.addEventListener("click", () => onOpponentStyleMode("guard-player"));
    this._controlsBound = true;
  }

  setControlsUI(rollMode, uniformMode, difficultyMode = "beginner", styleMode = "random") {
    if (this.mixBtn) this.mixBtn.classList.toggle("is-active", rollMode === "mixed");
    if (this.defBtn) this.defBtn.classList.toggle("is-active", rollMode === "defense");
    if (this.offBtn) this.offBtn.classList.toggle("is-active", rollMode === "offense");
    this.setModeUI(uniformMode);
    if (this.beginnerBtn) this.beginnerBtn.classList.toggle("is-active", difficultyMode === "beginner");
    if (this.liveBtn) this.liveBtn.classList.toggle("is-active", difficultyMode === "live");
    if (this.styleRandomBtn) this.styleRandomBtn.classList.toggle("is-active", styleMode === "random");
    if (this.stylePressureBtn) this.stylePressureBtn.classList.toggle("is-active", styleMode === "pressure-passer");
    if (this.styleChokeBtn) this.styleChokeBtn.classList.toggle("is-active", styleMode === "choke-hunter");
    if (this.styleGuardBtn) this.styleGuardBtn.classList.toggle("is-active", styleMode === "guard-player");
  }

  setModeUI(mode) {
    if (this.giBtn) this.giBtn.classList.toggle("is-active", mode === "gi");
    if (this.nogiBtn) this.nogiBtn.classList.toggle("is-active", mode === "nogi");
  }

  _bindNumberKeys(onChoose) {
    if (this._keyHandler) document.removeEventListener("keydown", this._keyHandler);
    this._keyHandler = (event) => {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
      if (isTypingTarget(event.target)) return;
      const index = choiceIndexForKey(event, 9);
      if (index < 0) return;
      const btn = this.lesson.querySelector(`.option[data-i="${index}"]`);
      if (!btn || btn.disabled) return;
      event.preventDefault();
      onChoose(index);
    };
    document.addEventListener("keydown", this._keyHandler);
  }

  _bindFeedbackKeys(onNext, onReplay) {
    this._unbindFeedbackKeys();
    this._feedbackKeyHandler = (event) => {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
      if (isTypingTarget(event.target)) return;
      if (event.key === "Enter") {
        event.preventDefault();
        onNext();
        return;
      }
      if (event.key.toLowerCase() === "r") {
        event.preventDefault();
        onReplay();
      }
    };
    document.addEventListener("keydown", this._feedbackKeyHandler);
  }

  _unbindFeedbackKeys() {
    if (!this._feedbackKeyHandler) return;
    document.removeEventListener("keydown", this._feedbackKeyHandler);
    this._feedbackKeyHandler = null;
  }

  _setupDojoNotes() {
    const toggle = document.getElementById("dojo-toggle");
    const notes = document.getElementById("dojo-notes");
    notes.innerHTML = `
      <h4>ポジション階層 (上ほど支配的)</h4>
      <ul>${DOJO_NOTES.hierarchy.map((h) => `<li>${escapeHtml(h)}</li>`).join("")}</ul>
      <h4>柔術の普遍原則</h4>
      <ul>${DOJO_NOTES.principles
        .map(([t, d]) => `<li><b>${escapeHtml(t)}</b> — ${escapeHtml(d)}</li>`)
        .join("")}</ul>
      <h4 class="glossary">用語</h4>
      <ul class="glossary">${DOJO_NOTES.glossary
        .map(([t, d]) => `<li><b>${escapeHtml(t)}</b>: ${escapeHtml(d)}</li>`)
        .join("")}</ul>
      <h4>研究の裏付け (英語文献・最新研究)</h4>
      <ul class="glossary">${DOJO_NOTES.research
        .map(([t, d]) => `<li><b>${escapeHtml(t)}</b> — ${escapeHtml(d)}</li>`)
        .join("")}</ul>
    `;
    toggle.addEventListener("click", () => {
      const open = notes.hasAttribute("hidden");
      if (open) notes.removeAttribute("hidden");
      else notes.setAttribute("hidden", "");
      toggle.textContent = open
        ? "道場の心得 / Dojo Principles ▴"
        : "道場の心得 / Dojo Principles ▾";
    });
  }
}

function escapeHtml(str) {
  return String(str).replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
  );
}

function isTypingTarget(target) {
  return target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

export function choiceIndexForKey(event, maxChoices = 9) {
  const fromKey = Number(event?.key);
  if (Number.isInteger(fromKey) && fromKey >= 1 && fromKey <= maxChoices) return fromKey - 1;
  const code = String(event?.code || "");
  const match = /^(?:Digit|Numpad)([1-9])$/.exec(code);
  if (!match) return -1;
  const fromCode = Number(match[1]);
  return fromCode <= maxChoices ? fromCode - 1 : -1;
}

function safeInlineHtml(str) {
  return escapeHtml(str)
    .replace(/&lt;b&gt;/g, "<b>")
    .replace(/&lt;\/b&gt;/g, "</b>");
}

function flowState(flow) {
  if (flow >= 2) return { label: "優勢", tone: "good" };
  if (flow === 1) return { label: "前進", tone: "good" };
  if (flow === -1) return { label: "危険", tone: "bad" };
  if (flow <= -2) return { label: "劣勢", tone: "bad" };
  return { label: "五分", tone: "neutral" };
}

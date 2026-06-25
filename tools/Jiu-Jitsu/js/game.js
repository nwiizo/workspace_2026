// game.js
// ゲーム状態機械: 一本の連続ロールを進行・採点し、Dojo (3D) と UI を駆動する。
// フロー: setup を見せる → attack (決断の瞬間) → 選択 → result を実演 →
//         正解なら少し見せてから自動で次局面へ (連続して動き続ける)。
// gi / no-gi モードに応じて選択肢 (技セット) を出し分ける。

import { POSES } from "./poses.js";
import { POSE_SPECS } from "./poseSpecs.js";
import { rolePairAllowed } from "./positionCatalog.js";
import { OFFENSE_SCENARIOS, SCENARIOS } from "./techniques.js";

function shuffle(arr) {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

function randomInt(min, max) {
  return min + Math.floor(Math.random() * (max - min + 1));
}

function normalizeNext(entry) {
  if (typeof entry === "string") return { id: entry, weight: 1 };
  if (entry && typeof entry.id === "string") {
    const weight = Number.isFinite(entry.weight) && entry.weight > 0 ? entry.weight : 1;
    return { id: entry.id, weight };
  }
  return null;
}

function weightedChoice(entries) {
  const total = entries.reduce((sum, entry) => sum + entry.weight, 0);
  let pick = Math.random() * total;
  for (const entry of entries) {
    pick -= entry.weight;
    if (pick <= 0) return entry;
  }
  return entries.at(-1) || null;
}

const BELTS = [
  { name: "白帯", icon: "⬜", min: 0 },
  { name: "青帯", icon: "🟦", min: 2 },
  { name: "紫帯", icon: "🟪", min: 3 },
  { name: "茶帯", icon: "🟫", min: 4 },
  { name: "黒帯", icon: "⬛", min: 5 },
];

const ATTACK_DELAY = 1500; // setup → attack
const ADVANCE_DELAY = 2300; // 正解の result を見せてから次局面へ
const DECISION_LIMIT = 9; // 相手が動いてくるまでの判断秒数
const OPPONENT_STYLES = [
  {
    id: "pressure-passer",
    label: "プレッシャーパサー",
    preferred: ["side-escape", "mount-escape", "attack-from-side", "attack-from-mount"],
  },
  {
    id: "choke-hunter",
    label: "絞めハンター",
    preferred: ["back-defense", "attack-from-back", "attack-triangle-guard"],
  },
  {
    id: "guard-player",
    label: "ガードプレイヤー",
    preferred: ["closed-guard-posture", "attack-armbar-guard", "attack-triangle-guard", "side-escape"],
  },
];
const ROLL_MISSIONS = [
  {
    id: "stay-safe",
    label: "安全第一",
    text: "時間切れなしで終え、危険な流れで終わらない",
    modes: ["mixed", "defense", "offense"],
    bonus: 8,
    target: { maxTimedOut: 0, minFinalFlow: 0 },
  },
  {
    id: "survive-to-attack",
    label: "守って攻めへ",
    text: "混合ロールで攻撃局面を1回以上正解する",
    modes: ["mixed"],
    bonus: 10,
    target: { minOffenseCorrect: 1 },
  },
  {
    id: "defensive-shell",
    label: "守りの骨格",
    text: "防御局面を2回以上正解する",
    modes: ["mixed", "defense"],
    bonus: 8,
    target: { minDefenseCorrect: 2 },
  },
  {
    id: "chain-reactions",
    label: "反応を追う",
    text: "2連続正解を作る",
    modes: ["mixed", "defense", "offense"],
    bonus: 8,
    target: { minMaxStreak: 2 },
  },
];
const ROLL_TACTICS = [
  {
    id: "survive-first",
    label: "生存優先",
    text: "首・肘・腰を先に守る。危険な流れでは守り局面が出やすい",
    modes: ["mixed", "defense"],
    timeDelta: 1,
    preferred: ["back-defense", "mount-escape", "side-escape", "closed-guard-posture"],
    roleBias: { defense: 1.25 },
  },
  {
    id: "position-ladder",
    label: "位置を上げる",
    text: "極めを急がず、サイド・マウント・バックへ階層を登る",
    modes: ["mixed", "offense"],
    timeDelta: 0,
    preferred: ["attack-from-side", "attack-from-mount", "attack-from-back"],
    roleBias: { offense: 1.2 },
  },
  {
    id: "submission-chain",
    label: "極めの連鎖",
    text: "腕十字・三角・バックを相手の反応でつなぐ",
    modes: ["mixed", "offense"],
    timeDelta: 0,
    preferred: ["attack-armbar-guard", "attack-triangle-guard", "attack-from-back"],
    roleBias: { offense: 1.15 },
  },
  {
    id: "fast-scramble",
    label: "速いスクランブル",
    text: "相手の反応が速い。判断時間が短く、展開が切り替わりやすい",
    modes: ["mixed", "defense", "offense"],
    timeDelta: -1,
    preferred: ["side-escape", "attack-from-back", "attack-from-side"],
    roleBias: {},
  },
];
const ROLL_STATE_LABELS = {
  "neck-safe": "首を守った",
  "neck-exposed": "首が開いた",
  "top-base": "上のベース",
  "guard-recovered": "ガード回復",
  "posture-safe": "姿勢を守った",
  "arm-exposed": "腕が伸びた",
  "back-exposed": "背中を見せた",
  "frame-lost": "フレーム喪失",
  "posture-broken": "姿勢崩れ",
  "angle-created": "角度を作った",
  "knee-shield": "膝盾が入った",
  "stack-pressure": "重ね圧",
};

export class Game {
  constructor(dojo, ui) {
    this.dojo = dojo;
    this.ui = ui;
    this.runScenarios = [];
    this.index = 0;
    this.score = 0;
    this.correctCount = 0;
    this.streak = 0;
    this.flow = 0;
    this.answered = false;
    this.decisionOpen = false;
    this.history = [];
    const initial = this._initialModes();
    this.rollMode = initial.rollMode; // "mixed" | "defense" | "offense"
    this.uniformMode = initial.uniformMode; // "gi" | "nogi"
    this.difficultyMode = initial.difficultyMode; // "beginner" | "live"
    this.opponentStyleMode = initial.opponentStyleMode; // "random" | OPPONENT_STYLES[].id
    this.scoredScenarioIds = new Set();
    this.resolvedIndexes = new Set();
    this.pendingNextIds = [];
    this.opponentStyle = this._pickOpponentStyle();
    this.mission = null;
    this.tactic = null;
    this.adaptiveFocus = null;
    this.rollState = new Set();
    this.missionBonusAwarded = false;
    this.runStats = this._emptyRunStats();
    this.decisionLimit = 0;
    this.secondsLeft = 0;
    this.currentAction = null;
    this.drillFocus = null;
  }

  _initialModes() {
    const params = new URLSearchParams(window.location.search);
    const mode = params.get("mode");
    const uniform = params.get("uniform");
    const difficulty = params.get("difficulty");
    const style = params.get("style");
    return {
      rollMode: ["mixed", "defense", "offense"].includes(mode) ? mode : "mixed",
      uniformMode: uniform === "nogi" ? "nogi" : "gi",
      difficultyMode: difficulty === "live" ? "live" : "beginner",
      opponentStyleMode: OPPONENT_STYLES.some((s) => s.id === style) ? style : "random",
    };
  }

  _pickOpponentStyle() {
    return OPPONENT_STYLES.find((s) => s.id === this.opponentStyleMode) || shuffle(OPPONENT_STYLES)[0];
  }

  _pickMission() {
    return shuffle(ROLL_MISSIONS.filter((mission) => mission.modes.includes(this.rollMode)))[0];
  }

  _pickTactic() {
    return shuffle(ROLL_TACTICS.filter((tactic) => tactic.modes.includes(this.rollMode)))[0];
  }

  _emptyRunStats() {
    return {
      correct: 0,
      timedOut: 0,
      offenseCorrect: 0,
      defenseCorrect: 0,
      maxStreak: 0,
      quickCorrect: 0,
      tempoBonus: 0,
    };
  }

  setRollMode(mode) {
    if (!["mixed", "defense", "offense"].includes(mode)) return;
    if (mode === this.rollMode) return;
    this.rollMode = mode;
    this.start();
  }

  setUniformMode(mode) {
    if (mode !== "gi" && mode !== "nogi") return;
    if (mode === this.uniformMode) return;
    this.uniformMode = mode;
    this.dojo.setUniformMode(mode);
    this.ui.setControlsUI(this.rollMode, this.uniformMode, this.difficultyMode, this.opponentStyleMode);
    if (!this.answered) this._renderCurrent();
  }

  setDifficultyMode(mode) {
    if (mode !== "beginner" && mode !== "live") return;
    if (mode === this.difficultyMode) return;
    this.difficultyMode = mode;
    this.ui.setControlsUI(this.rollMode, this.uniformMode, this.difficultyMode, this.opponentStyleMode);
    if (!this.answered) this._renderCurrent();
  }

  setOpponentStyleMode(mode) {
    if (mode !== "random" && !OPPONENT_STYLES.some((s) => s.id === mode)) return;
    if (mode === this.opponentStyleMode) return;
    this.opponentStyleMode = mode;
    this.start();
  }

  start({ drillFocus = null } = {}) {
    this.ui.bindControls({
      onRollMode: (mode) => this.setRollMode(mode),
      onUniformMode: (mode) => this.setUniformMode(mode),
      onDifficultyMode: (mode) => this.setDifficultyMode(mode),
      onOpponentStyleMode: (mode) => this.setOpponentStyleMode(mode),
    });
    this.ui.setControlsUI(this.rollMode, this.uniformMode, this.difficultyMode, this.opponentStyleMode);
    this.dojo.setUniformMode(this.uniformMode);
    clearTimeout(this._adv);
    clearTimeout(this._t);
    clearInterval(this._pressureTimer);
    this.adaptiveFocus = drillFocus ? null : this._adaptiveFocusFromHistory();
    this.drillFocus = drillFocus;
    this.opponentStyle = this._pickOpponentStyle();
    this.mission = this._pickMission();
    this.tactic = this._pickTactic();
    this.runScenarios = this._buildRunScenarios();
    this.index = 0;
    this.score = 0;
    this.correctCount = 0;
    this.streak = 0;
    this.flow = 0;
    this.history = [];
    this.scoredScenarioIds.clear();
    this.resolvedIndexes.clear();
    this.pendingNextIds = [];
    this.rollState.clear();
    this.missionBonusAwarded = false;
    this.runStats = this._emptyRunStats();
    this._loadScenario();
  }

  get current() {
    return this.runScenarios[this.index];
  }

  get scenarios() {
    return this.runScenarios.length ? this.runScenarios : this._scenarioPool();
  }

  _scenarioPool() {
    if (this.rollMode === "offense") return OFFENSE_SCENARIOS;
    if (this.rollMode === "defense") return SCENARIOS;
    return this._allScenarios();
  }

  _allScenarios() {
    return [...SCENARIOS, ...OFFENSE_SCENARIOS];
  }

  _buildRunScenarios() {
    const forced = this._forcedScenario();
    if (this.rollMode === "offense") {
      const pool = this._shuffleForOpponentStyle(this._scenarioPool());
      return this._withForcedScenario(pool, forced).slice(0, Math.min(pool.length, randomInt(4, 5)));
    }
    if (this.rollMode === "defense") {
      const pool = this._shuffleForOpponentStyle(SCENARIOS);
      return this._withForcedScenario(pool, forced).slice(0, Math.min(pool.length, randomInt(3, 4)));
    }

    const defense = this._shuffleForOpponentStyle(SCENARIOS);
    const offense = this._shuffleForOpponentStyle(OFFENSE_SCENARIOS);
    const defenseCount = Math.min(defense.length, randomInt(2, 3));
    const offenseCount = Math.min(offense.length, randomInt(2, 3));
    const run = [...defense.slice(0, defenseCount), ...offense.slice(0, offenseCount)];
    return this._withForcedScenario(run, forced).slice(0, Math.min(run.length, randomInt(4, 5)));
  }

  _shuffleForOpponentStyle(pool) {
    const preferredIds = new Set([
      ...(this.opponentStyle?.preferred || []),
      ...(this.tactic?.preferred || []),
    ]);
    const adaptive = shuffle(pool.filter((scenario) => this._matchesAdaptiveFocus(scenario)));
    const adaptiveIds = new Set(adaptive.map((scenario) => scenario.id));
    const preferred = shuffle(pool.filter((scenario) => preferredIds.has(scenario.id) && !adaptiveIds.has(scenario.id)));
    const rest = shuffle(pool.filter((scenario) => !preferredIds.has(scenario.id) && !adaptiveIds.has(scenario.id)));
    if (this.adaptiveFocus) return [...adaptive, ...preferred, ...rest];
    return [...preferred, ...rest];
  }

  _adaptiveFocusFromHistory() {
    const missed = this.history.filter((entry) => entry && (!entry.correct || entry.timedOut));
    if (!missed.length) return null;
    const counts = new Map();
    for (const entry of missed) {
      for (const cue of entry.readCues || []) counts.set(cue, (counts.get(cue) || 0) + 1);
    }
    const cue = [...counts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
    if (!cue) return null;
    return {
      cue,
      label: `${cue} の読み直し`,
      text: "前回ミスした読む線に関係する局面が少し出やすくなります",
    };
  }

  _matchesAdaptiveFocus(scenario) {
    const cue = this.adaptiveFocus?.cue;
    if (!cue) return false;
    if (scenario.readCues?.includes(cue)) return true;
    return (scenario.opponentActions || []).some((action) => action.readCues?.includes(cue));
  }

  _forcedScenario() {
    if (this.drillFocus?.scenarioId) {
      return this._allScenarios().find((scenario) => scenario.id === this.drillFocus.scenarioId) || null;
    }
    const id = new URLSearchParams(window.location.search).get("scenario");
    if (!id) return null;
    return this._allScenarios().find((scenario) => scenario.id === id) || null;
  }

  _withForcedScenario(run, forced) {
    if (!forced) return run;
    const rest = run.filter((scenario) => scenario.id !== forced.id);
    return [forced, ...rest];
  }

  beltFor(count) {
    let belt = BELTS[0];
    for (const b of BELTS) if (count >= b.min) belt = b;
    return belt;
  }

  _pose(id) {
    const p = POSES[id];
    if (!p) throw new Error(`未定義のポーズ: ${id}`);
    return p;
  }

  _setPosePair(redId, blueId, stage) {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole || !rolePairAllowed(redRole, blueRole)) {
      throw new Error(`未許可の体勢ペア: ${stage}:${redId}/${blueId}:${redRole || "unknown"}/${blueRole || "unknown"}`);
    }
    this.dojo.setPoses(this._pose(redId), this._pose(blueId));
  }

  // モードに合った選択肢のみ抽出 (giOnly / nogiOnly でフィルタ)
  _visibleOptions(s) {
    const options = s.options.filter((o) => {
      if (o.giOnly && this.uniformMode !== "gi") return false;
      if (o.nogiOnly && this.uniformMode !== "nogi") return false;
      if (!this._optionMatchesState(o)) return false;
      if (!this._optionMatchesAction(o, s)) return false;
      return true;
    });
    if (this.difficultyMode !== "beginner" || options.length <= 3) return options;
    const correct = options.find((option) => option.correct);
    const wrong = shuffle(options.filter((option) => !option.correct)).slice(0, 2);
    return shuffle([correct, ...wrong].filter(Boolean));
  }

  _optionMatchesState(option) {
    const required = option.requiresState || [];
    const forbidden = option.forbiddenState || [];
    return (
      required.every((flag) => this.rollState.has(flag)) &&
      forbidden.every((flag) => !this.rollState.has(flag))
    );
  }

  _optionMatchesAction(option, scenario) {
    const actionId = scenario.opponentAction?.id;
    const required = option.requiresAction || [];
    const forbidden = option.forbiddenAction || [];
    return (
      (!required.length || required.includes(actionId)) &&
      forbidden.every((id) => id !== actionId)
    );
  }

  _stateLabels(flags = [...this.rollState]) {
    return flags.map((flag) => ROLL_STATE_LABELS[flag] || flag);
  }

  _stateEffectLabels(effects) {
    const added = effects?.add || [];
    const removed = effects?.remove || [];
    return {
      added: this._stateLabels(added),
      removed: this._stateLabels(removed),
    };
  }

  _applyStateEffects(effects) {
    for (const flag of effects?.remove || []) this.rollState.delete(flag);
    for (const flag of effects?.add || []) this.rollState.add(flag);
  }

  _actionWeight(action) {
    const styleMatch = action.styles?.includes(this.opponentStyle?.id);
    const tacticMatch = action.tactics?.includes(this.tactic?.id);
    return (action.weight || 1) * (styleMatch ? 2 : 1) * (tacticMatch ? 1.6 : 1);
  }

  _defaultOpponentAction(scenario) {
    return {
      id: "default",
      label: "基本の初動",
      attack: scenario.attack,
      pressure: scenario.pressure,
      readCues: scenario.readCues,
      weight: 1,
    };
  }

  _selectOpponentAction(scenario) {
    if (this.drillFocus?.actionId && scenario.id === this.drillFocus.scenarioId) {
      const action = scenario.opponentActions?.find((item) => item.id === this.drillFocus.actionId);
      if (action) return action;
    }
    const actions = Array.isArray(scenario.opponentActions) && scenario.opponentActions.length
      ? scenario.opponentActions
      : [this._defaultOpponentAction(scenario)];
    const weighted = actions.map((action) => ({
      ...action,
      weight: this._actionWeight(action),
    }));
    return weightedChoice(weighted) || this._defaultOpponentAction(scenario);
  }

  _materializeScenario(scenario, action = this.currentAction) {
    if (!action) return scenario;
    return {
      ...scenario,
      attack: action.attack || scenario.attack,
      pressure: action.pressure || scenario.pressure,
      readCues: action.readCues || scenario.readCues,
      opponentAction: {
        id: action.id,
        label: action.label,
        cue: action.cue,
      },
    };
  }

  _renderCurrent({ reshuffle = true, revealAction = this.decisionOpen } = {}) {
    const s = revealAction ? this._materializeScenario(this.current) : this.current;
    const actionChanged = this.view?.opponentAction?.id !== s.opponentAction?.id;
    if (reshuffle || !this.view || this.view.id !== s.id || actionChanged) {
      this.view = { ...s, options: shuffle(this._visibleOptions(s)) };
    } else {
      this.view = { ...s, options: this.view.options };
    }
    this.ui.renderLesson(this.view, (i) => this._answer(i), {
      rollMode: this.rollMode,
      uniformMode: this.uniformMode,
      difficultyMode: this.difficultyMode,
      opponentStyle: this.opponentStyle,
      mission: this.mission,
      tactic: this.tactic,
      adaptiveFocus: this.adaptiveFocus,
      rollState: this._stateLabels(),
      decisionOpen: this.decisionOpen,
      index: this.index,
      total: this.scenarios.length,
    });
  }

  _loadScenario() {
    clearTimeout(this._adv);
    const s = this.current;
    this.answered = false;
    this.decisionOpen = false;
    this.decisionLimit = 0;
    this.secondsLeft = 0;
    this.currentAction = this._selectOpponentAction(s);
    this.drillFocus = null;
    this.dojo.setAutoRotate(false);

    this._setPosePair(s.setup.red, s.setup.blue, `${s.id}:setup`);
    this.ui.renderBadge(s.setup.badge);
    this.ui.renderScore(this.score, this.beltFor(this.correctCount), this.streak, this.flow);
    this._renderCurrent({ revealAction: false });

    clearTimeout(this._t);
    this._t = setTimeout(() => {
      if (this.answered) return;
      const active = this._materializeScenario(s);
      this._setPosePair(active.attack.red, active.attack.blue, `${active.id}:attack`);
      this.ui.renderBadge(active.attack.badge);
      this.decisionOpen = true;
      this._renderCurrent({ reshuffle: false, revealAction: true });
      this._startPressure(active);
    }, ATTACK_DELAY);
  }

  _startPressure(s) {
    clearInterval(this._pressureTimer);
    if (!this._timeLimitEnabled()) {
      this.decisionLimit = 0;
      this.secondsLeft = 0;
      this.ui.renderPressure(null, this._pressureText(s, Number.POSITIVE_INFINITY));
      return;
    }
    let left = this._timeLimit(s);
    this.decisionLimit = left;
    this.secondsLeft = left;
    this.ui.renderPressure(left, this._pressureText(s, left));
    this._pressureTimer = setInterval(() => {
      if (this.answered) {
        clearInterval(this._pressureTimer);
        return;
      }
      left -= 1;
      this.secondsLeft = Math.max(0, left);
      this.ui.renderPressure(left, this._pressureText(s, left));
      if (left <= 0) {
        clearInterval(this._pressureTimer);
        this._answer(this._timeoutOptionIndex(), { timedOut: true });
      }
    }, 1000);
  }

  _pressureText(s, secondsLeft) {
    const urgent = secondsLeft <= 3;
    if (!Number.isFinite(secondsLeft)) {
      if (s.pressure?.early) return s.pressure.early;
      return s.role === "offense"
        ? "相手の逃げ道を見て、落ち着いて攻めの順序を選ぶ"
        : "相手の圧を見て、落ち着いて危険な線を先に消す";
    }
    if (this.difficultyMode === "live" && !urgent) {
      if (this.opponentStyle?.id === "pressure-passer") {
        return "相手は圧で形を潰してくる。腰とフレームの線を急いで守る";
      }
      if (this.opponentStyle?.id === "choke-hunter") {
        return "相手は首と肩の線を探している。頭と手の位置を先に消す";
      }
      if (this.opponentStyle?.id === "guard-player") {
        return "相手は角度を作っている。肘と姿勢を戻す判断が必要";
      }
      return s.role === "offense"
        ? "相手が逃げ道を作っている。形が崩れる前に判断する"
        : "相手の圧が変わった。危険な線を先に消す";
    }
    if (s.pressure) return urgent ? s.pressure.urgent : s.pressure.early;
    if (s.role === "offense" || this.rollMode === "offense") {
      return urgent
        ? "相手がフレームを差し込み、逃げ道を作っている"
        : "相手が腰をずらし、防御手を戻そうとしている";
    }
    return urgent
      ? "相手の極めが深くなる。今動かないと遅い"
      : "相手が圧を強め、次の攻撃へ移っている";
  }

  _timeLimit(s) {
    const base = s.timeLimitSec || DECISION_LIMIT;
    const difficultyDelta = this.difficultyMode === "live" ? -2 : 3;
    const flowDelta = this.difficultyMode === "live" ? Math.max(-1, Math.min(1, this.flow)) : 0;
    const tacticDelta = this.tactic?.timeDelta || 0;
    return Math.max(5, base + difficultyDelta + flowDelta + tacticDelta);
  }

  _timeLimitEnabled() {
    return this.difficultyMode === "live";
  }

  _scenarioAllowed(scenario) {
    if (this.rollMode === "offense") return scenario.role === "offense";
    if (this.rollMode === "defense") return scenario.role === "defense";
    return true;
  }

  _scenarioAlreadySeen(id) {
    return this.history.some((entry, i) => i <= this.index && entry?.id === id);
  }

  _scenarioAlreadyScheduled(id, targetIndex) {
    return this.runScenarios.some(
      (scenario, i) => i > this.index && i !== targetIndex && scenario?.id === id,
    );
  }

  _flowRoleMultiplier(scenario) {
    if (this.rollMode !== "mixed") return 1;
    if (this.flow >= 1 && scenario.role === "offense") return 1.6;
    if (this.flow <= -1 && scenario.role === "defense") return 1.6;
    return 1;
  }

  _tacticRoleMultiplier(scenario) {
    return this.tactic?.roleBias?.[scenario.role] || 1;
  }

  _matchingStateBias(scenario) {
    return (scenario.stateBias || []).filter((flag) => this.rollState.has(flag));
  }

  _stateBiasMultiplier(scenario) {
    const hits = this._matchingStateBias(scenario).length;
    if (!hits) return 1;
    return Math.min(2.2, 1 + hits * 0.45);
  }

  _stateBiasReasonText(scenario) {
    const labels = this._matchingStateBias(scenario)
      .map((flag) => ROLL_STATE_LABELS[flag] || flag);
    if (!labels.length) return "";
    return `引き継ぎ状態: ${labels.join(" / ")}`;
  }

  _nextReasonFor(scenario, selectedByGraph) {
    const stateReason = this._stateBiasReasonText(scenario);
    if (!selectedByGraph) {
      return stateReason ? `予定されたロール順 / ${stateReason}` : "予定されたロール順";
    }
    const styleName = this.opponentStyle?.label || "相手";
    if (this.rollMode === "mixed") {
      if (this.flow >= 1 && scenario.role === "offense") {
        const reason = `流れが前へ出たため、${styleName}の反応から攻めへ接続`;
        return stateReason ? `${reason} / ${stateReason}` : reason;
      }
      if (this.flow <= -1 && scenario.role === "defense") {
        const reason = `流れが危険なため、${styleName}の追撃から守りへ接続`;
        return stateReason ? `${reason} / ${stateReason}` : reason;
      }
    }
    const reason = `${styleName}の反応から分岐`;
    return stateReason ? `${reason} / ${stateReason}` : reason;
  }

  _nextCandidateWeight(entry, scenario, preferredIds) {
    const tacticPreferredIds = new Set(this.tactic?.preferred || []);
    return entry.weight *
      (preferredIds.has(entry.id) ? 1.8 : 1) *
      (tacticPreferredIds.has(entry.id) ? 1.45 : 1) *
      this._flowRoleMultiplier(scenario) *
      this._tacticRoleMultiplier(scenario) *
      this._stateBiasMultiplier(scenario);
  }

  _timeoutOptionWeight(option, preferredIds, scenariosById) {
    const nextEntries = (option.next || []).map(normalizeNext).filter(Boolean);
    if (!nextEntries.length) return 1;
    return nextEntries.reduce((sum, entry) => {
      const scenario = scenariosById.get(entry.id);
      const styleFactor = preferredIds.has(entry.id) ? 2.2 : 1;
      const tacticFactor = this.tactic?.preferred?.includes(entry.id) ? 1.5 : 1;
      const flowFactor = scenario ? this._flowRoleMultiplier(scenario) : 1;
      const roleFactor = scenario ? this._tacticRoleMultiplier(scenario) : 1;
      const stateFactor = scenario ? this._stateBiasMultiplier(scenario) : 1;
      return sum + entry.weight * styleFactor * tacticFactor * flowFactor * roleFactor * stateFactor;
    }, 1);
  }

  _timeoutOptionIndex() {
    const preferredIds = new Set(this.opponentStyle?.preferred || []);
    const scenariosById = new Map(this._allScenarios().map((scenario) => [scenario.id, scenario]));
    const candidates = this.view.options
      .map((option, index) => ({
        index,
        correct: option.correct,
        weight: this._timeoutOptionWeight(option, preferredIds, scenariosById),
      }))
      .filter((entry) => !entry.correct);
    return weightedChoice(candidates)?.index ?? 0;
  }

  _tempoForAnswer(option, timedOut = false) {
    if (!option?.correct || timedOut || !this.decisionLimit) {
      return { id: "none", label: "テンポなし", text: "", bonus: 0 };
    }
    const ratio = Math.max(0, this.secondsLeft) / this.decisionLimit;
    if (ratio >= 0.67) {
      return {
        id: "quick",
        label: "先手の判断",
        text: "相手の圧が深くなる前に正しい線を読めた",
        bonus: 2,
      };
    }
    if (ratio >= 0.34) {
      return {
        id: "stable",
        label: "安定した判断",
        text: "危険線が深くなる前に対応できた",
        bonus: 0,
      };
    }
    return {
      id: "late",
      label: "ぎりぎりの判断",
      text: "正解だが、相手の攻めが深くなる前にもう一拍早く選びたい",
      bonus: 0,
    };
  }

  _selectNextScenarioEntry() {
    if (!this.pendingNextIds.length) return null;
    const targetIndex = this.index + 1;
    const scenariosById = new Map(this._allScenarios().map((scenario) => [scenario.id, scenario]));
    const preferredIds = new Set(this.opponentStyle?.preferred || []);
    const candidates = this.pendingNextIds
      .map(normalizeNext)
      .filter(Boolean)
      .map((entry) => ({ ...entry, scenario: scenariosById.get(entry.id) }))
      .filter((entry) => entry.scenario)
      .filter((entry) => this._scenarioAllowed(entry.scenario))
      .filter((entry) => !this._scenarioAlreadySeen(entry.scenario.id))
      .filter((entry) => !this._scenarioAlreadyScheduled(entry.scenario.id, targetIndex))
      .map((entry) => ({
        ...entry,
        weight: this._nextCandidateWeight(entry, entry.scenario, preferredIds),
      }));
    return weightedChoice(candidates) || null;
  }

  _selectNextScenario() {
    return this._selectNextScenarioEntry()?.scenario || null;
  }

  _answer(optionIndex, { timedOut = false } = {}) {
    if (this.answered) return;
    if (!this.decisionOpen) return;
    this.answered = true;
    clearTimeout(this._t);
    clearInterval(this._pressureTimer);

    const s = this.view;
    const opt = s.options[optionIndex];
    this.dojo.setAutoRotate(false);

    this._setPosePair(opt.result.red, opt.result.blue, `${s.id}:result`);
    this.ui.renderBadge(opt.result.badge);
    this.pendingNextIds = [...(opt.next || [])];

    const firstResolution = !this.resolvedIndexes.has(this.index);
    const tempo = this._tempoForAnswer(opt, timedOut);
    const stateEffects = this._stateEffectLabels(opt.stateEffects);
    if (opt.correct) {
      if (firstResolution && !this.scoredScenarioIds.has(s.id)) {
        this.streak += 1;
        this.score += 10 + Math.max(0, (this.streak - 1) * 2); // 連続正解ボーナス
        this.score += tempo.bonus;
        this.correctCount += 1;
        this.scoredScenarioIds.add(s.id);
      }
      if (firstResolution) this.flow = Math.min(2, this.flow + 1);
    } else if (firstResolution) {
      this.streak = 0;
      this.score = Math.max(0, this.score - 3);
      this.flow = Math.max(-2, this.flow - (timedOut ? 2 : 1));
    }
    if (firstResolution) this._applyStateEffects(opt.stateEffects);
    if (firstResolution) this._recordRunStats(s, opt, timedOut);
    this.resolvedIndexes.add(this.index);
    this.history[this.index] = {
      id: s.id,
      role: s.role,
      positionJp: s.positionJp,
      prompt: s.prompt,
      chosenJp: opt.jp,
      opponentAction: s.opponentAction,
      readCues: s.readCues,
      continuation: opt.correct ? opt.reaction : opt.consequence,
      stateEffects,
      rollState: this._stateLabels(),
      flow: this.flow,
      correct: opt.correct,
      timedOut,
      tempo,
      principle: s.principle,
    };

    const isLast = this.index >= this.scenarios.length - 1;
    this.ui.renderScore(this.score, this.beltFor(this.correctCount), this.streak, this.flow);
    this.ui.renderFeedback(s, optionIndex, {
      isLast,
      autoAdvance: opt.correct && !isLast,
      advanceMs: ADVANCE_DELAY,
      timedOut,
      tempo,
      stateEffects,
      onNext: () => this._next(),
      onReplay: () => this._loadScenario(),
    });

    // 正解なら自動で次局面へ (連続したロールとして動き続ける)
    if (opt.correct && !isLast) {
      clearTimeout(this._adv);
      this._adv = setTimeout(() => this._next(), ADVANCE_DELAY);
    }
  }

  _next() {
    clearTimeout(this._adv);
    clearInterval(this._pressureTimer);
    if (this.index >= this.scenarios.length - 1) {
      const missionResult = this._missionResult();
      if (missionResult?.achieved && !this.missionBonusAwarded) {
        this.score += missionResult.bonus;
        missionResult.awarded = missionResult.bonus;
        this.missionBonusAwarded = true;
      }
      this.ui.renderComplete(
        this.score,
        this.correctCount,
        this.scenarios.length,
        this.beltFor(this.correctCount),
        this.rollMode,
        this.history,
        this.flow,
        missionResult,
        this.tactic,
        this._coachReview(),
        () => this.start(),
        this._weaknessFocus(),
      );
      this.dojo.setAutoRotate(false);
      this._setPosePair("standingRed", "standingBlue", "complete");
      this.ui.renderBadge("黙想 — お疲れさまでした");
      return;
    }
    const previousIndex = this.index;
    const nextIndex = this.index + 1;
    const nextEntry = this._selectNextScenarioEntry();
    const nextScenario = nextEntry?.scenario || null;
    this.index = nextIndex;
    if (nextScenario) this.runScenarios[this.index] = nextScenario;
    const resolvedNext = this.runScenarios[this.index];
    if (this.history[previousIndex] && resolvedNext) {
      this.history[previousIndex] = {
        ...this.history[previousIndex],
        nextPositionJp: resolvedNext.positionJp,
        nextRole: resolvedNext.role,
        nextSelectedByGraph: Boolean(nextScenario),
        nextReason: this._nextReasonFor(resolvedNext, Boolean(nextScenario)),
      };
    }
    this.pendingNextIds = [];
    this._loadScenario();
  }

  _recordRunStats(scenario, option, timedOut) {
    if (option.correct) {
      this.runStats.correct += 1;
      if (scenario.role === "offense") this.runStats.offenseCorrect += 1;
      if (scenario.role === "defense") this.runStats.defenseCorrect += 1;
      const tempo = this._tempoForAnswer(option, timedOut);
      if (tempo.id === "quick") this.runStats.quickCorrect += 1;
      this.runStats.tempoBonus += tempo.bonus;
    }
    if (timedOut) this.runStats.timedOut += 1;
    this.runStats.maxStreak = Math.max(this.runStats.maxStreak, this.streak);
  }

  _weaknessFocus() {
    const weak = this.history.find((entry) => entry && (!entry.correct || entry.timedOut));
    if (!weak) return null;
    return {
      scenarioId: weak.id,
      actionId: weak.opponentAction?.id || null,
      label: weak.positionJp,
      actionLabel: weak.opponentAction?.label || "",
      onStart: () => this.start({
        drillFocus: {
          scenarioId: weak.id,
          actionId: weak.opponentAction?.id || null,
        },
      }),
    };
  }

  _missionAchieved(mission = this.mission) {
    if (!mission) return false;
    const target = mission.target || {};
    const total = Math.max(1, this.scenarios.length);
    const correctRate = this.runStats.correct / total;
    if (target.maxTimedOut !== undefined && this.runStats.timedOut > target.maxTimedOut) return false;
    if (target.minFinalFlow !== undefined && this.flow < target.minFinalFlow) return false;
    if (target.minCorrectRate !== undefined && correctRate < target.minCorrectRate) return false;
    if (target.minOffenseCorrect !== undefined && this.runStats.offenseCorrect < target.minOffenseCorrect) return false;
    if (target.minDefenseCorrect !== undefined && this.runStats.defenseCorrect < target.minDefenseCorrect) return false;
    if (target.minMaxStreak !== undefined && this.runStats.maxStreak < target.minMaxStreak) return false;
    return true;
  }

  _missionProgress(mission = this.mission) {
    if (!mission) return "";
    const target = mission.target || {};
    if (target.minOffenseCorrect !== undefined) {
      return `攻撃正解 ${this.runStats.offenseCorrect}/${target.minOffenseCorrect}`;
    }
    if (target.minDefenseCorrect !== undefined) {
      return `防御正解 ${this.runStats.defenseCorrect}/${target.minDefenseCorrect}`;
    }
    if (target.minMaxStreak !== undefined) {
      return `最大連続 ${this.runStats.maxStreak}/${target.minMaxStreak}`;
    }
    if (target.maxTimedOut !== undefined) {
      return `時間切れ ${this.runStats.timedOut}/${target.maxTimedOut}`;
    }
    return `正解 ${this.runStats.correct}/${this.scenarios.length}`;
  }

  _missionResult() {
    const mission = this.mission;
    if (!mission) return null;
    return {
      label: mission.label,
      text: mission.text,
      bonus: mission.bonus,
      achieved: this._missionAchieved(mission),
      progress: this._missionProgress(mission),
      awarded: 0,
    };
  }

  _coachReview() {
    const total = Math.max(1, this.scenarios.length);
    const correctRate = this.runStats.correct / total;
    const firstMiss = this.history.find((entry) => entry && !entry.correct);
    if (this.runStats.timedOut > 0) {
      return {
        headline: "相手に先手を取られています",
        focus: "読む線を先に決め、タイマーが残っているうちに首・肘・腰のどれを守るか選ぶ",
        drill: "次は入門モードで同じ局面をリプレイし、相手の urgent 表示が出る前に数字キーで答える",
      };
    }
    if (correctRate >= 0.6 && this.runStats.quickCorrect === 0) {
      return {
        headline: "正解は取れていますが判断が遅めです",
        focus: "読む線を一つに絞り、相手の urgent 表示前に数字キーで決める",
        drill: "次は同じモードで、各局面の読みチップを見たら3秒以内に答える",
      };
    }
    if (this.rollMode === "mixed" && this.runStats.offenseCorrect === 0) {
      return {
        headline: "守りから攻めへの接続が弱いです",
        focus: "脱出で終わらず、上を取った直後にサイド・マウント・バックへ位置を上げる",
        drill: "次は攻撃フォーカスか「位置を上げる」制約で、正解後の next 理由を確認する",
      };
    }
    if (this.runStats.defenseCorrect === 0 && this.rollMode !== "offense") {
      return {
        headline: "最初の防御構造を作れていません",
        focus: "首、肘、腰の優先順位を守る。極められる前にフレームと姿勢を作る",
        drill: "次は防御フォーカスで、読む線チップを声に出してから 1〜4 を押す",
      };
    }
    if (correctRate >= 0.75 && this.flow >= 1) {
      return {
        headline: "主導権を維持できています",
        focus: "位置を保ったまま相手の反応を読み、極めを急がず次の支配へつなぐ",
        drill: "次は実戦モードで速いスクランブルを引き、同じ判断を短い時間で再現する",
      };
    }
    return {
      headline: firstMiss ? "崩れた局面を一つだけ直しましょう" : "基礎判断は安定しています",
      focus: firstMiss
        ? `${firstMiss.positionJp} の選択を復習し、相手の反応と次局面まで結びつける`
        : "正解の理由と相手の反応を見て、次局面まで一つの流れとして覚える",
      drill: "次は「この局面をもう一度」で一手だけ復習してから、別スタイルの相手でロールする",
    };
  }
}

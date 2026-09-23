// スクランブルエンジン。
// 台本化されたロール列は持たない。ポジショングラフ (局面ノード + 選択肢の next 辺) を、
// 回答結果 × 引き継ぎ状態 × フォーカス × SRS 優先度の重みで動的に歩く。
// 同じ開始局面でも相手初動と遷移が毎回変わり、ミスしても consequence から不利な局面へ続く。

import type { Choice, OpponentAction, Scenario, ScenarioId, StateFlag, Uniform } from "../content/types";
import { scenarioById, SCENARIOS } from "../content/scenarios";
import { itemKey, priorityOf, type SrsState } from "./srs";
import { mulberry32, shuffled, weightedPick, type Rng } from "./rng";

export type Focus = "mixed" | "defense" | "offense";
export type Difficulty = "beginner" | "live";

export interface RollConfig {
  focus: Focus;
  uniform: Uniform;
  difficulty: Difficulty;
  /** ロールの体力 = 判断回数。尽きたらロール終了 */
  maxSteps?: number;
  seed?: number;
}

export interface Step {
  index: number;
  scenario: Scenario;
  action: OpponentAction;
  /** 表示条件 (uniform / 初動 / 状態) でフィルタ済み・シャッフル済み */
  options: Choice[];
  /** 局面に入った時点の引き継ぎ状態 */
  statesAtEntry: StateFlag[];
  /** 実戦のみ判断制限時間。入門は null */
  timeLimitSec: number | null;
}

export interface Outcome {
  correct: boolean;
  timedOut: boolean;
  choice: Choice;
  /** 読めた線 (正解) / 見落とした線 (不正解・時間切れ) */
  readCues: string[];
  missedCues: string[];
  srsKey: string;
  nextId: ScenarioId | null;
  rollEnded: boolean;
}

export interface StepRecord {
  scenarioId: ScenarioId;
  positionJp: string;
  actionLabel: string;
  actionCue: string;
  chosenJp: string;
  correct: boolean;
  timedOut: boolean;
  reaction?: string;
  consequence?: string;
  principle: string;
  nextId: ScenarioId | null;
  srsKey: string;
}

const DEFAULT_MAX_STEPS = 6;

export function normalizeNext(choice: Choice): { id: ScenarioId; weight: number }[] {
  return choice.next.map((n) => (typeof n === "string" ? { id: n, weight: 1 } : n));
}

/** この局面で今表示される選択肢 (uniform / 相手初動 / 引き継ぎ状態で出し分け) */
export function visibleOptions(
  scenario: Scenario,
  actionId: string,
  uniform: Uniform,
  states: readonly StateFlag[],
): Choice[] {
  return scenario.options.filter((o) => {
    if (o.giOnly && uniform !== "gi") return false;
    if (o.nogiOnly && uniform !== "nogi") return false;
    if (o.requiresAction && !o.requiresAction.includes(actionId)) return false;
    if (o.forbiddenAction?.includes(actionId)) return false;
    if (o.requiresState && !o.requiresState.some((s) => states.includes(s))) return false;
    if (o.forbiddenState?.some((s) => states.includes(s))) return false;
    return true;
  });
}

/** 全 (局面 × 相手初動) の SRS キー — 稽古記録ダッシュボード用 */
export function allItemKeys(): { key: string; scenario: Scenario; action: OpponentAction }[] {
  return SCENARIOS.flatMap((scenario) =>
    scenario.opponentActions.map((action) => ({
      key: itemKey(scenario.id, action.id),
      scenario,
      action,
    })),
  );
}

export class RollEngine {
  private readonly config: Required<Pick<RollConfig, "focus" | "uniform" | "difficulty" | "maxSteps">>;
  private readonly rng: Rng;
  private readonly srs: SrsState;
  private readonly now: number;

  private states: StateFlag[] = [];
  readonly history: StepRecord[] = [];
  private currentStep: Step | null = null;
  private pendingNextId: ScenarioId | null = null;

  constructor(config: RollConfig, srs: SrsState, now: number) {
    this.config = {
      focus: config.focus,
      uniform: config.uniform,
      difficulty: config.difficulty,
      maxSteps: config.maxSteps ?? DEFAULT_MAX_STEPS,
    };
    this.rng = mulberry32(config.seed ?? Math.floor(Math.random() * 2 ** 31));
    this.srs = srs;
    this.now = now;
  }

  get step(): Step | null {
    return this.currentStep;
  }

  get statesNow(): readonly StateFlag[] {
    return this.states;
  }

  /** ロール開始。startId 指定で苦手局面からの再ロールにも使える */
  start(startId?: ScenarioId): Step {
    this.states = [];
    this.history.length = 0;
    const scenario = startId ? scenarioById(startId) : this.pickStart();
    this.currentStep = this.makeStep(scenario, 0);
    return this.currentStep;
  }

  private pickStart(): Scenario {
    const pool = SCENARIOS.filter(
      (s) => this.config.focus === "mixed" || s.role === this.config.focus,
    );
    const picked = weightedPick(this.rng, pool, (s) => 1 + this.scenarioSrsPriority(s));
    if (!picked) throw new Error("no start scenario");
    return picked;
  }

  /** 局面内の弱い初動 (未学習・期日超過) ほど出やすい */
  private makeStep(scenario: Scenario, index: number): Step {
    const action = weightedPick(
      this.rng,
      scenario.opponentActions,
      (a) => a.weight * (1 + priorityOf(this.srs, itemKey(scenario.id, a.id), this.now)),
    );
    if (!action) throw new Error(`scenario has no opponentActions: ${scenario.id}`);
    const options = shuffled(
      this.rng,
      visibleOptions(scenario, action.id, this.config.uniform, this.states),
    );
    return {
      index,
      scenario,
      action,
      options,
      statesAtEntry: [...this.states],
      timeLimitSec: this.config.difficulty === "live" ? scenario.timeLimitSec : null,
    };
  }

  private scenarioSrsPriority(scenario: Scenario): number {
    let max = 0;
    for (const a of scenario.opponentActions) {
      max = Math.max(max, priorityOf(this.srs, itemKey(scenario.id, a.id), this.now));
    }
    return max;
  }

  /** 回答する。index は step.options のインデックス */
  answer(index: number): Outcome {
    const step = this.mustStep();
    const choice = step.options[index];
    if (!choice) throw new Error(`invalid option index: ${index}`);
    return this.resolve(step, choice, false);
  }

  /** 実戦モードの時間切れ。「考えている間に極められる」— 悪手が自動で選ばれる */
  timeout(): Outcome {
    const step = this.mustStep();
    const bad = step.options.filter((o) => !o.correct);
    const choice = weightedPick(this.rng, bad.length > 0 ? bad : step.options, () => 1);
    if (!choice) throw new Error("no options to time out into");
    return this.resolve(step, choice, true);
  }

  private mustStep(): Step {
    if (!this.currentStep) throw new Error("roll not started");
    return this.currentStep;
  }

  private resolve(step: Step, choice: Choice, timedOut: boolean): Outcome {
    const correct = choice.correct && !timedOut;

    if (choice.stateEffects) {
      const remove = new Set(choice.stateEffects.remove ?? []);
      this.states = this.states.filter((s) => !remove.has(s));
      for (const add of choice.stateEffects.add ?? []) {
        if (!this.states.includes(add)) this.states.push(add);
      }
    }

    const rollEnded = this.history.length + 1 >= this.config.maxSteps;
    const nextId = rollEnded ? null : this.pickNext(step, choice);
    const srsKey = itemKey(step.scenario.id, step.action.id);

    this.history.push({
      scenarioId: step.scenario.id,
      positionJp: step.scenario.positionJp,
      actionLabel: step.action.label,
      actionCue: step.action.cue,
      chosenJp: choice.jp,
      correct,
      timedOut,
      reaction: choice.reaction,
      consequence: choice.consequence,
      principle: step.scenario.principle,
      nextId,
      srsKey,
    });
    this.pendingNextId = nextId;

    return {
      correct,
      timedOut,
      choice,
      readCues: correct ? step.action.readCues : [],
      missedCues: correct ? [] : step.action.readCues,
      srsKey,
      nextId,
      rollEnded,
    };
  }

  /**
   * 次局面の動的選択 (スクランブル):
   * 選ばれた手の next 候補を土台に、フォーカス一致・引き継ぎ状態との噛み合い・
   * SRS 弱点を掛け、直近と同じ局面は出にくくする。
   */
  private pickNext(step: Step, choice: Choice): ScenarioId {
    const recent = new Set(this.history.slice(-2).map((h) => h.scenarioId));
    const candidates = normalizeNext(choice);
    const picked = weightedPick(this.rng, candidates, ({ id, weight }) => {
      const target = scenarioById(id);
      let w = weight;
      if (this.config.focus !== "mixed" && target.role === this.config.focus) w *= 2;
      if (target.stateBias.some((s) => this.states.includes(s))) w *= 1.5;
      w *= 1 + 0.8 * this.scenarioSrsPriority(target);
      if (id === step.scenario.id) w *= 0.3;
      else if (recent.has(id)) w *= 0.5;
      return w;
    });
    return picked?.id ?? step.scenario.id;
  }

  /** 直前の回答で決まった次局面へ進む。ロール終了なら null */
  advance(): Step | null {
    if (this.pendingNextId === null) {
      this.currentStep = null;
      return null;
    }
    const next = scenarioById(this.pendingNextId);
    this.pendingNextId = null;
    this.currentStep = this.makeStep(next, this.history.length);
    return this.currentStep;
  }
}

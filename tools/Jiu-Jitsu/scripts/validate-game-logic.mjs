import { Game } from "../js/game.js";
import { choiceIndexForKey } from "../js/ui.js";

global.window = { location: { search: "" } };

const noop = () => {};
const dojo = {
  setAutoRotate: noop,
  setPoses: noop,
  setUniformMode: noop,
};
const ui = {
  bindControls: noop,
  setControlsUI: noop,
  renderBadge: noop,
  renderScore: noop,
  renderLesson: noop,
  renderPressure: noop,
  renderFeedback: noop,
  renderComplete: noop,
};

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function ids(scenarios) {
  return scenarios.map((scenario) => scenario.id);
}

function assertUniqueRun(game, label) {
  for (let i = 0; i < 200; i += 1) {
    const runIds = ids(game._buildRunScenarios());
    assert(new Set(runIds).size === runIds.length, `${label}: duplicate run ${runIds.join(",")}`);
  }
}

const game = new Game(dojo, ui);
let rejectedInvalidPosePair = false;
try {
  game._setPosePair("standingRed", "redMountTop", "invalid-test");
} catch {
  rejectedInvalidPosePair = true;
}
assert(rejectedInvalidPosePair, "game should reject pose pairs outside the BJJ position catalog");

assert(choiceIndexForKey({ key: "1", code: "Digit1" }, 4) === 0, "Digit 1 should choose first option");
assert(choiceIndexForKey({ key: "4", code: "Digit4" }, 4) === 3, "Digit 4 should choose fourth option");
assert(choiceIndexForKey({ key: "Unidentified", code: "Numpad2" }, 4) === 1, "Numpad 2 should choose second option");
assert(choiceIndexForKey({ key: "5", code: "Digit5" }, 4) === -1, "Out-of-range key should be ignored");
assert(choiceIndexForKey({ key: "0", code: "Digit0" }, 4) === -1, "Zero key should be ignored");

game.answered = false;
game.decisionOpen = false;
game._answer(0);
assert(!game.answered, "answer should be ignored before opponent action opens the decision");

const renderStates = [];
const renderUi = {
  ...ui,
  renderLesson: (scenario) => renderStates.push(scenario),
};
const actionRenderGame = new Game(dojo, renderUi);
actionRenderGame.runScenarios = [
  {
    id: "render-action",
    role: "defense",
    belt: "白帯",
    positionJp: "初動表示確認",
    positionEn: "Render Action",
    term: "test",
    situation: "test",
    prompt: "test",
    setup: { red: "standingRed", blue: "standingBlue", badge: "setup" },
    attack: { red: "standingRed", blue: "standingBlue", badge: "attack" },
    pressure: { early: "base early text", urgent: "base urgent text" },
    readCues: ["base", "cue"],
    opponentActions: [
      {
        id: "variant",
        label: "variant action",
        cue: "variant cue should be saved",
        attack: { red: "standingBlue", blue: "standingRed", badge: "variant attack" },
        pressure: { early: "variant early text", urgent: "variant urgent text" },
        readCues: ["variant", "cue"],
      },
    ],
    options: [
      {
        jp: "correct",
        en: "correct",
        correct: true,
        next: [],
        reaction: "reaction text",
        feedback: "feedback text",
        result: { red: "standingRed", blue: "standingBlue", badge: "result" },
      },
    ],
    principle: "principle",
  },
];
actionRenderGame.currentAction = actionRenderGame.runScenarios[0].opponentActions[0];
actionRenderGame.decisionOpen = false;
actionRenderGame._renderCurrent({ revealAction: false });
assert(!renderStates.at(-1).opponentAction, "setup render should not reveal opponent action");
assert(renderStates.at(-1).readCues[0] === "base", "setup render should keep base read cues");
actionRenderGame.decisionOpen = true;
actionRenderGame._renderCurrent({ reshuffle: false, revealAction: true });
assert(renderStates.at(-1).opponentAction.label === "variant action", "attack render should reveal opponent action");
assert(renderStates.at(-1).opponentAction.cue === "variant cue should be saved", "attack render should expose action cue");
assert(renderStates.at(-1).attack.badge === "variant attack", "attack render should switch action pose");
assert(renderStates.at(-1).readCues[0] === "variant", "attack render should switch read cues");

game.opponentStyle = { id: "guard-player", preferred: [] };
game.tactic = { id: "submission-chain", preferred: [], roleBias: {} };
const activeActionWeight = game._actionWeight({
  id: "angle",
  label: "angle",
  styles: ["guard-player"],
  tactics: ["submission-chain"],
  weight: 1,
});
const neutralActionWeight = game._actionWeight({ id: "base", label: "base", weight: 1 });
assert(activeActionWeight > neutralActionWeight, "opponent action should be biased by style and tactic");
const materialized = game._materializeScenario(
  {
    id: "action-check",
    attack: { red: "standingRed", blue: "standingBlue" },
    pressure: { early: "base early text", urgent: "base urgent text" },
    readCues: ["base", "cue"],
  },
  {
    id: "variant",
    label: "variant action",
    cue: "variant cue should be saved",
    attack: { red: "standingBlue", blue: "standingRed", badge: "variant attack" },
    pressure: { early: "variant early text", urgent: "variant urgent text" },
    readCues: ["variant", "cue"],
  },
);
assert(materialized.opponentAction.label === "variant action", "materialized action label missing");
assert(materialized.opponentAction.cue === "variant cue should be saved", "materialized action cue missing");
assert(materialized.attack.badge === "variant attack", "materialized action attack missing");
assert(materialized.pressure.early.includes("variant"), "materialized action pressure missing");
game.tactic = null;

const pressureRenders = [];
const noLimitGame = new Game(dojo, { ...ui, renderPressure: (seconds, text) => pressureRenders.push({ seconds, text }) });
noLimitGame.difficultyMode = "beginner";
noLimitGame._startPressure({
  id: "no-limit",
  role: "defense",
  pressure: { early: "patient pressure", urgent: "urgent pressure" },
});
assert(noLimitGame.decisionLimit === 0, "beginner mode should not set a decision time limit");
assert(pressureRenders.at(-1).seconds === null, "beginner mode should render no-limit pressure state");
assert(pressureRenders.at(-1).text === "patient pressure", "beginner mode should keep pressure cue without countdown");
assert(
  noLimitGame._tempoForAnswer({ correct: true }).id === "none",
  "beginner no-limit answers should not receive tempo bonus",
);

const stateGame = new Game(dojo, ui);
stateGame.uniformMode = "gi";
stateGame.difficultyMode = "live";
const mountEscape = stateGame._allScenarios().find((scenario) => scenario.id === "mount-escape");
const backDefense = stateGame._allScenarios().find((scenario) => scenario.id === "back-defense");
const mountAttack = stateGame._allScenarios().find((scenario) => scenario.id === "attack-from-mount");
const backAttack = stateGame._allScenarios().find((scenario) => scenario.id === "attack-from-back");
const sideAttack = stateGame._allScenarios().find((scenario) => scenario.id === "attack-from-side");
const closedGuardPosture = stateGame._allScenarios().find((scenario) => scenario.id === "closed-guard-posture");
stateGame.rollState = new Set();
for (const mode of ["gi", "nogi"]) {
  stateGame.uniformMode = mode;
  const chokeEntryOptions = stateGame._visibleOptions({
    ...backDefense,
    opponentAction: { id: "choke-hand-entry" },
  });
  assert(
    chokeEntryOptions.filter((option) => option.correct).length === 1 &&
      chokeEntryOptions.some((option) => option.jp.includes(mode === "gi" ? "首と襟" : "手首と前腕")),
    `choke-hand-entry back action should keep ${mode} neck defense as the single correct option`,
  );
}
stateGame.uniformMode = "gi";
const hookRideOptions = stateGame._visibleOptions({
  ...backDefense,
  opponentAction: { id: "hook-ride" },
});
assert(
  hookRideOptions.filter((option) => option.correct).length === 1 &&
    hookRideOptions.some((option) => option.requiresAction?.includes("hook-ride")),
  "hook-ride back action should expose hook-clear escape as the single correct option",
);
const seatbeltOptions = stateGame._visibleOptions({
  ...backDefense,
  opponentAction: { id: "seatbelt-tighten" },
});
assert(
  seatbeltOptions.filter((option) => option.correct).length === 1 &&
    seatbeltOptions.some((option) => option.requiresAction?.includes("seatbelt-tighten")),
  "seatbelt-tighten back action should expose shoulder-to-mat escape as the single correct option",
);
const elbowHideMountAttackOptions = stateGame._visibleOptions({
  ...mountAttack,
  opponentAction: { id: "elbow-hide" },
});
assert(
  elbowHideMountAttackOptions.filter((option) => option.correct).length === 1 &&
    elbowHideMountAttackOptions.some((option) => option.jp.includes("胸で圧をかけて")),
  "elbow-hide mount attack should keep elbow isolation as the single correct option",
);
const bridgeThreatMountAttackOptions = stateGame._visibleOptions({
  ...mountAttack,
  opponentAction: { id: "bridge-threat" },
});
assert(
  bridgeThreatMountAttackOptions.filter((option) => option.correct).length === 1 &&
    bridgeThreatMountAttackOptions.some((option) => option.requiresAction?.includes("bridge-threat")),
  "bridge-threat mount attack should expose base recovery as the single correct option",
);
const handFightBackAttackOptions = stateGame._visibleOptions({
  ...backAttack,
  opponentAction: { id: "hand-fight" },
});
assert(
  handFightBackAttackOptions.filter((option) => option.correct).length === 1 &&
    handFightBackAttackOptions.some((option) => option.jp.includes("防御手を剥がして")),
  "hand-fight back attack should keep hand stripping as the single correct option",
);
const hipSlideBackAttackOptions = stateGame._visibleOptions({
  ...backAttack,
  opponentAction: { id: "hip-slide" },
});
assert(
  hipSlideBackAttackOptions.filter((option) => option.correct).length === 1 &&
    hipSlideBackAttackOptions.some((option) => option.requiresAction?.includes("hip-slide")),
  "hip-slide back attack should expose mount transition as the single correct option",
);
const armIsolationOptions = stateGame._visibleOptions({
  ...mountEscape,
  opponentAction: { id: "arm-isolation" },
});
assert(
  armIsolationOptions.filter((option) => option.correct).length === 1 &&
    armIsolationOptions.some((option) => option.jp.includes("アッパ")),
  "arm-isolation mount action should keep upa as the single correct option",
);
const highMountOptions = stateGame._visibleOptions({
  ...mountEscape,
  opponentAction: { id: "high-mount-climb" },
});
assert(
  highMountOptions.filter((option) => option.correct).length === 1 &&
    highMountOptions.some((option) => option.requiresAction?.includes("high-mount-climb")),
  "high-mount action should expose knee-elbow escape as the single correct option",
);
const grapevineOptions = stateGame._visibleOptions({
  ...mountEscape,
  opponentAction: { id: "grapevine-base" },
});
assert(
  grapevineOptions.filter((option) => option.correct).length === 1 &&
    grapevineOptions.some((option) => option.requiresAction?.includes("grapevine-base")),
  "grapevine action should expose clear-hooks escape as the single correct option",
);
const baseSideOptions = stateGame._visibleOptions(sideAttack);
assert(
  baseSideOptions.some((option) => option.jp.includes("フレームを潰して腰を制し")),
  "base side attack should show normal climb option",
);
assert(
  !baseSideOptions.some((option) => option.requiresState?.includes("guard-recovered")),
  "state-gated side attack option should be hidden without state",
);
const frameRecoveryOptions = stateGame._visibleOptions({
  ...sideAttack,
  opponentAction: { id: "frame-recovery" },
});
assert(
  frameRecoveryOptions.filter((option) => option.correct).length === 1 &&
    frameRecoveryOptions.some((option) => option.jp.includes("フレームを潰して腰を制し")),
  "frame recovery action should keep mount climb as the single correct option",
);
const turnAwayOptions = stateGame._visibleOptions({
  ...sideAttack,
  opponentAction: { id: "turn-away" },
});
assert(
  turnAwayOptions.filter((option) => option.correct).length === 1 &&
    turnAwayOptions.some((option) => option.requiresAction?.includes("turn-away")),
  "turn-away action should expose back-take as the single correct option",
);
const kneeShieldOptions = stateGame._visibleOptions({
  ...sideAttack,
  opponentAction: { id: "knee-shield-insert" },
});
assert(
  kneeShieldOptions.filter((option) => option.correct).length === 1 &&
    kneeShieldOptions.some((option) => option.requiresAction?.includes("knee-shield-insert")),
  "knee-shield action should expose knee-smash as the single correct option",
);
const postureBreakOptions = stateGame._visibleOptions({
  ...closedGuardPosture,
  opponentAction: { id: "posture-break" },
});
assert(
  postureBreakOptions.filter((option) => option.correct).length === 1 &&
    postureBreakOptions.some((option) => option.jp.includes("背筋を立て")),
  "posture-break action should keep posture recovery as the single correct option",
);
const angleCutOptions = stateGame._visibleOptions({
  ...closedGuardPosture,
  opponentAction: { id: "angle-cut" },
});
assert(
  angleCutOptions.filter((option) => option.correct).length === 1 &&
    angleCutOptions.some((option) => option.requiresAction?.includes("angle-cut")),
  "angle-cut action should expose hip-square defense as the single correct option",
);
const hipBumpOptions = stateGame._visibleOptions({
  ...closedGuardPosture,
  opponentAction: { id: "hip-bump-threat" },
});
assert(
  hipBumpOptions.filter((option) => option.correct).length === 1 &&
    hipBumpOptions.some((option) => option.requiresAction?.includes("hip-bump-threat")),
  "hip-bump action should expose no-post hip-control defense as the single correct option",
);
stateGame.rollState = new Set(["guard-recovered"]);
const recoveredSideOptions = stateGame._visibleOptions(sideAttack);
assert(
  recoveredSideOptions.some((option) => option.requiresState?.includes("guard-recovered")),
  "guard-recovered state should show re-smash option",
);
assert(
  !recoveredSideOptions.some((option) => option.forbiddenState?.includes("guard-recovered")),
  "guard-recovered state should hide forbidden base option",
);

const stateAnswerGame = new Game(dojo, ui);
stateAnswerGame.runScenarios = [
  {
    id: "state-answer",
    role: "defense",
    positionJp: "状態確認",
    prompt: "state?",
    readCues: ["腰", "肘"],
    principle: "state",
  },
];
stateAnswerGame.view = {
  ...stateAnswerGame.runScenarios[0],
  options: [
    {
      jp: "state correct",
      en: "state correct",
      correct: true,
      next: [],
      reaction: "state branch",
      stateEffects: { add: ["guard-recovered"], remove: ["frame-lost"] },
      feedback: "state feedback",
      result: { red: "standingRed", blue: "standingBlue", badge: "state" },
    },
  ],
};
stateAnswerGame.index = 0;
stateAnswerGame.decisionOpen = true;
stateAnswerGame.answered = false;
stateAnswerGame._answer(0);
assert(stateAnswerGame.rollState.has("guard-recovered"), "answer should apply roll state effects");
assert(stateAnswerGame.history[0].rollState.includes("ガード回復"), "history should keep roll state labels");
assert(
  stateAnswerGame.history[0].stateEffects.added.includes("ガード回復"),
  "history should keep state effect labels",
);

const stateBiasGame = new Game(dojo, ui);
stateBiasGame.rollMode = "mixed";
stateBiasGame.flow = 0;
stateBiasGame.tactic = null;
stateBiasGame.rollState = new Set(["guard-recovered"]);
const stateBiasedWeight = stateBiasGame._nextCandidateWeight(
  { id: "attack-from-side", weight: 1 },
  { id: "attack-from-side", role: "offense", stateBias: ["guard-recovered"] },
  new Set(),
);
const stateNeutralWeight = stateBiasGame._nextCandidateWeight(
  { id: "attack-from-mount", weight: 1 },
  { id: "attack-from-mount", role: "offense", stateBias: ["top-base"] },
  new Set(),
);
assert(stateBiasedWeight > stateNeutralWeight, "rollState should bias matching next scenarios");
assert(
  stateBiasGame._nextReasonFor(
    { id: "attack-from-side", role: "offense", stateBias: ["guard-recovered"] },
    true,
  ).includes("ガード回復"),
  "next reason should mention matching roll state",
);
const stateBiasScenarios = new Map([
  ["attack-from-side", { id: "attack-from-side", role: "offense", stateBias: ["guard-recovered"] }],
  ["attack-from-mount", { id: "attack-from-mount", role: "offense", stateBias: ["top-base"] }],
]);
const timeoutStateBiasedWeight = stateBiasGame._timeoutOptionWeight(
  { correct: false, next: [{ id: "attack-from-side", weight: 1 }] },
  new Set(),
  stateBiasScenarios,
);
const timeoutStateNeutralWeight = stateBiasGame._timeoutOptionWeight(
  { correct: false, next: [{ id: "attack-from-mount", weight: 1 }] },
  new Set(),
  stateBiasScenarios,
);
assert(timeoutStateBiasedWeight > timeoutStateNeutralWeight, "timeout should respect rollState bias");

const tempoGame = new Game(dojo, ui);
tempoGame.runScenarios = [
  {
    id: "tempo-check",
    role: "defense",
    positionJp: "テンポ確認",
  prompt: "tempo?",
  readCues: ["姿勢", "肘"],
  opponentAction: { id: "tempo-action", label: "tempo action", cue: "tempo action cue" },
  principle: "tempo",
  },
];
tempoGame.view = {
  ...tempoGame.runScenarios[0],
  options: [
    {
      jp: "quick correct",
      en: "quick correct",
      correct: true,
      next: [],
      reaction: "tempo branch",
      feedback: "tempo feedback",
      result: { red: "standingRed", blue: "standingBlue", badge: "tempo" },
    },
  ],
};
tempoGame.index = 0;
tempoGame.decisionOpen = true;
tempoGame.answered = false;
tempoGame.decisionLimit = 10;
tempoGame.secondsLeft = 8;
tempoGame._answer(0);
assert(tempoGame.score === 12, `quick correct should add tempo bonus: ${tempoGame.score}`);
assert(tempoGame.runStats.quickCorrect === 1, "quick correct should be tracked");
assert(tempoGame.runStats.tempoBonus === 2, "tempo bonus should be tracked");
assert(tempoGame.history[0].readCues[0] === "姿勢", "answer history should keep read cues");
assert(tempoGame.history[0].opponentAction.cue === "tempo action cue", "answer history should keep action cue");

const drillGame = new Game(dojo, ui);
drillGame.rollMode = "mixed";
drillGame.opponentStyle = { preferred: [] };
drillGame.tactic = null;
drillGame.drillFocus = { scenarioId: "closed-guard-posture", actionId: "angle-cut" };
const drillRun = drillGame._buildRunScenarios();
assert(drillRun[0]?.id === "closed-guard-posture", "drill focus scenario should be first");
const drillAction = drillGame._selectOpponentAction(drillRun[0]);
assert(drillAction.id === "angle-cut", `drill focus action failed: ${drillAction.id}`);
drillGame.history = [
  {
    id: "back-defense",
    positionJp: "バックコントロール",
    correct: false,
    opponentAction: { id: "hook-ride", label: "腰のフックで追う" },
  },
];
const weakness = drillGame._weaknessFocus();
assert(weakness.scenarioId === "back-defense", "weakness focus scenario missing");
assert(weakness.actionId === "hook-ride", "weakness focus action missing");

const adaptiveGame = new Game(dojo, ui);
adaptiveGame.rollMode = "defense";
adaptiveGame.history = [
  {
    id: "mount-escape",
    positionJp: "マウント",
    correct: false,
    readCues: ["腰", "肘"],
  },
];
adaptiveGame.adaptiveFocus = adaptiveGame._adaptiveFocusFromHistory();
assert(adaptiveGame.adaptiveFocus?.cue === "腰", "adaptive focus should capture missed read cue");
const adaptiveOrdered = adaptiveGame._shuffleForOpponentStyle([
  { id: "back-defense", readCues: ["首"], opponentActions: [] },
  { id: "closed-guard-posture", readCues: ["姿勢"], opponentActions: [{ readCues: ["腰"] }] },
  { id: "side-escape", readCues: ["首フレーム"], opponentActions: [] },
]);
assert(adaptiveOrdered[0]?.id === "closed-guard-posture", "adaptive focus should bias matching scenarios first");
adaptiveGame.start({ drillFocus: { scenarioId: "back-defense", actionId: "hook-ride" } });
assert(!adaptiveGame.adaptiveFocus, "drill focus should disable adaptive focus");

for (const mode of ["mixed", "defense", "offense"]) {
  game.rollMode = mode;
  game.opponentStyle = { preferred: [] };
  assertUniqueRun(game, mode);
}

game.rollMode = "mixed";
game.index = 0;
game.history = [{ id: "back-defense" }];
game.runScenarios = [
  { id: "back-defense" },
  { id: "mount-escape" },
  { id: "attack-from-back" },
];
game.pendingNextIds = [
  { id: "attack-from-back", weight: 10 },
  { id: "attack-from-side", weight: 1 },
];
game.opponentStyle = { preferred: ["attack-from-back"] };
const next = game._selectNextScenario();
assert(next?.id === "attack-from-side", `future duplicate was not avoided: ${next?.id}`);

game.pendingNextIds = ["attack-from-side"];
const plain = game._selectNextScenario();
assert(plain?.id === "attack-from-side", `plain string next failed: ${plain?.id}`);

game.rollMode = "mixed";
game.flow = 1;
const offenseWeight = game._nextCandidateWeight(
  { id: "attack-from-side", weight: 1 },
  { id: "attack-from-side", role: "offense" },
  new Set(),
);
const defenseWeightWhileAhead = game._nextCandidateWeight(
  { id: "side-escape", weight: 1 },
  { id: "side-escape", role: "defense" },
  new Set(),
);
assert(offenseWeight > defenseWeightWhileAhead, "positive flow should favor offense next");

game.flow = -1;
const defenseWeight = game._nextCandidateWeight(
  { id: "side-escape", weight: 1 },
  { id: "side-escape", role: "defense" },
  new Set(),
);
const offenseWeightWhileBehind = game._nextCandidateWeight(
  { id: "attack-from-side", weight: 1 },
  { id: "attack-from-side", role: "offense" },
  new Set(),
);
assert(defenseWeight > offenseWeightWhileBehind, "negative flow should favor defense next");

game.tactic = { preferred: ["attack-from-side"], roleBias: { offense: 1.2 }, timeDelta: -1 };
game.flow = 0;
game.opponentStyle = { preferred: [] };
const tacticOrdered = game._shuffleForOpponentStyle([
  { id: "side-escape", role: "defense" },
  { id: "attack-from-side", role: "offense" },
]);
assert(tacticOrdered[0]?.id === "attack-from-side", "tactic should bias initial run order");
const tacticPreferredWeight = game._nextCandidateWeight(
  { id: "attack-from-side", weight: 1 },
  { id: "attack-from-side", role: "offense" },
  new Set(),
);
game.tactic = null;
const tacticNeutralWeight = game._nextCandidateWeight(
  { id: "attack-from-side", weight: 1 },
  { id: "attack-from-side", role: "offense" },
  new Set(),
);
assert(tacticPreferredWeight > tacticNeutralWeight, "tactic should bias preferred next scenarios");

game.difficultyMode = "beginner";
const normalTime = game._timeLimit({ timeLimitSec: 8 });
game.tactic = { timeDelta: -1 };
const fastTime = game._timeLimit({ timeLimitSec: 8 });
assert(fastTime === normalTime - 1, "tactic timeDelta should affect decision time");
game.tactic = null;

game.flow = 0;
const scenariosById = new Map(game._allScenarios().map((scenario) => [scenario.id, scenario]));
const timeoutPreferredWeight = game._timeoutOptionWeight(
  { correct: false, next: [{ id: "attack-triangle-guard", weight: 1 }] },
  new Set(["attack-triangle-guard"]),
  scenariosById,
);
const timeoutNeutralWeight = game._timeoutOptionWeight(
  { correct: false, next: [{ id: "attack-from-side", weight: 1 }] },
  new Set(["attack-triangle-guard"]),
  scenariosById,
);
assert(timeoutPreferredWeight > timeoutNeutralWeight, "timeout should favor opponent-style consequences");

game.rollMode = "mixed";
game.flow = -1;
const timeoutDefenseWeight = game._timeoutOptionWeight(
  { correct: false, next: [{ id: "side-escape", weight: 1 }] },
  new Set(),
  scenariosById,
);
const timeoutOffenseWeight = game._timeoutOptionWeight(
  { correct: false, next: [{ id: "attack-from-side", weight: 1 }] },
  new Set(),
  scenariosById,
);
assert(timeoutDefenseWeight > timeoutOffenseWeight, "timeout should respect negative flow pressure");

game.runScenarios = [{ id: "a" }, { id: "b" }, { id: "c" }];
game.runStats = { correct: 2, timedOut: 0, offenseCorrect: 1, defenseCorrect: 1, maxStreak: 2 };
game.flow = 1;
assert(
  game._missionAchieved({ target: { maxTimedOut: 0, minFinalFlow: 0 } }),
  "safe mission should be achieved",
);
assert(
  game._missionAchieved({ target: { minOffenseCorrect: 1 } }),
  "offense mission should be achieved",
);
assert(
  !game._missionAchieved({ target: { minDefenseCorrect: 2 } }),
  "defense mission should not be achieved",
);

game.rollMode = "mixed";
game.runScenarios = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
game.history = [];
game.runStats = { correct: 2, timedOut: 1, offenseCorrect: 1, defenseCorrect: 1, maxStreak: 1 };
game.flow = -1;
assert(
  game._coachReview().headline.includes("先手"),
  "coach review should prioritize timeout pressure",
);

game.runStats = { correct: 2, timedOut: 0, offenseCorrect: 0, defenseCorrect: 2, maxStreak: 2 };
game.flow = 0;
assert(
  game._coachReview().headline.includes("守りから攻め"),
  "coach review should catch missing offense connection",
);

game.runStats = { correct: 4, timedOut: 0, offenseCorrect: 2, defenseCorrect: 2, maxStreak: 3 };
game.flow = 1;
assert(
  game._coachReview().headline.includes("主導権"),
  "coach review should reward strong rolls",
);

console.log(
  JSON.stringify(
    {
      runSamples: 600,
      numberKeys: true,
      posePairGuard: true,
      decisionLock: true,
      opponentActionReveal: true,
      opponentActionAttack: true,
      opponentActionCue: true,
      opponentActions: true,
      noLimitBeginner: true,
      actionGatedChoices: true,
      rollState: true,
      stateBias: true,
      tempoBonus: true,
      readCueFeedback: true,
      weaknessDrill: true,
      adaptiveFocus: true,
      futureDuplicateAvoided: true,
      plainNext: true,
      flowBias: true,
      tacticBias: true,
      tacticInitialBias: true,
      timeoutChase: true,
      missions: true,
      coachReview: true,
    },
    null,
    2,
  ),
);

import { PRACTICE_LESSONS, practiceLesson } from "./content/practice";
import { practicePair } from "./render/practicePose";
import { BodyScene, type PracticeCamera } from "./render/bodyScene";
import { h } from "./ui/dom";
import { SCENARIOS } from "./content/scenarios";
import { rollPair } from "./render/rollPose";

const params = new URLSearchParams(location.search);
const roll = SCENARIOS.find((item) => item.id === params.get("roll"));
if (params.has("roll") && !roll) throw new Error("Unknown roll scenario");
const stages = (scenario: typeof SCENARIOS[number]) => [
  { label: "開始", stage: scenario.setup }, { label: "基本の攻防", stage: scenario.attack },
  ...scenario.opponentActions.map((action) => ({ label: `初動: ${action.label}`, stage: action.attack })),
  ...scenario.options.map((option) => ({ label: `回答後: ${option.jp}`, stage: option.result })),
];
const stageIndex = Number(params.get("stage") ?? 0);
const rollStage = roll ? stages(roll)[stageIndex] : undefined;
if (roll && !rollStage) throw new Error("Unknown roll stage");
const lessonId = params.get("lesson") ?? PRACTICE_LESSONS[0]!.id;
const lesson = practiceLesson(lessonId);
const nodeId = params.get("node") ?? lesson.starts[0];
const node = lesson.nodes[nodeId];
if (!node) throw new Error(`Unknown audit node: ${nodeId}`);
const view = params.get("view") ?? "diagonal";
if (!["diagonal", "side", "top"].includes(view)) throw new Error(`Unknown view: ${view}`);
const scene = new BodyScene(document.querySelector<HTMLCanvasElement>("#canvas")!, document.querySelector<HTMLElement>("#pins")!);
scene.show(rollStage ? rollPair(rollStage.stage) : practicePair(lessonId, nodeId));
scene.setCamera(view as PracticeCamera);
scene.setDisplay(params.has("bones"), params.has("fade"), true, roll?.role === "offense" ? "blue" : "red");
scene.setActive(true);

const toolbar = document.querySelector<HTMLElement>("#toolbar")!;
const select = h("select", { attrs: { "aria-label": "検証する場面" } });
for (const item of PRACTICE_LESSONS) {
  const group = h("optgroup", { attrs: { label: item.title } });
  for (const [id, state] of Object.entries(item.nodes)) {
    const option = h("option", { text: `${id} — ${state.title}`, attrs: { value: `${item.id}/${id}` } });
    option.selected = !roll && item.id === lessonId && id === nodeId;
    group.append(option);
  }
  select.append(group);
}
for (const scenario of SCENARIOS) {
  const group = h("optgroup", { attrs: { label: `応用ロール: ${scenario.positionJp}` } });
  stages(scenario).forEach((item, index) => {
    const option = h("option", { text: item.label, attrs: { value: `roll/${scenario.id}/${index}` } });
    option.selected = roll?.id === scenario.id && stageIndex === index;
    group.append(option);
  });
  select.append(group);
}
select.addEventListener("change", () => {
  const [lesson, node, stage] = select.value.split("/");
  if (lesson === "roll") { params.set("roll", node!); params.set("stage", stage!); }
  else { params.delete("roll"); params.delete("stage"); params.set("lesson", lesson!); params.set("node", node!); }
  location.search = params.toString();
});
toolbar.append(select);
for (const [label, camera] of [["斜め", "diagonal"], ["横", "side"], ["真上", "top"]] as const) toolbar.append(h("button", { text: label, onClick: () => scene.setCamera(camera) }));
toolbar.append(h("button", { text: "骨格", onClick: () => scene.setDisplay(true, false, true) }), h("button", { text: "身体", onClick: () => scene.setDisplay(false, false, true) }), h("button", { text: "相手を透かす", onClick: () => scene.setDisplay(false, true, true) }));
if (rollStage) document.querySelector("#caption")!.innerHTML = `${roll!.id} / ${stageIndex} — ${rollStage.label} ｜ ${rollStage.stage.badge}`;
else document.querySelector("#caption")!.textContent = `${lessonId} / ${nodeId} — ${node.title} ｜ ${node.facts.join(" · ")}`;
window.addEventListener("pagehide", () => scene.dispose(), { once: true });

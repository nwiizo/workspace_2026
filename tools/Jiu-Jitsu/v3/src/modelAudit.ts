import * as T from "three";
import { Mannequin } from "./render/mannequin";
import { neutralModelPose } from "./render/modelStudy";
import { practicePair } from "./render/practicePose";
import { rollPair } from "./render/rollPose";
import { h } from "./ui/dom";

document.querySelector<HTMLAnchorElement>("#reference")!.href = new URL("../docs/model-production/assets/reference-three-view.png", import.meta.url).href;

const poses = [
  ["基準立位", neutralModelPose],
  ["マウントの上", () => practicePair("mount-base", "low").red],
  ["ハーフガードの下", () => practicePair("half-bottom", "shield").blue],
  ["タートルの下", () => practicePair("turtle-bottom", "compact").blue],
  ["腕十字の保持側", () => rollPair({ red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "" }).red],
] as const;
const poseSelect = h("select", { attrs: { "aria-label": "形状を確認する姿勢" } }, ...poses.map(([name], i) => h("option", { text: name, attrs: { value: String(i) } })));
const bones = h("input", { attrs: { type: "checkbox", "aria-label": "骨格" } });
const wire = h("input", { attrs: { type: "checkbox", "aria-label": "ワイヤーフレーム" } });
const oblique = h("input", { attrs: { type: "checkbox", "aria-label": "背面を斜めから" } });
document.querySelector("#toolbar")!.append(poseSelect, h("label", {}, bones, "骨格"), h("label", {}, wire, "ワイヤーフレーム"), h("label", {}, oblique, "背面を斜めから"));
const views = ([
  ["正面", new T.Vector3(0, 0, 4)], ["右側面", new T.Vector3(-4, 0, 0)], ["背面", new T.Vector3(0, 0, -4)],
] as const).map(([name, direction]) => {
  const canvas = h("canvas", { attrs: { "aria-label": name } });
  const caption = h("figcaption", { text: name });
  document.querySelector("#views")!.append(h("figure", {}, canvas, caption));
  const renderer = new T.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  const scene = new T.Scene(); scene.background = new T.Color(0xe6e8eb);
  scene.add(new T.HemisphereLight(0xffffff, 0x7a8493, 2.5));
  const light = new T.DirectionalLight(0xffffff, 2.2); light.position.copy(direction).add(new T.Vector3(-1.5, 3, 1)); scene.add(light);
  const model = new Mannequin(neutralModelPose(), 0x647f9d); scene.add(model.root);
  const camera = new T.OrthographicCamera(-0.7, 0.7, 0.9, -0.9, 0.1, 20);
  camera.position.copy(direction).add(new T.Vector3(0, 0.81, 0)); camera.lookAt(0, 0.81, 0);
  return { canvas, renderer, scene, model, camera, direction, caption, name };
});
function draw(): void {
  const pose = poses[Number(poseSelect.value)]![1]();
  for (const view of views) {
    view.model.updatePose(pose); view.model.setDisplay(bones.checked, false);
    view.model.root.traverse((obj) => { if (obj instanceof T.Mesh) {
      const materials = Array.isArray(obj.material) ? obj.material : [obj.material];
      for (const material of materials) if (material instanceof T.MeshStandardMaterial) material.wireframe = wire.checked;
    } });
    const width = view.canvas.clientWidth, height = view.canvas.clientHeight;
    view.renderer.setSize(width, height, false);
    const bounds = new T.Box3().setFromObject(view.model.root, true), size = bounds.getSize(new T.Vector3());
    const standing = poseSelect.value === "0";
    view.caption.textContent = standing ? (view === views[2] && oblique.checked ? "斜め後ろ" : view.name)
      : view === views[0] ? "マットの +Z 側" : view === views[1] ? "マットの −X 側" : oblique.checked ? "マットの斜め後ろ側" : "マットの −Z 側";
    const center = standing ? new T.Vector3(0, 0.81, 0) : bounds.getCenter(new T.Vector3());
    const direction = view === views[2] && oblique.checked ? new T.Vector3(2.5, 0, -4) : view.direction;
    view.camera.position.copy(center).add(direction); view.camera.lookAt(center);
    const halfHeight = standing ? Math.max(0.9, 0.56 * height / width) : Math.max(size.y * 0.60, Math.max(size.x, size.z) * 0.60 * height / width);
    view.camera.top = halfHeight; view.camera.bottom = -halfHeight;
    view.camera.left = -halfHeight * width / height; view.camera.right = halfHeight * width / height;
    view.camera.updateProjectionMatrix(); view.renderer.render(view.scene, view.camera);
  }
  const geometries = new Set<T.BufferGeometry>(); let triangles = 0, meshes = 0;
  views[0]!.model.root.traverse((obj) => { if (obj instanceof T.Mesh) {
    geometries.add(obj.geometry); meshes++; triangles += (obj.geometry.index?.count ?? obj.geometry.getAttribute("position").count) / 3;
  } });
  const stats = { triangles, geometries: geometries.size, meshes, drawCalls: views[0]!.renderer.info.render.calls, drawnTriangles: views[0]!.renderer.info.render.triangles };
  document.querySelector("#stats")!.textContent = `全形状 ${triangles.toLocaleString()} 三角形 / geometry ${geometries.size} / mesh ${meshes} ｜この表示 ${stats.drawCalls} 描画呼び出し・${stats.drawnTriangles.toLocaleString()} 三角形`;
  document.querySelector("#stats")!.setAttribute("data-metrics", JSON.stringify(stats));
}
for (const input of [poseSelect, bones, wire, oblique]) input.addEventListener("change", draw);
new ResizeObserver(draw).observe(document.querySelector("#views")!);
draw();

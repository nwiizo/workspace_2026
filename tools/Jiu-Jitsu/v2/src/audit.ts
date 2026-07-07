// ポーズ監査ページ (開発用)。UI 層を通さず、poses/fighter だけで全ペアを並べて描画する。
// _audit.html?q=side のようにペア名で絞り込み、?one=redSideControl+blueUnderSide で単体表示。

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { Fighter } from "./render/fighter";
import { POSES, type PoseName } from "./render/poses";

const PAIRS: [PoseName, PoseName][] = [
  ["standingRed", "standingBlue"],
  ["redBackControl", "blueSeatedFront"],
  ["redBackControl", "blueBackDefend"],
  ["redBackControl", "blueTapped"],
  ["redBackControl", "blueGivesBack"],
  ["redMountTop", "blueUnderMount"],
  ["redMountArmbar", "blueUnderMount"],
  ["redRolledBottom", "blueUpaTop"],
  ["redSideControl", "blueUnderSide"],
  ["redSideControl", "blueShrimpRecover"],
  ["redClosedGuardBottom", "blueTopInGuard"],
  ["redClosedGuardBottom", "blueGuardPass"],
  ["redGuardOpened", "blueGuardPass"],
  ["redGuardArmbarFinish", "blueGuardArmbarCaught"],
  ["redTriangleFinish", "blueCaughtInTriangle"],
];

const params = new URLSearchParams(location.search);
const q = params.get("q");
const one = params.get("one");
let pairs = PAIRS;
if (one) {
  const [r, b] = one.split(" ").join("+").split("+");
  pairs = PAIRS.filter(([pr, pb]) => pr === r && pb === b);
} else if (q) {
  pairs = PAIRS.filter(([pr, pb]) => pr.toLowerCase().includes(q.toLowerCase()) || pb.toLowerCase().includes(q.toLowerCase()));
}

const canvas = document.getElementById("c") as HTMLCanvasElement;
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x14161c);
scene.add(new THREE.AmbientLight(0xffffff, 0.7));
const key = new THREE.DirectionalLight(0xfff2df, 1.4);
key.position.set(4, 8, 3);
scene.add(key);

const COLS = Math.ceil(Math.sqrt(pairs.length));
const GAP = 2.6;
const anchors: { name: string; pos: THREE.Vector3 }[] = [];

pairs.forEach(([redName, blueName], i) => {
  const col = i % COLS;
  const row = Math.floor(i / COLS);
  const ox = (col - (COLS - 1) / 2) * GAP;
  const oz = (row - (Math.ceil(pairs.length / COLS) - 1) / 2) * GAP;

  const cell = new THREE.Group();
  cell.position.set(ox, 0, oz);
  scene.add(cell);

  const floor = new THREE.Mesh(
    new THREE.CircleGeometry(1.15, 32),
    new THREE.MeshStandardMaterial({ color: 0x3f6f52, roughness: 1 }),
  );
  floor.rotation.x = -Math.PI / 2;
  cell.add(floor);

  const red = new Fighter({ color: 0xc23b3b, accent: 0xffc9c9 });
  const blue = new Fighter({ color: 0x2f5fd0, accent: 0xbfd4ff });
  red.applyPose(POSES[redName], { immediate: true });
  blue.applyPose(POSES[blueName], { immediate: true });
  cell.add(red.root);
  cell.add(blue.root);

  anchors.push({ name: `${redName} + ${blueName}`, pos: new THREE.Vector3(ox, 1.35, oz) });
});

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
const span = COLS * GAP;
camera.position.set(span * 0.55, span * 0.7, span * 0.95);
const controls = new OrbitControls(camera, canvas);
controls.target.set(0, 0.3, 0);

const labels = document.getElementById("labels") as HTMLDivElement;
const labelEls = anchors.map(({ name }) => {
  const el = document.createElement("div");
  el.className = "lbl";
  el.textContent = name;
  labels.appendChild(el);
  return el;
});

function resize(): void {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
}
window.addEventListener("resize", resize);
resize();

const v = new THREE.Vector3();
renderer.setAnimationLoop(() => {
  controls.update();
  renderer.render(scene, camera);
  anchors.forEach(({ pos }, i) => {
    v.copy(pos).project(camera);
    const el = labelEls[i];
    if (!el) return;
    el.style.left = `${((v.x + 1) / 2) * innerWidth}px`;
    el.style.top = `${((1 - v.y) / 2) * innerHeight}px`;
    el.style.display = v.z < 1 ? "block" : "none";
  });
});

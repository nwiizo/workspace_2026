import * as T from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { Mannequin } from "./mannequin";
import type { Actor, PracticePair } from "./practicePose";

export type PracticeCamera = "diagonal" | "side" | "top";
export interface BodyHandle { id: string; point: T.Vector3; selected: boolean }

/** 静止した前後を比較する観察台。入力・リサイズ時だけ描画する。 */
export class BodyScene {
  private readonly scene = new T.Scene();
  private readonly camera = new T.OrthographicCamera(-1.5, 1.5, 1, -1, 0.1, 30);
  private readonly renderer: T.WebGLRenderer;
  private readonly controls: OrbitControls;
  private readonly observer: ResizeObserver;
  private readonly contacts = new T.Group();
  private blue?: Mannequin;
  private red?: Mannequin;
  private pair?: PracticePair;
  private active = false;
  private bones = false;
  private faded = false;
  private markers = true;
  private opponent: Actor = "red";
  private readonly handles = new T.Group();
  private edit?: { select: (id: string) => void; move: (id: string, point: T.Vector3) => void };
  private dragging?: { id: string; pointer: number; plane: T.Plane; offset: T.Vector3 };

  constructor(private readonly canvas: HTMLCanvasElement, private readonly pins: HTMLElement) {
    this.renderer = new T.WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = T.PCFSoftShadowMap;
    this.scene.background = new T.Color(0x172331);
    this.scene.add(new T.HemisphereLight(0xf1f6ff, 0x677381, 2));
    const key = new T.DirectionalLight(0xffecd4, 2.6);
    key.position.set(-2, 5, 3); key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.camera.left = -2; key.shadow.camera.right = 2;
    key.shadow.camera.top = 2; key.shadow.camera.bottom = -2;
    key.shadow.normalBias = 0.012;
    this.scene.add(key);
    const mat = new T.Mesh(new T.PlaneGeometry(12, 12), new T.MeshStandardMaterial({ color: 0x364958, roughness: 1 }));
    mat.rotation.x = -Math.PI / 2; mat.receiveShadow = true; this.scene.add(mat);
    const grid = new T.GridHelper(8, 32, 0x607080, 0x435869);
    grid.position.y = 0.001; this.scene.add(grid, this.contacts, this.handles);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.target.set(0.08, 0.36, 0.23);
    this.controls.enablePan = false;
    this.controls.minPolarAngle = 0.015;
    this.controls.maxPolarAngle = Math.PI / 2 - 0.025;
    this.controls.minZoom = 0.7; this.controls.maxZoom = 2.8;
    this.controls.addEventListener("change", this.draw);
    this.setCamera("diagonal");
    this.observer = new ResizeObserver(() => this.refreshSize());
    this.observer.observe(canvas.parentElement!);
    canvas.addEventListener("pointerdown", this.pointerDown, true);
    canvas.addEventListener("pointermove", this.pointerMove, true);
    canvas.addEventListener("pointerup", this.pointerUp, true);
    canvas.addEventListener("pointercancel", this.pointerUp, true);
  }

  setHandles(handles: BodyHandle[], select: (id: string) => void, move: (id: string, point: T.Vector3) => void): void {
    this.edit = { select, move };
    this.handles.traverse((obj) => { if (obj instanceof T.Mesh) { obj.geometry.dispose(); (obj.material as T.Material).dispose(); } });
    this.handles.clear();
    for (const handle of handles) {
      const mesh = new T.Mesh(new T.SphereGeometry(handle.selected ? 0.042 : 0.029, 12, 8), new T.MeshBasicMaterial({ color: handle.selected ? 0xffd378 : 0xb8f5ef, depthTest: false }));
      mesh.position.copy(handle.point); mesh.userData.handle = handle.id; mesh.renderOrder = 10;
      this.handles.add(mesh);
    }
    this.draw();
  }

  private ray(event: PointerEvent): T.Raycaster {
    const rect = this.canvas.getBoundingClientRect();
    const ray = new T.Raycaster();
    ray.setFromCamera(new T.Vector2((event.clientX - rect.left) / rect.width * 2 - 1, 1 - (event.clientY - rect.top) / rect.height * 2), this.camera);
    return ray;
  }
  private pointerDown = (event: PointerEvent): void => {
    if (!this.active || !this.edit || event.button !== 0) return;
    const ray = this.ray(event), hit = ray.intersectObjects(this.handles.children)[0];
    if (!hit) return;
    const id = String(hit.object.userData.handle);
    const plane = new T.Plane().setFromNormalAndCoplanarPoint(this.camera.getWorldDirection(new T.Vector3()), hit.object.position);
    const at = ray.ray.intersectPlane(plane, new T.Vector3());
    if (!at) return;
    this.dragging = { id, pointer: event.pointerId, plane, offset: hit.object.position.clone().sub(at) };
    this.controls.enabled = false;
    event.preventDefault(); event.stopImmediatePropagation();
    this.canvas.setPointerCapture(event.pointerId);
    this.edit.select(id);
  };
  private pointerMove = (event: PointerEvent): void => {
    const drag = this.dragging;
    if (!drag || drag.pointer !== event.pointerId) return;
    event.preventDefault(); event.stopImmediatePropagation();
    const point = this.ray(event).ray.intersectPlane(drag.plane, new T.Vector3());
    if (point) this.edit?.move(drag.id, point.add(drag.offset));
  };
  private pointerUp = (event: PointerEvent): void => {
    if (!this.dragging || this.dragging.pointer !== event.pointerId) return;
    event.stopImmediatePropagation();
    if (this.canvas.hasPointerCapture(event.pointerId)) this.canvas.releasePointerCapture(event.pointerId);
    this.dragging = undefined; this.controls.enabled = this.active;
  };

  setCamera(view: PracticeCamera): void {
    const offset = view === "top" ? new T.Vector3(0, 5, 0.01) : view === "side" ? new T.Vector3(3.6, 0.55, 0) : new T.Vector3(2.8, 3.1, 3.8);
    this.camera.position.copy(this.controls.target).add(offset);
    this.camera.zoom = 1;
    this.camera.updateProjectionMatrix();
    this.controls.update();
    this.draw();
  }

  show(pair: PracticePair): void {
    this.pair = pair;
    if (this.blue && this.red) { this.blue.updatePose(pair.blue); this.red.updatePose(pair.red); }
    else {
      this.blue = new Mannequin(pair.blue, 0x629de6); this.red = new Mannequin(pair.red, 0xd87966);
      this.scene.add(this.blue.root, this.red.root);
    }
    this.clearContacts();
    for (const { actor, joint } of pair.supports) {
      const ring = new T.Mesh(new T.RingGeometry(0.079, 0.095, 32), new T.MeshBasicMaterial({ color: actor === "blue" ? 0xa7d6ff : 0xffbaa2, side: T.DoubleSide }));
      ring.rotation.x = -Math.PI / 2;
      ring.position.copy(pair[actor][joint]).setY(0.003);
      this.contacts.add(ring);
    }
    this.pins.replaceChildren(...pair.cues.map((cue, index) => {
      const pin = document.createElement("span");
      pin.className = `practice-pin pin-${cue.kind}`;
      pin.textContent = String(index + 1);
      return pin;
    }));
    this.setDisplay(this.bones, this.faded, this.markers);
  }

  setDisplay(bones: boolean, faded: boolean, markers: boolean, opponent: Actor = this.opponent): void {
    this.bones = bones; this.faded = faded; this.markers = markers;
    this.opponent = opponent;
    this.blue?.setDisplay(bones, faded && opponent === "blue");
    this.red?.setDisplay(bones, faded && opponent === "red");
    this.contacts.visible = markers; this.pins.hidden = !markers;
    this.draw();
  }

  refreshSize(): void {
    const parent = this.canvas.parentElement;
    if (!parent || parent.clientWidth === 0 || parent.clientHeight === 0) return;
    const width = parent.clientWidth, height = parent.clientHeight;
    this.renderer.setSize(width, height, false);
    const aspect = width / height;
    const halfHeight = Math.max(1.04, 1.27 / aspect);
    this.camera.left = -halfHeight * aspect; this.camera.right = halfHeight * aspect;
    this.camera.top = halfHeight; this.camera.bottom = -halfHeight;
    this.camera.updateProjectionMatrix();
    this.draw();
  }

  setActive(active: boolean): void {
    this.active = active;
    this.controls.enabled = active;
    if (!active && this.dragging) {
      if (this.canvas.hasPointerCapture(this.dragging.pointer)) this.canvas.releasePointerCapture(this.dragging.pointer);
      this.dragging = undefined;
    }
    if (active) this.refreshSize();
  }

  private draw = (): void => {
    if (!this.active) return;
    this.renderer.render(this.scene, this.camera);
    this.pair?.cues.forEach((cue, index) => {
      const point = cue.point.clone().project(this.camera);
      const pin = this.pins.children[index] as HTMLElement | undefined;
      if (!pin) return;
      pin.hidden = point.z < -1 || point.z > 1 || Math.abs(point.x) > 0.95 || Math.abs(point.y) > 0.95;
      pin.style.left = `${(point.x + 1) * 50}%`;
      pin.style.top = `${(1 - point.y) * 50}%`;
    });
  };

  private clearContacts(): void {
    this.contacts.traverse((obj) => { if (obj instanceof T.Mesh) { obj.geometry.dispose(); (obj.material as T.Material).dispose(); } });
    this.contacts.clear();
  }

  dispose(): void {
    this.active = false; this.observer.disconnect(); this.controls.dispose();
    this.canvas.removeEventListener("pointerdown", this.pointerDown, true);
    this.canvas.removeEventListener("pointermove", this.pointerMove, true);
    this.canvas.removeEventListener("pointerup", this.pointerUp, true);
    this.canvas.removeEventListener("pointercancel", this.pointerUp, true);
    this.blue?.dispose(); this.red?.dispose(); this.clearContacts();
    this.scene.remove(...[this.blue?.root, this.red?.root].filter((item): item is T.Group => !!item));
    this.scene.traverse((obj) => {
      if (obj instanceof T.Mesh || obj instanceof T.LineSegments) {
        obj.geometry.dispose();
        const materials = Array.isArray(obj.material) ? obj.material : [obj.material];
        materials.forEach((material) => material.dispose());
      }
      if (obj instanceof T.DirectionalLight) obj.shadow.dispose();
    });
    this.renderer.dispose();
  }
}

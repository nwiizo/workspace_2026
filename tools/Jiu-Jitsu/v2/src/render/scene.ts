// Three.js シーン: 畳・ライト・カメラ。赤青 2 体または単体 (関節ラボ) を載せる。

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { Fighter } from "./fighter";

export interface DojoSceneOptions {
  canvas: HTMLCanvasElement;
  /** true なら赤青 2 体、false なら単体 (関節ラボ用) */
  pair: boolean;
}

export class DojoScene {
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  readonly renderer: THREE.WebGLRenderer;
  readonly controls: OrbitControls;
  readonly red: Fighter | null;
  readonly blue: Fighter;

  private clock = new THREE.Clock();
  private disposed = false;

  constructor({ canvas, pair }: DojoSceneOptions) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.shadowMap.enabled = true;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.scene.background = new THREE.Color(0x14161c);
    this.scene.fog = new THREE.Fog(0x14161c, 6, 14);

    this.camera = new THREE.PerspectiveCamera(42, 1, 0.1, 40);
    this.camera.position.set(2.3, 1.7, 2.6);

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.target.set(0, 0.45, 0);
    this.controls.enableDamping = true;
    this.controls.maxPolarAngle = Math.PI * 0.52;
    this.controls.minDistance = 1.2;
    this.controls.maxDistance = 8;

    // 畳
    const mat = new THREE.Mesh(
      new THREE.CylinderGeometry(4.4, 4.4, 0.08, 48),
      new THREE.MeshStandardMaterial({ color: 0x3f6f52, roughness: 0.95 }),
    );
    mat.position.y = -0.04;
    mat.receiveShadow = true;
    this.scene.add(mat);
    const edge = new THREE.Mesh(
      new THREE.TorusGeometry(4.4, 0.05, 10, 60),
      new THREE.MeshStandardMaterial({ color: 0x2b4a38, roughness: 0.9 }),
    );
    edge.rotation.x = Math.PI / 2;
    this.scene.add(edge);

    // ライト
    this.scene.add(new THREE.AmbientLight(0xffffff, 0.55));
    const key = new THREE.DirectionalLight(0xfff2df, 1.6);
    key.position.set(3, 5, 2.5);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    this.scene.add(key);
    const rim = new THREE.DirectionalLight(0x9db8ff, 0.5);
    rim.position.set(-3, 3, -3);
    this.scene.add(rim);

    this.blue = new Fighter({ color: 0x2f5fd0, accent: 0xbfd4ff });
    this.scene.add(this.blue.root);
    if (pair) {
      this.red = new Fighter({ color: 0xc23b3b, accent: 0xffc9c9 });
      this.scene.add(this.red.root);
    } else {
      this.red = null;
    }

    this.resize();
    window.addEventListener("resize", this.resize);
    this.renderer.setAnimationLoop(this.tick);
  }

  private resize = (): void => {
    const canvas = this.renderer.domElement;
    const parent = canvas.parentElement;
    if (!parent) return;
    const w = parent.clientWidth;
    const h = parent.clientHeight;
    if (w === 0 || h === 0) return;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  };

  /** 外部レイアウト変更後に呼ぶ (タブ切替など) */
  refreshSize(): void {
    this.resize();
  }

  private tick = (): void => {
    if (this.disposed) return;
    const dt = Math.min(this.clock.getDelta(), 0.05);
    this.blue.update(dt);
    this.red?.update(dt);
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
  };

  dispose(): void {
    this.disposed = true;
    this.renderer.setAnimationLoop(null);
    window.removeEventListener("resize", this.resize);
    this.controls.dispose();
    this.renderer.dispose();
  }
}

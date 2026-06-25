// scene.js
// Three.js のシーン・カメラ・ライト・畳・カメラ操作をまとめる。
// 2 体の Fighter を保持し、毎フレーム update する。

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { Fighter } from "./fighter.js";

export class Dojo {
  constructor(container) {
    this.container = container;

    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x0b1018);
    this.scene.fog = new THREE.Fog(0x0b1018, 7, 16);

    this.camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
    this.camera.position.set(3.6, 2.15, 0.35);

    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.08;
    container.appendChild(this.renderer.domElement);

    // ソフトな環境反射 (PBR マテリアルに立体感を与える)
    const pmrem = new THREE.PMREMGenerator(this.renderer);
    this.scene.environment = pmrem.fromScene(
      new RoomEnvironment(),
      0.04,
    ).texture;

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.target.set(0, 0.5, 0);
    this.camera.lookAt(this.controls.target);
    this.controls.minDistance = 2;
    this.controls.maxDistance = 8;
    this.controls.maxPolarAngle = Math.PI * 0.495; // 床下に潜らない
    this.controls.autoRotate = false;
    this.controls.autoRotateSpeed = 0.7;

    this._buildEnvironment();

    // 赤 = 仕掛ける側, 青 = あなた (守り→攻め)
    this.red = new Fighter({ color: 0xe5484d, accent: 0x3a0d0e, mode: "gi" });
    this.blue = new Fighter({ color: 0x3b82f6, accent: 0x0c1c3a, mode: "gi" });
    this.scene.add(this.red.root, this.blue.root);

    this._clock = new THREE.Clock();
    this._onResize = () => this.resize();
    window.addEventListener("resize", this._onResize);
    this.resize();
  }

  _buildEnvironment() {
    // 環境光 + 主光 (影付き) + 補助光
    this.scene.add(new THREE.HemisphereLight(0x9fb6d6, 0x202830, 0.7));

    const key = new THREE.DirectionalLight(0xffffff, 1.6);
    key.position.set(3, 6, 4);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    const s = 4;
    key.shadow.camera.left = -s;
    key.shadow.camera.right = s;
    key.shadow.camera.top = s;
    key.shadow.camera.bottom = -s;
    key.shadow.camera.near = 0.5;
    key.shadow.camera.far = 20;
    key.shadow.bias = -0.0004;
    this.scene.add(key);

    const rim = new THREE.DirectionalLight(0x6ea8ff, 0.5);
    rim.position.set(-4, 3, -3);
    this.scene.add(rim);

    // 畳マット (市松模様)
    const matGroup = new THREE.Group();
    const tiles = 8;
    const tileSize = 0.6;
    const a = new THREE.MeshStandardMaterial({ color: 0x20303f, roughness: 0.95 });
    const b = new THREE.MeshStandardMaterial({ color: 0x1a2733, roughness: 0.95 });
    const geo = new THREE.BoxGeometry(tileSize, 0.06, tileSize);
    for (let i = 0; i < tiles; i++) {
      for (let j = 0; j < tiles; j++) {
        const m = new THREE.Mesh(geo, (i + j) % 2 ? a : b);
        m.position.set(
          (i - tiles / 2 + 0.5) * tileSize,
          -0.03,
          (j - tiles / 2 + 0.5) * tileSize,
        );
        m.receiveShadow = true;
        matGroup.add(m);
      }
    }
    this.scene.add(matGroup);

    // 中央の競技円 (リング)
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(1.6, 1.68, 48),
      new THREE.MeshBasicMaterial({ color: 0xe3b341, side: THREE.DoubleSide }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.005;
    this.scene.add(ring);
  }

  /** 攻防それぞれのポーズを同時にセット */
  setPoses(redPose, bluePose, opts = {}) {
    this.red.applyPose(redPose, opts);
    this.blue.applyPose(bluePose, opts);
  }

  setAutoRotate(on) {
    this.controls.autoRotate = on;
  }

  setUniformMode(mode) {
    this.red.setMode(mode);
    this.blue.setMode(mode);
  }

  resize() {
    const w = this.container.clientWidth;
    const h = this.container.clientHeight;
    if (w === 0 || h === 0) return;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  start() {
    const loop = () => {
      this._raf = requestAnimationFrame(loop);
      const dt = Math.min(this._clock.getDelta(), 0.05);
      this.red.update(dt);
      this.blue.update(dt);
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
    };
    loop();
  }
}

// main.js
// 起動エントリ。Dojo (3D) と UI を生成し Game で繋ぐ。

import { Dojo } from "./scene.js";
import { UI } from "./ui.js";
import { Game } from "./game.js";

function boot() {
  const stage = document.getElementById("stage");
  const loading = document.getElementById("loading");

  let dojo;
  try {
    dojo = new Dojo(stage);
  } catch (err) {
    loading.textContent =
      "3D 初期化に失敗しました。WebGL 対応ブラウザで、静的サーバー経由 (file:// 不可) で開いてください。";
    console.error(err);
    return;
  }

  loading.remove();
  dojo.start();

  const ui = new UI();
  const game = new Game(dojo, ui);
  game.start();
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}

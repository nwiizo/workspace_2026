// 起動: #app に App を載せ、稽古記録は localStorage に永続化する。

import "./style.css";
import { App } from "./ui/app";

const root = document.getElementById("app");
if (!root) throw new Error("#app not found");

new App(root, window.localStorage);

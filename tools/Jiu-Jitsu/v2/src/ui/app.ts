// アプリシェル: ヘッダー (タイトル + タブ + 現在の帯) と 4 タブの切替。
// DojoScene / JointLab は各タブで 1 度だけ生成し、タブ切替では refreshSize する。

import { allItemKeys } from "../engine/roll";
import { beltFor, masteredCount } from "../engine/srs";
import { loadProgress, type KeyValueStore } from "../engine/storage";
import { createJointLab, type JointLabHandle } from "../labs/jointLab";
import { DojoTab } from "./dojoTab";
import { renderNotesTab } from "./notesTab";
import { renderRecordsTab } from "./recordsTab";
import { h } from "./dom";

type TabId = "dojo" | "lab" | "records" | "notes";

const TABS: readonly (readonly [TabId, string])[] = [
  ["dojo", "道場"],
  ["lab", "関節ラボ"],
  ["records", "稽古記録"],
  ["notes", "心得"],
];

const TOTAL_ITEMS = allItemKeys().length;

export class App {
  private readonly store: KeyValueStore;
  private active: TabId = "dojo";

  private readonly beltEl: HTMLElement;
  private readonly tabButtons = new Map<TabId, HTMLButtonElement>();
  private readonly panels = new Map<TabId, HTMLElement>();

  private readonly dojo: DojoTab;
  private readonly labContainer: HTMLElement;
  private lab: JointLabHandle | null = null;

  constructor(root: HTMLElement, store: KeyValueStore) {
    this.store = store;

    this.beltEl = h("span", { class: "belt-badge" });
    const nav = h("nav", { class: "tabs" });
    for (const [id, label] of TABS) {
      const btn = h("button", { class: "tab", text: label, onClick: () => this.switchTab(id) });
      this.tabButtons.set(id, btn);
      nav.append(btn);
    }
    const header = h("header", { class: "app-header" },
      h("div", { class: "app-brand" },
        h("h1", { class: "app-title", text: "柔術道場" }),
        h("span", { class: "app-subtitle", text: "Grappling Structure Dojo" }),
      ),
      nav,
      this.beltEl,
    );

    this.dojo = new DojoTab(store, {
      switchToLab: () => this.switchTab("lab"),
      onProgressChange: () => this.onProgressChange(),
    });
    this.labContainer = h("div", { class: "lab-root" });
    const recordsPanel = h("div", { class: "tab-panel" });
    const notesPanel = h("div", { class: "tab-panel" });

    this.panels.set("dojo", h("div", { class: "tab-panel" }, this.dojo.root));
    this.panels.set("lab", h("div", { class: "tab-panel" }, this.labContainer));
    this.panels.set("records", recordsPanel);
    this.panels.set("notes", notesPanel);

    const main = h("main", { class: "app-main" });
    for (const panel of this.panels.values()) main.append(panel);

    root.append(header, main);

    renderNotesTab(notesPanel);
    this.updateBelt();
    this.applyActive();

    window.addEventListener("keydown", (e) => {
      if (this.active === "dojo") this.dojo.handleKey(e);
    });

    requestAnimationFrame(() => this.dojo.refreshSize());
  }

  private switchTab(id: TabId): void {
    if (id === this.active) return;
    this.active = id;
    this.applyActive();

    if (id === "dojo") requestAnimationFrame(() => this.dojo.refreshSize());
    if (id === "records") renderRecordsTab(this.panels.get("records")!, this.store);
    if (id === "lab") {
      if (!this.lab) this.lab = createJointLab(this.labContainer);
      requestAnimationFrame(() => this.lab?.refreshSize());
    }
  }

  private applyActive(): void {
    for (const [id, btn] of this.tabButtons) btn.classList.toggle("tab-on", id === this.active);
    for (const [id, panel] of this.panels) panel.classList.toggle("tab-panel-on", id === this.active);
  }

  private onProgressChange(): void {
    this.updateBelt();
    if (this.active === "records") renderRecordsTab(this.panels.get("records")!, this.store);
  }

  private updateBelt(): void {
    const srs = loadProgress(this.store).srs;
    this.beltEl.textContent = beltFor(masteredCount(srs), TOTAL_ITEMS);
  }
}

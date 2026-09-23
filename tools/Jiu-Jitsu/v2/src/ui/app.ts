// 動きの道場を入口に、ポジション練習・資料・記録をつなぐ。

import { loadProgress, type KeyValueStore } from "../engine/storage";
import { BodyLab, type BodyExperiment } from "../labs/bodyLab";
import { DojoTab } from "./dojoTab";
import { PracticeTab } from "./practiceTab";
import { renderNotesTab } from "./notesTab";
import { renderRecordsTab } from "./recordsTab";
import { h } from "./dom";
import { RouteTab } from "./routeTab";

type TabId = "practice" | "dojo" | "lab" | "route" | "route-watch" | "records" | "notes";

const TABS: readonly (readonly [TabId, string])[] = [
  ["lab", "動きの道場"],
  ["practice", "ポジション練習"],
  ["route-watch", "ルート確認"],
  ["route", "ルート作成"],
  ["dojo", "応用ロール"],
  ["records", "稽古記録"],
  ["notes", "心得"],
];

export class App {
  private readonly store: KeyValueStore;
  private active: TabId = "lab";

  private readonly beltEl: HTMLElement;
  private readonly tabButtons = new Map<TabId, HTMLButtonElement>();
  private readonly panels = new Map<TabId, HTMLElement>();

  private dojo: DojoTab | null = null;
  private route: RouteTab | null = null;
  private routeWatch: RouteTab | null = null;
  private readonly practice: PracticeTab;
  private readonly labContainer: HTMLElement;
  private lab: BodyLab | null = null;
  private returningToPractice = false;

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
        h("span", { class: "app-subtitle", text: "JIU-JITSU / 一手ずつ、理解する" }),
      ),
      nav,
      this.beltEl,
    );

    this.practice = new PracticeTab(store, () => this.onProgressChange(), (experiment) => this.openBody(experiment), (course) => {
      this.switchTab("route-watch"); this.routeWatch?.openCourse(course);
    });
    this.labContainer = h("div", { class: "lab-root" });
    this.lab = new BodyLab(this.labContainer, (experiment) => {
      if (!this.returningToPractice) this.practice.openLesson(experiment === "base" ? "mount-base" : "side-space");
      this.returningToPractice = false;
      this.switchTab("practice");
    }, store, () => this.onProgressChange());
    const recordsPanel = h("div", { class: "tab-panel" });
    const notesPanel = h("div", { class: "tab-panel" });

    this.panels.set("practice", h("div", { class: "tab-panel" }, this.practice.root));
    this.panels.set("dojo", h("div", { class: "tab-panel" }));
    this.panels.set("route", h("div", { class: "tab-panel" }));
    this.panels.set("route-watch", h("div", { class: "tab-panel" }));
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
      if (this.active === "dojo") this.dojo?.handleKey(e);
      if (this.active === "practice") this.practice.handleKey(e);
    });
    document.addEventListener("visibilitychange", () => this.applyActive());
    requestAnimationFrame(() => this.applyActive());
  }

  private switchTab(id: TabId): void {
    if (id === this.active) return;
    this.active = id;
    this.applyActive();

    if (id === "dojo") {
      if (!this.dojo) {
        this.dojo = new DojoTab(this.store, {
          switchToLab: () => this.switchTab("lab"),
          onProgressChange: () => this.onProgressChange(),
        });
        this.panels.get("dojo")!.append(this.dojo.root);
      }
      requestAnimationFrame(() => this.dojo?.refreshSize());
    }
    if (id === "records") renderRecordsTab(this.panels.get("records")!, this.store);
    if (id === "route" && !this.route) {
      this.route = new RouteTab(this.store, "edit", (item) => { this.switchTab("route-watch"); this.routeWatch?.openRoute(item); }); this.panels.get("route")!.append(this.route.root);
      requestAnimationFrame(() => this.applyActive());
    }
    if (id === "route-watch" && !this.routeWatch) {
      this.routeWatch = new RouteTab(this.store, "watch", (item) => { this.switchTab("route"); this.route?.editExample(item); });
      this.panels.get("route-watch")!.append(this.routeWatch.root);
      requestAnimationFrame(() => this.applyActive());
    }
    if (id === "lab") {
      requestAnimationFrame(() => this.lab?.refreshSize());
    }
    this.applyActive();
  }

  private openBody(experiment: BodyExperiment): void {
    this.returningToPractice = true;
    this.switchTab("lab");
    this.lab?.open(experiment);
  }

  private applyActive(): void {
    for (const [id, btn] of this.tabButtons) {
      btn.classList.toggle("tab-on", id === this.active);
      btn.setAttribute("aria-pressed", String(id === this.active));
    }
    for (const [id, panel] of this.panels) panel.classList.toggle("tab-panel-on", id === this.active);
    this.practice.setActive(this.active === "practice" && !document.hidden);
    this.dojo?.setActive(this.active === "dojo" && !document.hidden);
    this.route?.setActive(this.active === "route" && !document.hidden);
    this.routeWatch?.setActive(this.active === "route-watch" && !document.hidden);
    this.lab?.setActive(this.active === "lab" && !document.hidden);
  }

  private onProgressChange(): void {
    this.updateBelt();
    if (this.active === "records") renderRecordsTab(this.panels.get("records")!, this.store);
  }

  private updateBelt(): void {
    const srs = loadProgress(this.store).srs;
    this.beltEl.textContent = `動きの発見 ${["lever", "base", "leg"].filter((id) => srs[`body:${id}`]).length} / 3`;
  }
}

// タブ4: 心得 — 道場の普遍原則と研究の裏付けを静的表示する。

import { DOJO_NOTES } from "../content/dojoNotes";
import { h } from "./dom";

export function renderNotesTab(container: HTMLElement): void {
  container.replaceChildren();
  const wrap = h("div", { class: "notes" });

  wrap.append(section("ポジション階層", hierarchyList()));
  wrap.append(section("五つの原則", pairList(DOJO_NOTES.principles)));
  wrap.append(section("用語", pairList(DOJO_NOTES.glossary)));
  wrap.append(section("研究の裏付け", pairList(DOJO_NOTES.research)));

  container.append(wrap);
}

function section(title: string, body: HTMLElement): HTMLElement {
  return h("section", { class: "notes-section" }, h("h2", { text: title }), body);
}

function hierarchyList(): HTMLElement {
  const ol = h("ol", { class: "hierarchy" });
  for (const item of DOJO_NOTES.hierarchy) ol.append(h("li", { text: item }));
  return ol;
}

function pairList(pairs: readonly (readonly [string, string])[]): HTMLElement {
  const dl = h("dl", { class: "pair-list" });
  for (const [term, desc] of pairs) {
    dl.append(h("dt", { text: term }));
    dl.append(h("dd", { text: desc }));
  }
  return dl;
}

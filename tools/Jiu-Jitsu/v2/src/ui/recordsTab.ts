// タブ3: 稽古記録 — (局面 × 相手初動) ごとの習熟を Leitner box で可視化する。

import { allItemKeys } from "../engine/roll";
import { beltFor, isDue, MASTERY_BOX, MAX_BOX, masteredCount } from "../engine/srs";
import { loadProgress, type KeyValueStore } from "../engine/storage";
import type { Scenario } from "../content/types";
import { h } from "./dom";

export function renderRecordsTab(container: HTMLElement, store: KeyValueStore): void {
  container.replaceChildren();
  const progress = loadProgress(store);
  const srs = progress.srs;
  const now = Date.now();
  const items = allItemKeys();
  const total = items.length;
  const mastered = masteredCount(srs);

  const wrap = h("div", { class: "records" });

  const summary = h("div", { class: "records-summary" });
  summary.append(stat(`${mastered}`, `習得 / 全 ${total}`));
  summary.append(stat(beltFor(mastered, total), "現在の帯"));
  summary.append(stat(`${progress.rollsCompleted}`, "完了ロール数"));
  wrap.append(summary);

  // 局面ごとにグループ表示
  const byScenario = new Map<string, { scenario: Scenario; rows: typeof items }>();
  for (const it of items) {
    const g = byScenario.get(it.scenario.id);
    if (g) g.rows.push(it);
    else byScenario.set(it.scenario.id, { scenario: it.scenario, rows: [it] });
  }

  for (const { scenario, rows } of byScenario.values()) {
    const card = h("section", { class: "records-card" });
    card.append(
      h(
        "header",
        { class: "records-card-head" },
        h("span", { class: "belt-tag", text: scenario.belt }),
        h("h3", { text: scenario.positionJp }),
        h("span", { class: "records-card-en", text: scenario.positionEn }),
      ),
    );
    for (const { key, action } of rows) {
      const box = srs[key]?.box;
      const learned = box !== undefined;
      const row = h("div", { class: "records-row" });
      row.append(h("span", { class: "records-action", text: action.label }));
      row.append(boxBar(learned ? box : -1));

      const badges = h("span", { class: "records-badges" });
      if (learned && box >= MASTERY_BOX) {
        badges.append(h("span", { class: "badge badge-mastered", text: "習得" }));
      }
      if (isDue(srs, key, now)) {
        badges.append(h("span", { class: "badge badge-due", text: learned ? "復習どき" : "未学習" }));
      }
      row.append(badges);
      card.append(row);
    }
    wrap.append(card);
  }

  container.append(wrap);
}

function stat(value: string, label: string): HTMLElement {
  return h("div", { class: "stat" }, h("span", { class: "stat-value", text: value }), h("span", { class: "stat-label", text: label }));
}

/** box を 0..MAX_BOX のドットで表す。box=-1 は未学習 (全グレー) */
function boxBar(box: number): HTMLElement {
  const bar = h("div", { class: "box-bar" });
  for (let i = 1; i <= MAX_BOX; i++) {
    const on = box >= i;
    bar.append(h("span", { class: `box-dot${on ? " box-dot-on" : ""}` }));
  }
  return bar;
}

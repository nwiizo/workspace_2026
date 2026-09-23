// タブ3: 稽古記録 — (局面 × 相手初動) ごとの習熟を Leitner box で可視化する。

import { allItemKeys } from "../engine/roll";
import { isDue, MASTERY_BOX, MAX_BOX } from "../engine/srs";
import { PRACTICE_LESSONS } from "../content/practice";
import { checkedPracticeCount, practiceKey } from "../engine/practice";
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
  const mastered = items.filter(({ key }) => (srs[key]?.box ?? -1) >= MASTERY_BOX).length;

  const wrap = h("div", { class: "records" });
  wrap.append(h("h2", { text: "動かして発見したこと" }));
  for (const [id, name, principle] of [
    ["lever", "腕とテコ", "同じ力でも、受ける位置と骨の向きで回転作用が変わる。"],
    ["base", "支えとバランス", "支えを広げることと、重心を戻すことは、位置関係を変える別々の方法。"],
    ["leg", "股関節と膝", "股関節で膝の位置が動き、膝を曲げると足先の位置が変わる。"],
  ]) wrap.append(h("section", { class: "records-card" }, h("h3", { text: `${srs[`body:${id}`] ? "✓" : "○"} ${name}` }), h("p", { text: srs[`body:${id}`] ? principle : "動きの道場で、点やスライダーを動かして試せます。" })));
  wrap.append(h("h2", { text: `基礎練習：確認 ${checkedPracticeCount(srs)} / ${PRACTICE_LESSONS.length}` }),
    h("p", { text: "ヒントなしで最後まで判断できた課題の記録です。実技の習得や帯の評価ではありません。記録はこのブラウザに保存されます。" }));
  for (const lesson of PRACTICE_LESSONS) {
    const learned = srs[practiceKey(lesson.id, "learn")];
    const checked = srs[practiceKey(lesson.id, "check")];
    const status = checked?.box > 0 ? "確認済み" : checked ? "もう一度確認" : learned ? "練習済み・次は確認" : "未練習";
    const due = checked ? checked.dueAt <= now ? "復習どき" : `次の復習：${new Date(checked.dueAt).toLocaleString("ja-JP", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}` : "";
    wrap.append(h("section", { class: "records-card" }, h("h3", { text: lesson.title }), h("p", { text: `${status}${due ? ` / ${due}` : ""}` }), h("p", { text: lesson.principle })));
  }
  wrap.append(h("h2", { text: "応用ロールの判断記録" }));

  const summary = h("div", { class: "records-summary" });
  summary.append(stat(`${mastered}`, `習得 / 全 ${total}`));
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

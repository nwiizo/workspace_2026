import { PRACTICE_LESSONS } from "../content/practice";
import { checkedPracticeCount, practiceKey } from "../engine/practice";
import { loadProgress, type KeyValueStore } from "../engine/storage";
import { h } from "./dom";

export function renderRecordsTab(container: HTMLElement, store: KeyValueStore): void {
  container.replaceChildren();
  const progress = loadProgress(store);
  const srs = progress.srs;
  const now = Date.now();

  const wrap = h("div", { class: "records" });
  wrap.append(h("h2", { text: "動かして発見したこと" }));
  for (const [id, name, principle] of [
    ["lever", "腕とテコ", "同じ力でも、受ける位置と骨の向きで回転作用が変わる。"],
    ["base", "支えとバランス", "支えを広げることと、重心を戻すことは、位置関係を変える別々の方法。"],
    ["leg", "股関節と膝", "股関節で膝の位置が動き、膝を曲げると足先の位置が変わる。"],
  ]) wrap.append(h("section", { class: "records-card" }, h("h3", { text: `${srs[`body:${id}`] ? "✓" : "○"} ${name}` }), h("p", { text: srs[`body:${id}`] ? principle : "動きの道場で、点やスライダーを動かして試せます。" })));
  wrap.append(h("h2", { text: `ポジション練習：確認 ${checkedPracticeCount(srs)} / ${PRACTICE_LESSONS.length}` }),
    h("p", { text: "ヒントなしで最後まで判断できた課題の記録です。実技の習得や帯の評価ではありません。記録はこのブラウザに保存されます。" }));
  for (const lesson of PRACTICE_LESSONS) {
    const learned = srs[practiceKey(lesson.id, "learn")];
    const checked = srs[practiceKey(lesson.id, "check")];
    const status = checked?.box > 0 ? "確認済み" : checked ? "もう一度確認" : learned ? "練習済み・次は確認" : "未練習";
    const due = checked ? checked.dueAt <= now ? "復習どき" : `次の復習：${new Date(checked.dueAt).toLocaleString("ja-JP", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}` : "";
    wrap.append(h("section", { class: "records-card" }, h("h3", { text: lesson.title }), h("p", { text: `${status}${due ? ` / ${due}` : ""}` }), h("p", { text: lesson.principle })));
  }
  wrap.append(h("h2", { text: "対戦の振り返り" }), h("p", {}, "応用ロールの「動きの変遷」で、青と赤の行動やトップの交代を振り返れます。対戦の履歴は再読込・新しい対戦で消えます。", h("a", { text: "応用ロールを開く", attrs: { href: "#dojo" } })));

  container.append(wrap);
}

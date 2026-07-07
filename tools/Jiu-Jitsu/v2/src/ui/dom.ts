// 素の DOM を組む最小ヘルパ。フレームワークは使わない。
// html は作者管理のコンテンツ定数のみ渡す (ユーザー入力経路なし)。

export interface Props {
  class?: string;
  text?: string;
  html?: string;
  onClick?: () => void;
  attrs?: Record<string, string>;
}

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Props = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (props.class) el.className = props.class;
  if (props.text !== undefined) el.textContent = props.text;
  if (props.html !== undefined) el.innerHTML = props.html;
  if (props.onClick) el.addEventListener("click", props.onClick);
  if (props.attrs) {
    for (const [k, v] of Object.entries(props.attrs)) el.setAttribute(k, v);
  }
  el.append(...children);
  return el;
}

export function clear(el: HTMLElement): void {
  el.replaceChildren();
}

export type ChipKind = "base" | "read" | "missed" | "state";

export function chip(text: string, kind: ChipKind = "base"): HTMLElement {
  return h("span", { class: `chip chip-${kind}`, text });
}

export function chipRow(labels: readonly string[], kind: ChipKind = "base"): HTMLElement {
  const row = h("div", { class: "chip-row" });
  for (const l of labels) row.append(chip(l, kind));
  return row;
}

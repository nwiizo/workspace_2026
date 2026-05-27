---
name: marp-slide-editing
description: Marpプレゼンテーションの技術的制約とベストプラクティス。marp:trueのMarkdown編集時やスライド作成リクエスト時に自動使用。
---

# Marp Slide Editing Skill

## Trigger Conditions

- `marp: true` フロントマターを含むMarkdownファイルの編集時
- 「スライドを作って」「プレゼンを編集して」等のMarp関連リクエスト

## Markdown Rendering Issues

### `**太字**` vs `<strong>` 問題

Marpの `<div>` 要素内では、Markdownの `**text**` 記法が正しくレンダリングされないことがある。

**ルール:**
- `<div>` 内の太字は常に `<strong>text</strong>` を使う
- テーブル内の太字も `<strong>` の方が安全
- 通常のMarkdown（div外）では `**text**` で問題なし
- 新規作成・修正とも `<strong>` で統一する

### 画像の貼り方

`<div>` 内でMarkdownの画像記法 `![alt](src)` を使うと表示されないことがある。HTMLの `<img>` タグを使う。

```html
<div style="text-align: center;">
<img src="path/to/image.png" alt="説明" style="max-width: 80%;">
</div>
```

## Slide Layout

### フォントサイズの階層

| サイズ | 用途 | 判定 |
|---|---|---|
| 0.8em | 余裕のあるスライド | OK |
| 0.75em | 標準 | 推奨 |
| 0.7em | やや詰まった | 注意 |
| 0.68em以下 | 分割が必要 | NG |
| 0.65em | 絶対限界 | 分割必須 |

### スライド分割の判断

以下のいずれかに該当したら分割:
- フォントサイズが 0.68em 以下
- テーブル2つ以上 + 説明文 + パンチラインが同居
- 4カラム Flexbox レイアウト
- テーブル行数5行以上

### 分割時のルール

1. 分割後はフォントサイズを 0.75em（標準）に戻す
2. 各スライドに文脈をつなぐ文章を加筆する（テーブルだけにしない）
3. タイトルは内容を反映した別の文言にする（`(1/2)` 形式は使わない）
4. パンチラインは2枚目に置く

### ボックス内の文言が収まらない場合

文言を短くするより `font-size: 0.9em` 等で微調整する方が内容を保てる。

## CSS Limitations

### position の制約

Marpでは `position: absolute` や `position: sticky` でのタイトル固定は困難:
- Marpはスライドを `<section>` 要素としてレンダリング
- PDF出力時にCSSの一部が無視される
- `position: absolute` はコンテンツの重なり問題を引き起こす

**対策:** 通常のドキュメントフローを使用し、収まらない場合はスライド分割。

### Flexbox の align-items

`align-items: center` を基本指定して垂直中央揃えにする。画像とテキストを並べる場合、画像がスライド上に寄りすぎないよう注意。

## Build & Preview

```bash
# HTML生成
npx @marp-team/marp-cli <file>.md -o <file>.html

# ブラウザで確認
open <file>.html
```

## Editing Discipline

スライド修正時は外科的に行う:
- 指示された箇所のみ修正する。関係ないスライドを変更しない
- パンチラインや表現を勝手に追加・変更しない
- 複数スライドファイルの場合、対象が不明なら確認するか全ファイルを対象にする
- 修正案は複数示し、発表者に選ばせる

## Anti-Patterns

- 1スライドに情報を詰め込みすぎる（1スライド1メッセージが原則）
- `<div>` 内で `**太字**` を使う（`<strong>` を使うべき）
- フォントサイズを 0.65em 以下にして無理に1枚に収める
- 分割スライドのタイトルに `(1/2)` `(2/2)` を使う
- padding/margin を極端に減らして詰め込む
- 依頼されていない表現・文体の「改善」を行う
- 「明日からできること」「アクションアイテム」系スライドを勝手に追加する

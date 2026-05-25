---
name: karpathy-guidelines
description: LLM特有のコーディング失敗（黙って推測する・過剰実装・スコープ滑落・成功基準が曖昧）を抑えるための事前規律。曖昧/大きい/不確実なタスクを受けた直後に明示起動して、実装前に前提と検証可能ゴールを言語化する。
---

# Karpathy Guidelines

Andrej Karpathy の LLM コーディング観察に由来する 4 原則。
**主な追加価値は実装"前"のゲート**（既存 skill は事後レビューが中心）。

> Tradeoff: caution > speed. 自明なタスクには適用しない。

## いつ使うか

- 要件が曖昧（"X を直して" / "Y を改善して" など）
- 影響範囲が読みにくい変更（複数ファイル / 共通基盤 / 既存挙動への上書き）
- 自分の中で前提を 1 つでも仮定したと気づいたとき
- 「とりあえず書いてみる」とコードを打ち始めそうになったとき

## 手順（コード生成前に実行）

### 1. Think Before Coding — 前提を surface する

- 仮定した前提を **3〜5 個** 箇条書きで出す
- 解釈が複数あるなら **全部** 提示する。黙って 1 つ選ばない
- 不明点が残るなら **止まって質問**。何が不明か名指しする
- 既存により簡素な方法があるなら、その案も並べる

### 2. Goal-Driven Execution — 検証可能ゴールに変換

曖昧タスクを次のいずれかの形に書き換えてから着手:

| 入力 | 検証可能ゴール |
|---|---|
| "バグを直して" | 再現するテストを書く → 通す |
| "バリデーション追加" | 不正入力のテストを書く → 通す |
| "リファクタ" | 変更前後でテスト green を確認 |
| "機能追加" | 受け入れ条件をテスト化 → 通す |

複数ステップなら以下の形式で計画を **1 度だけ** 出す:

```
1. [step] → verify: [check]
2. [step] → verify: [check]
3. [step] → verify: [check]
```

### 3. Simplicity First — 書きながら抑制

- 要求されていない機能・抽象・設定可能性を入れない
- 起き得ないシナリオへの error handling を書かない
- 200 行になったら、50 行で済まないか自問

### 4. Surgical Changes — スコープ封じ

- 隣接コードの "improve" は禁止（フォーマット・コメント含む）
- 既存スタイルに合わせる。違和感があっても黙って合わせる
- 関連しない dead code に気付いても **report only / 削除しない**
- 自分の変更が生んだ未使用 import/var/fn のみ片付ける

## 実装後の自己テスト（2 つだけ）

両方 yes になるまでコミットしない:

- **Senior engineer test**: シニアが見て「これ過剰では」と言わないか
- **Traceability test**: diff の **全行** がユーザ要求に直接トレースできるか
  - できない行を 1 つでも見つけたら、その行が本当に必要か再検討

## 既存 skill との関係

- `self-review` / `fix-review-comments`: **事後**レビュー。本 skill は**事前**ゲート
- `iterative-refinement`: Build→Test サイクル。本 skill は最初のループに入る前の準備
- `proactive-suggestions`: 改善提案。本 skill は提案より先に前提確認

## 出典

[Andrej Karpathy on LLM coding pitfalls](https://x.com/karpathy/status/2015883857489522876) /
[multica-ai/andrej-karpathy-skills](https://github.com/multica-ai/andrej-karpathy-skills)

---
name: self-review
description: 手元のコード変更をセルフレビューし、指摘を批判的に評価した上で自動修正するループ。diff/staged/branch/PRを対象に複数レビューアを並列実行。
argument-hint: [レビュー対象] [reviewer名]
---

以下の手順を順番に実行してください。

## ステップ1: 引数の解釈

$ARGUMENTS を以下のルールで解釈してください：
- 第一引数: レビュー対象（省略時は `diff` = 現在のunstaged changes + untracked files）
- 第二引数: reviewer名（省略時は全reviewerを並列実行）

### レビュー対象の指定方法

- 指定なし / `diff`: `git diff` + `git ls-files --others --exclude-standard` で新規ファイルも取得
- `staged`: `git diff --cached`
- `branch`: `git diff origin/main...HEAD`
- `PR #123` または `pr 123`: `gh pr diff 123`
- ファイルパス: そのファイルの内容をレビュー対象にする

### 利用可能なreviewer名

- `code` - home-code-reviewer エージェントによる品質・セキュリティ・パフォーマンスレビュー
- `simplify` - home-simplify-reviewer エージェントによる可読性・一貫性・保守性レビュー（修正せず指摘のみ）
- `codex` - home-codex-reviewer エージェントによる追加観点からのレビュー
- `rust` - home-rust-code-review スキルによるRust特化レビュー（Rustコードの場合のみ）
- `design` - home-design-review スキルによる設計レビュー

reviewer名が上記のいずれにも一致しない場合は、エラーとしてユーザーに利用可能なreviewer名を案内してください。

## ステップ2: レビュー実行

各レビュアーは**修正を行わず、レビュー指摘の報告のみ**を行う。

- reviewer名が指定された場合: そのreviewerのエージェントを起動し、レビュー対象の差分情報を渡してコードレビューを実行する
- reviewer名が省略された場合: 対象コードの言語を判定し、適切なreviewerを**同時に並列起動**してレビューを実行する
  - Rustコードが含まれる場合: `code`, `simplify`, `codex`, `rust` を並列実行
  - それ以外: `code`, `simplify`, `codex` を並列実行

各レビュアーには指摘に連番を振らせる（例: code #1, simplify #1, rust #1）。これはステップ3での追跡に使用する。

## ステップ3: レビュー指摘の修正

すべてのレビューが完了したら、/home-fix-review-comments スキルを実行して、レビュー指摘に対応してください。

このスキルは以下を行う：
- 各指摘の妥当性を批判的に評価する（レビュアーの指摘がすべて正しいとは限らない）
- 妥当と判断した指摘のみ修正を実施する
- レビュアー識別子付きのサマリーを出力する（対応した指摘・対応しなかった指摘の両方）

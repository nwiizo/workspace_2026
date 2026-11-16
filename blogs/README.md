# ブログの執筆環境

このディレクトリで記事を書きます。原稿・画像・資料は`contents/`、
レビュー結果は`reviews/`、推敲前の稿や検査記録は`.work/`に置きます。
新しい記事は`contents/blog.md`から始められます。

## 記事を書く

このディレクトリからClaude CodeかCodexを起動します。

```sh
rtk proxy claude
rtk proxy codex --no-daemon
```

起動後の対話欄で、素材・対象読者・書きたい問いを添えて依頼します。

| 作業 | Claude Code | Codex |
|---|---|---|
| 素材から執筆・レビュー対応・推敲 | `/blog-harness` | `$blog-harness` |
| 人気記事との比較・指示の評価 | `/blog-harness-eval` | `$blog-harness-eval` |

例：「この素材から`contents/blog.md`を書き、独立レビューと推敲まで仕上げてください」。
新規記事・全面改稿は、執筆した側と異なるCLIでレビューします。
Codexで書いた記事はClaude Code、Claude Codeで書いた記事はCodexが確認します。
日本語の記事はyomiyasu・home-suiko・Textlintで仕上げます。

パンチライン・話の流れ・論証・深掘りなどは、[スキル一覧](docs/skills.md)から必要なものを選べます。
[レビュー](docs/review.md)、[仕上げ](docs/finishing.md)、[人気記事との比較](docs/evaluation.md)に手順があります。
文章の評価と、公開後のブックマーク数は分けて確認します。

## 配置と保守

| 場所 | 内容 |
|---|---|
| `contents/` | 原稿・画像・記事ごとの資料。連載は各READMEで整理 |
| `reviews/` | 記事ごとのレビュー結果と対応 |
| `.work/` | 推敲前の稿・検査記録などの作業データ |
| `docs/` | 著者情報・執筆手順・環境の保守 |
| `.agents/rules/` | 編集・評価のルールへのリンク |
| `.agents/skill-sources/` | 16スキルへのリンク |
| `.claude/rules/`・`.claude/skills/` | 同じルールとスキルをClaude Codeから使う入口 |

執筆・編集用の15スキルは[非公開のblog-harness](https://github.com/nwiizo/blog-harness)、
評価スキル・比較群・評価CLIは[非公開のblog-harness-eval](https://github.com/nwiizo/blog-harness-eval)で管理します。
このディレクトリに実体のコピーは置きません。
両クライアントが使うローカルの概要は[AGENTS.md](AGENTS.md)です。

Textlintの依存は既存のlockfileから復元できます。

```sh
rtk proxy npm ci --include=dev --ignore-scripts --no-audit --no-fund
rtk proxy npx --no-install textlint contents/blog.md
```

このワークスペースはREADME以外Gitの対象外です。
原稿・私用資料・実行ログを公開リポジトリへ追加しません。
再構成前のデータの退避先とリンクの確認方法は[環境の保守](docs/environment.md)にあります。

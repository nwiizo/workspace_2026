# ブログの執筆ワークスペース

原稿は `contents/`、編集の入口はローカルの `AGENTS.md` です。連載の構成は各ディレクトリのREADMEを参照します。以下の設定・資料はローカルに置き、Gitでは管理しません。

## ルールとSkill

ルール、Skill、編集資料の実体はこのワークスペースにあります。

- `.agents/rules/`：編集範囲、著者の好み、企画と反響
- `.agents/skill-sources/`：ホームから呼び出すSkillの実体
- `.agents/docs/skills.md`：Skillの用途一覧
- `docs/codex/finishing.md`：本文の検証と校正

ホームの `~/.agents/skills/` と `~/.claude/skills/` は同じSkillを参照します。設定や資料を直すときは `.agents/docs/skill-maintenance.md` を使います。媒体別のルールは設けません。

公開済み記事の履歴は別の `hatena-blog-pull` リポジトリで確認します。保存時点と公開日時を区別し、最近の記事が保存されていなければ公開ページから補います。編集中の記事へ一括取得を重ねません。

## 校正

依存関係が未導入の場合は、このワークスペースのlockfileから復元します。

```sh
rtk proxy npm ci --include=dev --ignore-scripts --no-audit --no-fund
rtk proxy suiko lint contents/blog.md --genre essay --json
rtk proxy npx --no-install textlint contents/blog.md
```

対象のパスとジャンルは記事に合わせます。Textlintはローカルの設定とprh.ymlを使います。書名を含む見出しはprhの置換対象外ですが、他のルールでは検査します。

このワークスペースはREADME以外Gitの対象外です。原稿と私用の記録を公開リポジトリへ追加せず、ignore設定を保ちます。

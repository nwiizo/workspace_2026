# Access Log ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** 情報漏洩
**目標:** サポートログディレクトリにアクセスする

---

## 実行手順

ブラウザで以下にアクセス:
```
http://localhost:3000/support/logs
```

## 解説

- アクセスログファイルが一覧表示される
- ログをダウンロードできる
- 本来は管理者だけがアクセスできるべき

**ログに含まれる情報:**
- IPアドレス
- アクセス日時
- リクエストURL
- ユーザーエージェント
- リファラー

**なぜ危険？**
- ユーザーの行動パターンが把握される
- 内部URLが漏洩する
- 攻撃者が攻撃計画を立てやすくなる

## 関連チャレンジ

- [Exposed Metrics](../difficulty-1/exposed-metrics.md)
- [Exposed Credentials](../difficulty-2/exposed-credentials.md)

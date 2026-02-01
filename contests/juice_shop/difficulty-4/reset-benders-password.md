# Reset Bender's Password ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** OSINT / セキュリティ質問の推測
**目標:** セキュリティ質問の答えを推測してパスワードをリセット

---

## 背景知識

### セキュリティ質問の脆弱性

「秘密の質問」や「セキュリティ質問」は、パスワードを忘れた際の本人確認手段として広く使われてきた。しかし、この方式には**根本的な問題**がある。

```
┌─────────────────────────────────────────────────────────────────┐
│                セキュリティ質問の問題点                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【よくある質問】                                                │
│  - 母親の旧姓は？                                               │
│  - 最初のペットの名前は？                                       │
│  - 出身の小学校は？                                             │
│  - 最初に働いた会社は？  ← Bender の質問                        │
│                                                                 │
│  【なぜ危険か】                                                  │
│  1. 推測可能: SNSや公開情報から特定できる                        │
│  2. 固定的: 答えは一生変わらない（変更できない）                 │
│  3. 共通: 複数サービスで同じ答えを使いがち                       │
│  4. 漏洩: 1つのサービスで漏洩すると他も危険                      │
│                                                                 │
│  【フィクションキャラクターの場合】                              │
│  - 設定資料や Wiki で答えが公開されている                       │
│  - ファンなら誰でも知っている情報                                │
│  → OSINT で簡単に特定可能                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### OSINT（Open Source Intelligence）とは

公開されている情報源から情報を収集・分析する手法。攻撃者は以下の情報源を活用する:

```
┌─────────────────────────────────────────────────────────────────┐
│                     OSINT の情報源                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【SNS】                                                        │
│  - Twitter/X: ペットの写真、出身地の投稿                        │
│  - Facebook: 家族構成、学歴、職歴                               │
│  - LinkedIn: 会社名、役職、経歴                                 │
│  - Instagram: 旅行先、趣味、家族                                │
│                                                                 │
│  【公的記録】                                                   │
│  - 結婚/離婚記録、不動産記録、会社登記                          │
│                                                                 │
│  【フィクションの場合】                                          │
│  - Wikipedia                                                    │
│  - Fandom Wiki（キャラクターWiki）                              │
│  - IMDb（映画/TVデータベース）                                  │
│  - 公式サイト、ファンサイト                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

有名人のファンクラブ会員が、その有名人になりすましてパスワードリセットを試みる。質問が「ペットの名前は？」なら、ファンなら誰でも知っている。Bender の場合も同じ。

---

## 思考プロセス

### ステップ1: ユーザーの背景を調査

```
「bender@juice-sh.op のセキュリティ質問に答えるには？」
    ↓
「Bender ってどんなキャラクター？」
    ↓
「Futurama という SF アニメのキャラクター」
    ↓
「Wikipedia や Fandom で情報を収集」
```

### ステップ2: セキュリティ質問を確認

```
「Forgot Password ページで質問を確認」
    ↓
「"Company you first worked for as an adult?"」
    ↓
「大人になって最初に働いた会社...」
    ↓
「Futurama Wiki で Bender の経歴を調べる」
```

### ステップ3: OSINT で答えを発見

```
「Bender の Wiki ページを確認」
    ↓
「Bender worked at Stop'n'Drop before being 'born' again...」
    ↓
「答えは Stop'n'Drop！」
```

---

## 実行手順

1. `http://localhost:3000/#/forgot-password` にアクセス
2. Email: `bender@juice-sh.op`
3. 質問: "Company you first worked for as an adult?"
4. 答え: `Stop'n'Drop`
5. 新しいパスワードを設定

### OSINT 調査の流れ

```
1. Google 検索: "Bender Futurama first job"
    ↓
2. Futurama Wiki にアクセス
    ↓
3. Bender のキャラクター設定を確認
    ↓
4. "Stop'n'Drop" という会社名を発見
    ↓
5. セキュリティ質問の答えとして使用
```

---

## Futurama と Bender について

```
┌─────────────────────────────────────────────────────────────────┐
│                     Bender Bending Rodríguez                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【作品】 Futurama (1999-2023)                                  │
│  【制作】 Matt Groening (シンプソンズの作者)                    │
│                                                                 │
│  【キャラクター設定】                                            │
│  - 型番: Bending Unit 22                                        │
│  - 職業: 金属を曲げるロボット                                   │
│  - 趣味: 酒、タバコ、犯罪                                       │
│  - 決め台詞: "Bite my shiny metal ass!"                         │
│                                                                 │
│  【職歴】                                                       │
│  - Stop'n'Drop (最初の職場) ← セキュリティ質問の答え            │
│  - Planet Express (現在の職場)                                  │
│                                                                 │
│  【Juice Shop での他の登場】                                     │
│  - Login Bender: SQLi でログイン                                │
│  - パスワード: OhG0dPlease1LubYou                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Juice Shop の脆弱なコードパターン

### 脆弱なコード（推定）

```typescript
// ❌ 脆弱なコード
// routes/resetPassword.ts
export function resetPassword() {
  return async (req: Request, res: Response) => {
    const { email, securityAnswer, newPassword } = req.body

    const user = await UserModel.findOne({ where: { email } })
    if (!user) {
      return res.status(404).json({ error: 'User not found' })
    }

    // ❌ 問題1: 回答の照合が単純な文字列比較
    if (user.securityAnswer.toLowerCase() === securityAnswer.toLowerCase()) {
      user.password = hash(newPassword)
      await user.save()
      return res.json({ status: 'success' })
    }

    // ❌ 問題2: レートリミットがない
    // ❌ 問題3: ロックアウト機構がない
    res.status(401).json({ error: 'Wrong answer' })
  }
}
```

### 問題点

1. **推測可能な質問**: 答えが公開情報から特定できる
2. **ブルートフォース対策なし**: 何度でも試行可能
3. **追加認証なし**: セキュリティ質問だけでリセット可能

---

## 安全な実装

```typescript
// ✅ 安全なコード
// routes/resetPassword.ts
export function resetPassword() {
  return async (req: Request, res: Response) => {
    const { email } = req.body

    const user = await UserModel.findOne({ where: { email } })
    if (!user) {
      // タイミング攻撃対策: 存在しなくても同じレスポンス
      return res.json({ message: 'If the email exists, a reset link will be sent' })
    }

    // 1. 一時的なリセットトークンを生成
    const resetToken = crypto.randomBytes(32).toString('hex')
    const hashedToken = crypto.createHash('sha256').update(resetToken).digest('hex')

    // 2. トークンを保存（有効期限付き）
    await ResetTokenModel.create({
      userId: user.id,
      token: hashedToken,
      expiresAt: new Date(Date.now() + 15 * 60 * 1000)  // 15分
    })

    // 3. メールでリセットリンクを送信
    await sendEmail({
      to: email,
      subject: 'Password Reset',
      body: `Click here to reset: https://example.com/reset?token=${resetToken}`
    })

    res.json({ message: 'If the email exists, a reset link will be sent' })
  }
}
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **メールベースリセット** | セキュリティ質問ではなくメールで確認 |
| ✅ **一時的トークン** | 推測不能なランダムトークンを生成 |
| ✅ **有効期限** | トークンは15-60分で期限切れ |
| ✅ **ワンタイム** | トークンは1回使用したら無効化 |
| ✅ **レートリミット** | リセット要求の回数を制限 |
| ✅ **MFA連携** | 可能なら2要素認証も要求 |

---

## Bender の完全な認証情報

```
メール: bender@juice-sh.op
パスワード: OhG0dPlease1LubYou
パスワードハッシュ: 0c36e517e3fa95aabf1bbffc6744a4ef
セキュリティ質問: Company you first worked for as an adult?
セキュリティ回答: Stop'n'Drop
```

---

## 他の Juice Shop キャラクターのセキュリティ質問

| ユーザー | 質問 | 答え | 情報源 |
|---------|------|------|--------|
| jim@juice-sh.op | 兄弟の名前 | Samuel | Star Trek (George Samuel Kirk) |
| bjoern@owasp.org | ペットの名前 | Zaya | 本人の SNS |
| uvogin@juice-sh.op | 好きな映画 | Silence of the Lambs | Hunter x Hunter の設定 |
| emma@juice-sh.op | 最初の勤務先 | ITsec | 公開プロフィール |

---

## OSINT ツール

| ツール | 用途 |
|--------|------|
| **Google Dorks** | `site:twitter.com "ペットの名前"` など |
| **Shodan** | インターネット接続デバイスの検索 |
| **theHarvester** | メールアドレス、サブドメイン収集 |
| **Maltego** | 関連情報のビジュアル化 |
| **recon-ng** | 偵察フレームワーク |

---

## OWASP との関連

- **A07:2021 - Identification and Authentication Failures**: 弱い認証メカニズム

---

## 関連チャレンジ

- [Login Bender](../difficulty-3/login-bender.md) - SQLi でログイン
- [Bjoern's Favorite Pet](../difficulty-3/bjoerns-favorite-pet.md) - 写真から推測
- [Reset Jim's Password](../difficulty-3/reset-jims-password.md) - Star Trek ファンの質問

## 参考リンク

- [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)
- [Futurama Wiki - Bender](https://futurama.fandom.com/wiki/Bender_Bending_Rodr%C3%ADguez)
- [NIST SP 800-63B - Authentication Guidelines](https://pages.nist.gov/800-63-3/sp800-63b.html)

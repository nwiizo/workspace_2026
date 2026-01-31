# Leaked API Key ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Sensitive Data Exposure
**目標:** 漏洩した API キーを Contact フォームで報告する

## ソースコード分析

### チャレンジ検証

**ファイル:** `routes/verify.ts` (lines 415-423)

```typescript
function leakedApiKeyChallenge () {
  FeedbackModel.findAndCountAll({
    where: { comment: { [Op.like]: '%6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie%' } }
  }).then(({ count }: { count: number }) => {
    if (count > 0) {
      challengeUtils.solve(challenges.leakedApiKeyChallenge)
    }
  })

  ComplaintModel.findAndCountAll({
    where: { message: { [Op.like]: '%6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie%' } }
  }).then(({ count }: { count: number }) => {
    if (count > 0) {
      challengeUtils.solve(challenges.leakedApiKeyChallenge)
    }
  })
}
```

### API キー

```
6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie
```

## 実行手順

### Step 1: Contact フォームで報告

```javascript
// CAPTCHA を取得
const captchaRes = await fetch('/rest/captcha').then(r => r.json());

// フィードバックを送信
await fetch('/api/Feedbacks', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    captchaId: captchaRes.captchaId,
    captcha: captchaRes.answer,
    comment: '6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie',
    rating: 1
  })
});
```

### 代替: Complaint フォームで報告

```javascript
const formData = new FormData();
formData.append('message', '6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie');

fetch('/api/Complaints', {
  method: 'POST',
  headers: {
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: formData
});
```

## 公式ヒント

チャレンジのヒント:

1. ソーシャルメディアチャンネルで定期的に投稿されるコンテンツを確認
2. API レスポンスから取得されるコンテンツを特定
3. API 呼び出しが行われる場所を特定 (Web アプリ内ではない)
4. juicy-coupon-bot リポジトリを調査

### 発見方法 (想定される解法)

1. **juicy-coupon-bot リポジトリ**を調査
   - https://github.com/juice-shop/juicy-coupon-bot
   - ソーシャルメディアへの自動投稿を行うボット

2. **コミット履歴**を確認
   - 過去のコミットで認証情報が露出している可能性

3. **Travis CI ログ**を確認
   - ビルドログに誤って出力された認証情報

## 解説

### API キー漏洩とは？

**日常的な例えで説明すると:**

家の鍵を SNS にうっかり投稿してしまった状況。

- 投稿を削除しても、スクリーンショットを取られていたら意味がない
- 鍵を交換するまで、誰でも家に入れる

API キーも同じ。一度公開されたら「削除」では解決しない。

### なぜ「削除」では解決しないのか？

```
┌─────────────────────────────────────────────────────┐
│           Git の履歴は「削除」できない               │
├─────────────────────────────────────────────────────┤
│                                                     │
│  コミット1: 機能追加                                │
│  コミット2: API キーをハードコード (ここで漏洩!)    │
│  コミット3: API キーを削除 ← これでは遅い!          │
│                                                     │
│  攻撃者: git log -p で過去のコミットを見れる        │
│         → コミット2 から API キーを取得              │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### シークレットが漏洩する経路

```
┌─────────────────────────────────────────────────────┐
│           よくある漏洩パターン                       │
├─────────────────────────────────────────────────────┤
│                                                     │
│  1. ソースコードにハードコード                      │
│     const API_KEY = "abc123...";                   │
│                                                     │
│  2. 設定ファイルをコミット                          │
│     .env, config.json を .gitignore し忘れ        │
│                                                     │
│  3. CI/CD ログに出力                               │
│     console.log("API_KEY:", process.env.API_KEY); │
│                                                     │
│  4. エラーメッセージに含まれる                      │
│     Error: Invalid API key: abc123...              │
│                                                     │
│  5. Stack Overflow に質問                          │
│     「このコードがエラーになります」+ キー含む       │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### このチャレンジのシナリオ

```
1. Juice Shop の開発者が別リポジトリ (juicy-coupon-bot) を作成
2. API キーをハードコードしてコミット
3. 後で削除したが、Git 履歴に残っている
4. 攻撃者がコミット履歴を調査して発見
5. 発見した API キーを Contact フォームで報告
```

### 根本原因

**「シークレットは一度公開されたら取り消せない」**

| 行動 | 結果 |
|------|------|
| コードに書いてコミット | Git 履歴に永久保存 |
| CI ログに出力 | ログが消えるまで残る |
| Stack Overflow に投稿 | Internet Archive に残る |

### 正しい対応

| ステップ | 説明 |
|----------|------|
| 1. **即座に無効化** | API プロバイダーでキーを失効させる |
| 2. **新しいキーを発行** | 古いキーは二度と使わない |
| 3. **環境変数に移行** | コードにシークレットを書かない |
| 4. **git filter-branch** | 必要なら履歴から削除（非推奨） |

### 予防策

```bash
# 1. .gitignore で設定ファイルを除外
echo ".env" >> .gitignore

# 2. pre-commit フックでスキャン
git secrets --install
git secrets --scan

# 3. 環境変数を使う
API_KEY=${{ secrets.API_KEY }}
```

### 教訓

**「シークレットがコードに入った時点でアウト」**

- 削除しても手遅れ
- 気づいたら即座にキーを無効化
- 最初から環境変数を使う習慣をつける

### 対策

```bash
# 1. 環境変数を使用
export API_KEY=${{ secrets.API_KEY }}

# 2. .gitignore で設定ファイルを除外
echo ".env" >> .gitignore

# 3. git-secrets でスキャン
git secrets --scan

# 4. 漏洩した場合はキーをローテーション
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `routes/verify.ts:415-423` | チャレンジ検証 |
| `data/static/challenges.yml` | チャレンジ定義 |

## Playwright MCP での実行

### 方法1: API 直接呼び出し (推奨)

UIの制約を回避して直接APIを呼び出す方法:

```javascript
// browser_evaluate を使用
async () => {
  // 1. CAPTCHAを取得
  const captchaRes = await fetch('/rest/captcha').then(r => r.json());

  // 2. フィードバックを送信
  const feedbackRes = await fetch('/api/Feedbacks', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      captchaId: captchaRes.captchaId,
      captcha: captchaRes.answer,
      comment: '6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie',
      rating: 1
    })
  });

  return {
    status: feedbackRes.status,
    body: await feedbackRes.json()
  };
}
// 結果: { status: 201, body: { status: "success", data: {...} } }
```

### 方法2: UI操作

```javascript
// 1. Contact ページにアクセス
mcp__playwright__browser_navigate({ url: "http://localhost:3000/#/contact" });

// 2. スナップショットで ref を確認
mcp__playwright__browser_snapshot();

// 3. API キーをコメントに入力
mcp__playwright__browser_type({
  ref: "e3060",  // Comment フィールドの ref
  text: "6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie",
  element: "Comment field"
});

// 4. CAPTCHA 問題を取得
mcp__playwright__browser_evaluate({
  function: "() => document.body.innerText.match(/What is\\s+([\\d\\s\\+\\-\\*\\/]+)\\s*\\?/)[1]"
});
// 例: "6-9*3" → 答えは -21

// 5. CAPTCHA 答えを入力
mcp__playwright__browser_type({
  ref: "e3080",  // CAPTCHA Result フィールドの ref
  text: "-21",
  element: "CAPTCHA result field"
});

// 6. Submit ボタンをクリック
// 注意: UI ではボタンが disabled のままの場合があるため、方法1 を推奨
```

### 実行時の注意点

- **UI 制約**: Angular Material の Radio ボタンやフォームバリデーションが原因で Submit ボタンが有効化されないことがある
- **推奨**: `browser_evaluate` で API を直接呼び出す方法が確実
- **CAPTCHA バイパス**: `/rest/captcha` から CAPTCHA ID と答えを取得可能

## 参考リンク

- [GitHub - juicy-coupon-bot](https://github.com/juice-shop/juicy-coupon-bot)
- [OWASP Secrets Management](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [GitLeaks](https://github.com/gitleaks/gitleaks)

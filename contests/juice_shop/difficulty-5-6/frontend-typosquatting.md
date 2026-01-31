# Frontend Typosquatting ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Vulnerable Components
**目標:** フロントエンドにあるtyposquattingされた依存関係を発見し報告

## ソースコード分析

### 脆弱なパッケージ

**ファイル:** `frontend/package.json` (line 55)

```json
{
  "dependencies": {
    "ngy-cookie": "^6.0.0"
  }
}
```

**正規パッケージ**: `ngx-cookie` (Angular Cookie Service)
**typosquatted**: `ngy-cookie` (ngy vs ngx)

### インポート箇所

**ファイル:** `frontend/src/main.ts` (line 65)

```typescript
import { CookieService, CookieModule } from 'ngy-cookie';
```

### チャレンジ検証

**ファイル:** `routes/verify.ts` (line 307)

```typescript
FeedbackModel.findAndCountAll({
  where: { comment: { [Op.like]: '%ngy-cookie%' } }
}).then(({ count }: { count: number }) => {
  if (count > 0) {
    challengeUtils.solve(challenges.typosquattingAngularChallenge)
  }
})
```

## 実行手順

### Step 1: Contact フォームでパッケージ名を報告

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
    comment: 'You are a typosquatting victim of this NPM package: ngy-cookie',
    rating: 1
  })
});
```

### Step 2: チャレンジ解決確認

```javascript
fetch('/api/Challenges')
  .then(r => r.json())
  .then(data => {
    const challenge = data.data.find(c => c.key === 'typosquattingAngularChallenge');
    console.log('Solved:', challenge.solved);
  });
```

## Playwright MCP での実行

```javascript
// 1. Contact ページにアクセス
browser_navigate({ url: "http://localhost:3000/#/contact" });

// 2. コメントを入力
browser_type({
  ref: "comment入力欄",
  text: "ngy-cookie"
});

// 3. 評価を設定
browser_click({ ref: "1つ星" });

// 4. CAPTCHA を解く (ページで計算結果を入力)
// 5. 送信
browser_click({ ref: "Submit" });
```

## 解説

### Typosquatting とは？

**日常的な例えで説明すると:**

ブランドの偽物バッグを想像してください。

```
正規: GUCCI
偽物: CUCCI (Gが Cに)
```

npm パッケージでも同じことが起きている。

```
正規: ngx-cookie
偽物: ngy-cookie (xが yに)
```

### なぜ気づかないのか？

```
┌─────────────────────────────────────────────────────┐
│              キーボード配列                          │
├─────────────────────────────────────────────────────┤
│                                                     │
│     Q W E R T [Y] U I O P                          │
│      A S D F G H J K L                              │
│       Z [X] C V B N M                               │
│                                                     │
│  X と Y は近い位置 → タイプミスしやすい              │
└─────────────────────────────────────────────────────┘
```

さらに、目で見ても区別しにくい:

```
ngx-cookie   ← 正規
ngy-cookie   ← 偽物
^^^
この部分だけ見ても、x と y の違いは一瞬では分からない
```

### サプライチェーン攻撃の流れ

```
1. 攻撃者: npm に「ngy-cookie」を公開
   ├─ postinstall スクリプトで悪意のあるコード実行
   └─ 環境変数 (API キー等) を外部に送信

2. 開発者: package.json に追加
   └─ 「ngx-cookie」のつもりで「ngy-cookie」をタイプ

3. npm install 実行
   └─ 偽パッケージがインストールされる

4. postinstall 実行
   └─ 攻撃コードが実行される!

5. 被害拡大
   ├─ CI/CD でも実行
   ├─ 本番環境にデプロイ
   └─ 全ユーザーに影響
```

### なぜ危険か？

| リスク | 説明 |
|--------|------|
| **自動実行** | `npm install` だけで悪意のあるコードが動く |
| **権限昇格** | 開発者のマシンで開発者の権限で実行 |
| **連鎖感染** | そのパッケージを使う全プロジェクトに影響 |
| **検出困難** | 大量の依存関係の中から見つけるのは困難 |

### 実際の例

```json
{
  "dependencies": {
    "react": "^18.0.0",
    "redux": "^4.0.0",
    "axios": "^1.0.0",
    "lodash": "^4.17.0",
    "moment": "^2.29.0",
    "ngy-cookie": "^6.0.0",  // ← ここに偽物!
    "express": "^4.18.0",
    // ... 100行以上続く
  }
}
```

人間のコードレビューで発見できますか？

### 根本原因

1. **人間の認知限界**: 似た文字列を区別するのが苦手
2. **npm の信頼モデル**: 「パッケージ名を知っている = 使いたい」と仮定
3. **自動化の罠**: `npm install` が全てを自動でやる

### 対策

| 対策 | 説明 |
|------|------|
| **npm audit** | 既知の悪意あるパッケージをスキャン |
| **lockfile レビュー** | 新しい依存関係を Git で確認 |
| **Snyk / Dependabot** | 自動で脆弱な依存を検出 |
| **パッケージ名の確認** | 追加前に npm で正式名を確認 |

```bash
# 追加前に確認
npm info ngx-cookie  # 正式名を確認
npm info ngy-cookie  # 怪しいパッケージは情報が少ない
```

### 対策

```bash
# 1. パッケージ名を慎重に確認
npm info ngy-cookie  # パッケージ情報を確認

# 2. npm audit を実行
npm audit

# 3. Snyk や Dependabot で監視
snyk test

# 4. ロックファイルをレビュー
git diff package-lock.json
```

## 関連チャレンジ

### Legacy Typosquatting (難易度4)

**ファイル:** `ftp/package.json.bak` (line 46)

```json
{
  "dependencies": {
    "epilogue-js": "~0.7"
  }
}
```

**正規**: `epilogue` → **Typosquatted**: `epilogue-js`

報告方法:
```javascript
comment: 'epilogue-js'
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `frontend/package.json:55` | typosquatted パッケージ |
| `frontend/src/main.ts:65` | インポート箇所 |
| `routes/verify.ts:307` | チャレンジ検証 |
| `ftp/package.json.bak:46` | Legacy typosquatting |

## 参考リンク

- [npm Typosquatting](https://snyk.io/blog/typosquatting-attacks/)
- [OWASP Dependency Check](https://owasp.org/www-project-dependency-check/)

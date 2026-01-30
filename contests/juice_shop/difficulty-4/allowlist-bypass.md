# Allowlist Bypass ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** バイパス
**目標:** リダイレクト制限を回避して任意のサイトに誘導する

---

## 実行手順

ブラウザで以下にアクセス:
```
http://localhost:3000/redirect?to=https://evil.com?x=https://github.com/juice-shop/juice-shop
```

## 解説

**許可リストのチェック:**
- サーバーは `to` パラメータに許可されたドメインが含まれているかチェック
- `github.com/juice-shop` は許可リストに入っている
- URLの一部に含まれていれば検証をパス

**バイパスの仕組み:**
```
https://evil.com?x=https://github.com/juice-shop/juice-shop
^^^^^^^^^^^^^^^ 実際のリダイレクト先
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 許可リストマッチ用
```

- `evil.com` にリダイレクトされる
- クエリパラメータに許可ドメインを含めることで検証をバイパス

**なぜ危険？**
- オープンリダイレクト攻撃
- フィッシングサイトへの誘導
- 信頼されたドメインからのリンクに見せかける

## 関連チャレンジ

- [Outdated Allowlist](../difficulty-1/outdated-allowlist.md)

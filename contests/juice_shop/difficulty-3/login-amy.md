# Login Amy ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 認証
**目標:** Amyとしてログインする（パスワード推測）

---

## 背景

- Amy は歌詞の一部をパスワードに使っている
- "K1f" + ピリオドの連続（歌詞の省略を表す）
- 歌詞: "Kif, crouch down here and shield my thighs from the cold ..."

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email: `amy@juice-sh.op`
3. Password: `K1f.....................` (K1f + 21個のピリオド = 計24文字)
4. ログインできれば成功

## 解説

**パスワードの構造:**
```
K1f.....................
^^^ ^^^^^^^^^^^^^^^^^^
K1f + 21個のピリオド = 24文字
```

**ヒントの発見方法:**
- Amyのプロフィールやレビューを調査
- 歌詞の引用がヒントになっている
- "..." は歌詞の省略を表す

**教訓:**
- パスワードに個人的な情報や趣味を使うのは危険
- 攻撃者はOSINTで個人情報を収集する

## 関連チャレンジ

- [Login Jim](login-jim.md)
- [Login MC SafeSearch](../difficulty-2/login-mc-safesearch.md)

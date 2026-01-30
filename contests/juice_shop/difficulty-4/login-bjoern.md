# Login Bjoern (Gmail) ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** 認証
**目標:** Bjoernのパスワードを解読してログインする

---

## 思考プロセス

**ステップ1: ヒントを探す**
```
「Bjoern Kimminich は Juice Shop の作者」
    ↓
「彼の Gmail アカウントでログインするチャレンジ」
    ↓
「パスワードに何かパターンがあるはず」
```

**ステップ2: 逆発想**
```
「メールアドレス自体がヒントかも？」
    ↓
「bjoern.kimminich@gmail.com」
    ↓
「これを逆順にしたら？」
    ↓
「moc.liamg@hcinimmik.nreojb」
```

**ステップ3: Base64エンコード**
```
「Web でよく使われるエンコード = Base64」
    ↓
「逆順の文字列を Base64 エンコード」
    ↓
「bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=」
```

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email: `bjoern.kimminich@gmail.com`
3. Password: `bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=`
4. ログインできれば成功

## コード/ペイロード

```javascript
// メールを逆順に
const email = 'bjoern.kimminich@gmail.com';
const reversed = email.split('').reverse().join('');
// → moc.liamg@hcinimmik.nreojb

// Base64エンコード
const password = btoa(reversed);
// → bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=
```

## 解説

**なぜこのパターン？**
- 作者のジョークとして、自分のメアドを逆順+Base64
- 一見ランダムに見えるが、ロジックがある
- 「隠蔽による安全」は危険という教訓

## 関連チャレンジ

- [Login Jim](../difficulty-3/login-jim.md)
- [Bjoern's Favorite Pet](../difficulty-3/bjoerns-favorite-pet.md)

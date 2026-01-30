# Login MC SafeSearch ✅

**難易度:** ⭐⭐
**カテゴリ:** OSINT
**目標:** 公開情報からパスワードを見つけてログインする

---

## 思考プロセス

**ステップ1: キャラクターを調査**
```
「MC SafeSearch という名前」
    ↓
「Google で検索してみる」
    ↓
「YouTube に "Protect Ya Passwordz" という動画がある！」
    ↓
「この動画の中でパスワードを歌っている」
```

**ステップ2: 動画から情報を収集**
```
「動画を視聴」
    ↓
「"My password used to be 'password', now it's 'Mr. N00dles'"」
    ↓
「パスワードは Mr. N00dles だ！」
```

**ステップ3: OSINT とは？**
```
「Open Source INTelligence = 公開情報調査」
    ↓
「SNS、動画、公開プロフィールから情報を収集」
    ↓
「実際の攻撃でも使われる手法」
```

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email: `mc.safesearch@juice-sh.op`
3. Password: `Mr. N00dles`
4. ログインできれば成功

## 解説

- このパスワードはYouTube動画で公開されている
- OSINT = Open Source Intelligence（公開情報調査）
- SNSや動画など、公開されている情報から個人情報を特定する手法

**OSINTの例:**
- SNSプロフィールからセキュリティ質問の答えを推測
- 公開された写真から位置情報を特定
- LinkedIn から勤務先や経歴を把握

## 関連チャレンジ

- [Meta Geo Stalking](meta-geo-stalking.md)
- [Visual Geo Stalking](visual-geo-stalking.md)
- [Bjoern's Favorite Pet](../difficulty-3/bjoerns-favorite-pet.md)

# Token Sale ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 隠蔽
**目標:** 隠された Token Sale (ICO) ページにアクセス

## ヒント

- **URL:** `/#/tokensale-ico-ea`
- **発見方法:**
  - ソースマップ (`main.js.map`) を解析
  - ルーティング設定から発見
- **関連:** Blockchain Hype チャレンジ

## 調査方法

```javascript
// DevTools Console でルート一覧を確認
// Angular の Router を調べる
```

## ソースマップ解析

```bash
# main.js.map をダウンロード
curl -o main.js.map http://localhost:3000/main.js.map

# 内容を検索
grep -i "tokensale\|ico" main.js.map
```

## 直接アクセス

```
http://localhost:3000/#/tokensale-ico-ea
```

## 検証ポイント

- [ ] URL にアクセスできるか
- [ ] Token Sale ページが表示されるか
- [ ] チャレンジが完了したか

## 解説

[未着手]

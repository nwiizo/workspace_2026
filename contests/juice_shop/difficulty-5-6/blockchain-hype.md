# Blockchain Hype ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 隠蔽
**目標:** 隠されたトークンセールページを見つける

---

## 思考プロセス

**ステップ1: 隠しページの存在を推測**
```
「Juice Shopは色々な機能を隠している」
    ↓
「スコアボード、管理画面など...」
    ↓
「ブロックチェーン/ICO関連のページもあるのでは？」
    ↓
「JavaScript内にルート情報が含まれているはず」
```

**ステップ2: ソースコードを調査**
```
「DevTools → Sources → main.js を開く」
    ↓
「Ctrl+F で 'token', 'ico', 'blockchain' を検索」
    ↓
「'tokensale' というルートを発見」
    ↓
「難読化されているが、URL部分は読める」
```

## 実行手順

1. `F12` → Sources タブで `main.js` を開く
2. `Ctrl+F` で `tokensale` を検索
3. 難読化されたルートを発見
4. ブラウザでアクセス:
   ```
   http://localhost:3000/#/tokensale-ico-ea
   ```
5. ICO（Initial Coin Offering）ページが表示されれば成功

## 解説

**なぜ "Security through Obscurity" は危険？**
```
「URLを隠しても、ソースコードに痕跡が残る」
    ↓
「誰でもDevToolsでJavaScriptを読める」
    ↓
「適切なアクセス制御がないと、見つかったら誰でもアクセス可能」
```

**ICO（Initial Coin Offering）とは:**
- 暗号通貨を使った資金調達方法
- トークンを発行して投資家から資金を集める
- 詐欺的なICOも多く、注意が必要

## 関連チャレンジ

- [Score Board](../difficulty-1/score-board.md)
- [Web3 Sandbox](../difficulty-1/web3-sandbox.md)

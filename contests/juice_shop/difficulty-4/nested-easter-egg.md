# Nested Easter Egg ❌

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** その他
**目標:** イースターエッグの中にある、さらなる隠しコンテンツを発見する

---

## 思考プロセス

**ステップ1: Easter Egg チャレンジからの継続**
```
「Easter Egg チャレンジで隠しページを発見した」
    ↓
「/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg」
    ↓
「"within the easter egg" = イースターエッグの中にもう一つある？」
    ↓
「このページのソースコードや隠し要素を調べよう」
```

**ステップ2: 隠しコンテンツを探す**
```
「ページにアクセスすると Planet Orangeuze が表示される」
    ↓
「このページのHTML/CSS/JSを確認」
    ↓
「隠し要素（display:none, visibility:hidden）がないか？」
    ↓
「Base64やエンコードされた文字列がないか？」
```

**ステップ3: DevToolsで詳細調査**
```
「Elements タブでHTML構造を確認」
    ↓
「Network タブで読み込まれるリソースを確認」
    ↓
「Console でエラーやログを確認」
    ↓
「Sources タブでJSファイルを確認」
```

## 前提条件

- Easter Egg チャレンジが完了していること
- 隠しページ `/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg` にアクセス済み

## 調査ポイント

1. **ページ内のテキスト/画像**
   - 目に見えない要素はないか
   - 画像に隠されたデータ（ステガノグラフィ）

2. **ソースコード**
   - コメントに隠されたヒント
   - Base64エンコードされた文字列
   - 難読化されたJavaScript

3. **HTTPレスポンス**
   - カスタムヘッダー
   - Cookie に隠された情報

## 実行手順

1. 隠しページにアクセス:
   ```
   http://localhost:3000/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg
   ```

2. DevTools を開いて調査:
   ```javascript
   // ページ内の全テキストを確認
   document.body.innerText
   
   // 隠し要素を確認
   document.querySelectorAll('[style*="display:none"], [style*="visibility:hidden"]')
   
   // data 属性を確認
   document.querySelectorAll('[data-*]')
   ```

3. ページソースを確認:
   - Ctrl+U でソースを表示
   - コメント `<!-- -->` を検索
   - Base64 らしき文字列を検索

## 検証ポイント

- [ ] Easter Egg ページのソースを詳細に確認
- [ ] 隠し要素やコメントを発見
- [ ] エンコードされた文字列をデコード
- [ ] 新しいURLやコンテンツを発見

## 関連チャレンジ

- [Easter Egg](easter-egg.md) - このチャレンジの前提
- [Score Board](../difficulty-1/score-board.md) - 隠しページの発見

## 解説

[未着手]

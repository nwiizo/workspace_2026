# Deprecated Interface ✅

**難易度:** ⭐⭐
**カテゴリ:** 設定ミス
**目標:** XMLファイルをアップロードする（本来はPDF/ZIPのみ）

---

## 思考プロセス

**ステップ1: ファイルアップロード機能を探す**
```
「苦情ページ /#/complain にファイルアップロード欄がある」
    ↓
「PDF と ZIP しか選択できない」
    ↓
「でもこれはフロントエンドの制限...」
```

**ステップ2: HTML属性を調査**
```
「Elements タブで input 要素を確認」
    ↓
「accept=".pdf,.zip" という属性を発見」
    ↓
「これを変更すれば他の形式もアップロードできる？」
```

**ステップ3: 制限を回避**
```
「Console で accept 属性を変更」
    ↓
「document.querySelector('input[type="file"]').accept = '.xml'」
    ↓
「XMLファイルがアップロード可能になった！」
    ↓
「これで XXE 攻撃への道が開ける」
```

## 実行手順

1. `http://localhost:3000/#/complain` にアクセス
2. `F12` → Console で以下を実行:
   ```javascript
   document.querySelector('input[type="file"]').accept = '.xml'
   ```
3. 任意のXMLファイルをアップロード

## 解説

**なぜ "Deprecated Interface" ？**
```
「XMLアップロード機能は廃止されたはずだった」
    ↓
「でもサーバー側のコードは残っている」
    ↓
「フロントエンドで隠しただけでは安全ではない」
```

- フロントエンドの制限は簡単に回避できる
- 本当のセキュリティはサーバー側で実装する必要がある

## 関連チャレンジ

- [XXE Data Access](../difficulty-3/xxe-data-access.md)
- [Upload Size](../difficulty-3/upload-size.md)
- [Upload Type](../difficulty-3/upload-type.md)

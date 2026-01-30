# Extra Language ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 自動化
**目標:** 隠されたKlingon（クリンゴン語）言語ファイルにアクセスする

---

## 実行手順

ブラウザで直接アクセス:
```
http://localhost:3000/assets/i18n/tlh_AA.json
```

または Console で:
```javascript
fetch('/assets/i18n/tlh_AA.json')
  .then(r => r.json())
  .then(data => console.log(data));
```

## 解説

**tlh_AA とは？**
- `tlh` = Klingon（クリンゴン語）の言語コード
- スタートレックに登場する架空の言語
- 通常の言語セレクターには表示されない隠し言語

**ファイルの内容例:**
```json
{
  "LANGUAGE": "tlhIngan",
  "NAV_SEARCH": "tu'",
  ...
}
```

**発見方法:**
- i18n（国際化）ディレクトリのパターンを推測
- 言語コードの一覧を試行
- ソースコードから参照を発見

**なぜ隠されている？**
- イースターエッグとして開発者が仕込んだ
- ファンサービス（スタートレックファン向け）
- 隠蔽による保護は不十分という教訓

## 関連チャレンジ

- [Score Board](../difficulty-1/score-board.md)
- [Easter Egg](../difficulty-4/easter-egg.md)

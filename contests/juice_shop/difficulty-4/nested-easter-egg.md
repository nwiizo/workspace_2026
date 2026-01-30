# Nested Easter Egg ❌

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** その他
**目標:** 多重エンコードされたイースターエッグを解読

## ヒント

- **ソース:** `/ftp/eastere.gg` (Poison Null Byte でアクセス)
- **URL:** `/ftp/eastere.gg%2500.md`
- **エンコード:** Base64 → ROT13 の順でデコード

## デコード手順

```bash
# 1. ファイル取得
curl "http://localhost:3000/ftp/eastere.gg%2500.md"

# 2. Base64 デコード
echo "エンコードされた文字列" | base64 -d

# 3. ROT13 デコード
echo "Base64デコード結果" | tr 'A-Za-z' 'N-ZA-Mn-za-m'
```

## オンラインツール

- Base64: https://www.base64decode.org/
- ROT13: https://rot13.com/

## eastere.gg の内容（予想）

```
Base64エンコードされた文字列
↓ Base64デコード
ROT13エンコードされた文字列
↓ ROT13デコード
隠されたメッセージ or URL
```

## 検証ポイント

- [ ] Poison Null Byte で eastere.gg を取得
- [ ] Base64 デコード成功
- [ ] ROT13 デコードで意味のある文字列

## 解説

[未着手]

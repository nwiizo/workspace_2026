# Forged Coupon ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** 暗号
**目標:** 有効なクーポンコードを偽造する

## ヒント

- **エンコーディング:** Z85 (ZeroMQ Base-85)
- **クーポン形式:** `MMMYY-XX` → Z85エンコード
  - MMM: 月の略称 (JAN, FEB, MAR...)
  - YY: 年 (24, 25, 26...)
  - XX: 割引率 (10, 20, 50, 90...)

## Z85 エンコーディング

```
Z85 は ZeroMQ プロジェクトで使用される Base85 エンコーディング
バイナリデータを ASCII 文字に変換
```

## 手順

1. 有効なクーポン形式を推測: `JAN26-90`
2. Z85 でエンコード
3. チェックアウト時に適用

## ツール

- https://cryptii.com/pipes/z85-encoder
- Python: `pip install z85`

```python
import z85
coupon = "JAN26-90"
encoded = z85.encode(coupon.encode())
print(encoded)
```

## 既知のクーポン分析

```bash
# coupons_2013.md.bak を取得
curl "http://localhost:3000/ftp/coupons_2013.md.bak%2500.md"

# 内容からパターンを分析
```

## 検証ポイント

- [ ] 過去のクーポンのパターンを分析
- [ ] Z85 エンコード/デコードを確認
- [ ] 偽造クーポンが受け入れられるか

## 解説

[未着手]

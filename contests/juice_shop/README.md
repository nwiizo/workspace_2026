# OWASP Juice Shop CTF ツールガイド

チャレンジを解くのに役立つツールを紹介します。全て無料で使えます。

---

## Juice Shop のセットアップ

### Docker を使う方法（推奨）

```bash
# Juice Shop を起動
docker run -d -p 3000:3000 bkimminich/juice-shop

# ブラウザでアクセス
open http://localhost:3000
```

### Node.js を使う方法

```bash
# リポジトリをクローン
git clone https://github.com/juice-shop/juice-shop.git
cd juice-shop

# 依存関係をインストール
npm install

# 起動
npm start

# ブラウザでアクセス
open http://localhost:3000
```

### 動作確認

1. http://localhost:3000 にアクセス
2. Juice Shop のトップページが表示されればOK
3. まず `http://localhost:3000/#/score-board` にアクセスしてスコアボードを開く

---

## 必須ツール

### 1. ブラウザ開発者ツール（DevTools）
**用途:** ほぼ全てのチャレンジで使用

- **開き方:** `F12` キーを押す
- **Console タブ:** JavaScript コードを実行
- **Network タブ:** API リクエストを観察
- **Elements タブ:** HTML を編集
- **Sources タブ:** JavaScript ソースコードを閲覧

**どのチャレンジで使う？**
- SQLi, XSS, IDOR, NoSQLi など、ほぼ全て

---

### 2. CrackStation
**URL:** https://crackstation.net/

**用途:** MD5/SHA1 などのハッシュを解読

**使い方:**
1. ハッシュ値をテキストボックスに貼り付け
2. CAPTCHA を解く
3. 「Crack Hashes」をクリック
4. よくあるパスワードなら数秒で解読される

**どのチャレンジで使う？**
- User Credentials（ユーザー情報抽出後）
- 各種ログインチャレンジ

**例:**
```
入力: 0192023a7bbd73250516f069df18b500
結果: admin123
```

---

### 3. Base64 デコーダー
**URL:** https://www.base64decode.org/

**用途:** Base64 エンコード/デコード

**使い方:**
1. Base64 文字列を貼り付け
2. 「Decode」をクリック

**どのチャレンジで使う？**
- Easter Egg
- Login Bjoern's Gmail
- JWT 関連

**例:**
```
入力: YWRtaW4xMjM=
結果: admin123
```

---

### 4. ROT13 デコーダー
**URL:** https://rot13.com/

**用途:** ROT13（13文字シフト）暗号の解読

**使い方:**
1. 暗号化された文字列を貼り付け
2. 自動的にデコードされる

**どのチャレンジで使う？**
- Easter Egg

**例:**
```
入力: /gur/qrif/ner/fb/shaal
結果: /the/devs/are/so/funny
```

---

### 5. JWT.io
**URL:** https://jwt.io/

**用途:** JWT トークンのデコード・編集

**使い方:**
1. JWT トークンを左側に貼り付け
2. ヘッダーとペイロードが右側に表示される
3. 編集して攻撃用トークンを作成

**どのチャレンジで使う？**
- Unsigned JWT
- Forged Signed JWT

---

### 6. URL エンコーダー/デコーダー
**URL:** https://www.urlencoder.org/

**用途:** URL の特殊文字をエンコード/デコード

**主な変換:**
```
スペース → %20
# → %23
%00 → %2500（ダブルエンコード）
```

**どのチャレンジで使う？**
- Poison Null Byte
- XSS ペイロード作成

---

## あると便利なツール

### 7. Burp Suite Community Edition
**URL:** https://portswigger.net/burp/communitydownload

**用途:** HTTP リクエストの傍受・編集

**特徴:**
- リクエストを途中で止めて編集できる
- リクエスト履歴を確認できる
- 繰り返しリクエストを送れる

**どのチャレンジで使う？**
- HTTPヘッダー操作
- パラメータ改ざん
- 認証バイパス

**注意:** やや上級者向け。DevTools で十分なチャレンジも多い。

---

### 8. Google Authenticator（または TOTP アプリ）
**URL:** App Store / Google Play で入手

**用途:** TOTP（2要素認証コード）の生成

**使い方:**
1. アプリを開く
2. 「手動で入力」を選択
3. シークレットキーを入力
4. 30秒ごとに変わる6桁コードを取得

**どのチャレンジで使う？**
- Two Factor Authentication

---

### 9. exiftool
**URL:** https://exiftool.org/

**用途:** 画像のメタデータ（EXIF）を読み取る

**使い方（コマンドライン）:**
```bash
exiftool image.jpg
```

**オンライン版:** https://exif.tools/

**どのチャレンジで使う？**
- Meta Geo Stalking（GPS座標の抽出）

---

### 10. Z85 エンコーダー
**URL:** https://cryptii.com/pipes/z85-encoder

**用途:** Z85 形式のエンコード/デコード

**どのチャレンジで使う？**
- Forged Coupon（クーポンコード生成）

---

## オンラインリソース

### OWASP 公式
- **Juice Shop ヘルプ:** https://help.owasp-juice.shop/
- **GitHub:** https://github.com/juice-shop/juice-shop

### 暗号解読
- **CyberChef:** https://gchq.github.io/CyberChef/ - 様々な変換を組み合わせ可能

### OSINT（公開情報調査）
- **Google:** キャラクター名で検索
- **Wikipedia:** 映画・アニメキャラの背景調査

---

## チャレンジ別ツール早見表

| チャレンジ | 主に使うツール |
|-----------|---------------|
| SQLi 系 | DevTools Console |
| XSS 系 | DevTools Console, URL エンコーダー |
| ハッシュ解読 | CrackStation |
| JWT 操作 | JWT.io, DevTools Console |
| TOTP | Google Authenticator |
| OSINT | Google 検索, Wikipedia |
| ファイル取得 | Poison Null Byte (URL エンコード) |
| 暗号解読 | Base64 デコーダー, ROT13, CyberChef |
| メタデータ | exiftool |

---

## 初心者へのアドバイス

1. **まず DevTools に慣れる** - 9割のチャレンジは DevTools だけで解ける
2. **Network タブを常に開く** - どんなリクエストが飛んでいるか観察する習慣をつける
3. **Console で試す** - JavaScript コードは Console にコピペして実行するだけ
4. **エラーメッセージを読む** - 攻撃のヒントが含まれていることが多い
5. **ソースコードを読む** - main.js には多くのヒントが隠されている

---

## キャラクター設定一覧（OSINT用）

Juice Shop のキャラクターは映画やアニメから来ています。OSINT チャレンジではキャラクターの背景知識が必要です。

| キャラクター | 元ネタ | セキュリティ質問の答え | パスワード |
|-------------|--------|----------------------|-----------|
| Jim | James T. Kirk (Star Trek) | `Samuel`（兄の名前） | `ncc-1701` |
| Bender | Bender (Futurama) | `Stop'n'Drop`（勤務先） | `OhG0dPlease1LubYou` |
| Bjoern | 作者本人 | `Zaya`（ペットの名前） | `bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=` |
| MC SafeSearch | YouTube動画のキャラ | - | `Mr. N00dles` |
| Amy | Kif の恋人 (Futurama) | - | `K1f.....................` |
| Morty | Morty (Rick and Morty) | - | ブルートフォース必要 |
| wurstbrot | ドイツ語で「ソーセージパン」 | - | TOTP必要 |

### キャラクター別ヒント

**Jim (Star Trek)**
- USSエンタープライズ号の登録番号: NCC-1701
- 兄: George Samuel Kirk（ミドルネームが答え）
- 好きな飲み物: Romulan Ale

**Bender (Futurama)**
- ロボット。曲げる作業が得意
- 最初の勤務先: Stop'n'Drop
- 決めゼリフ: "Bite my shiny metal ass!"

**Bjoern Kimminich**
- Juice Shop の作者
- ペットの名前: Zaya
- パスワード: メールアドレスを逆順にしてBase64

---

## 攻撃カテゴリ解説

### SQLi（SQLインジェクション）
**原理:** ユーザー入力がSQL文に直接組み込まれる脆弱性

```sql
-- 正常なクエリ
SELECT * FROM Users WHERE email = 'user@test.com'

-- 攻撃後
SELECT * FROM Users WHERE email = '' OR 1=1--'
```

**見つけ方:** 入力欄に `'` を入力してエラーが出るか確認

**主なペイロード:**
- `' OR 1=1--` - 全レコード取得
- `admin@juice-sh.op'--` - 特定ユーザーでログイン
- `')) UNION SELECT ...` - データ抽出

---

### XSS（クロスサイトスクリプティング）
**原理:** 悪意のあるスクリプトがページに挿入される脆弱性

**種類:**
| 種類 | 説明 | 例 |
|-----|------|-----|
| DOM XSS | クライアント側で発生 | 検索欄 |
| Reflected XSS | URLパラメータ経由 | `?id=<script>` |
| Stored XSS | データベースに保存 | コメント欄 |

**主なペイロード:**
```html
<iframe src="javascript:alert('xss')">
<img src=x onerror=alert(1)>
<<script>script>alert('xss')<</script>/script>
```

---

### IDOR（安全でない直接オブジェクト参照）
**原理:** URLやパラメータのIDを変えるだけで他人のデータにアクセスできる

```
本来: /api/basket/1  (自分のカート)
攻撃: /api/basket/2  (他人のカート)
```

**見つけ方:** URLやリクエストに含まれる数字を変えてみる

---

### NoSQLi（NoSQLインジェクション）
**原理:** MongoDBなどの演算子を悪用

**主なペイロード:**
```json
{"$ne": -1}    // 「-1でない全て」= 全レコード
{"$gt": ""}    // 「空文字より大きい全て」= 全レコード
```

**使用例:**
```javascript
// 全レビューを変更
body: JSON.stringify({id: {"$ne": -1}, message: "Hacked!"})
```

---

### XXE（XML外部エンティティ）
**原理:** XMLファイルで外部リソースを参照できる機能を悪用

```xml
<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<data>&xxe;</data>
```

**結果:** サーバー上のファイル（/etc/passwd など）が読み取れる

---

### JWT 攻撃
**原理:** JWT トークンのアルゴリズムを改ざん

**none アルゴリズム攻撃:**
```json
// 元のヘッダー
{"alg": "HS256", "typ": "JWT"}

// 攻撃後
{"alg": "none", "typ": "JWT"}
```

署名検証がスキップされ、ペイロードを自由に改ざん可能

---

## よくあるエラーと対処法

### 「SQLITE_ERROR」が表示される
**原因:** SQLインジェクションが成功している証拠
**対処:** このエラーはチャレンジ解決のヒント。ペイロードを調整する

### 「403 Forbidden」が表示される
**原因:** アクセス権限がない、またはファイル拡張子がブロックされている
**対処:**
- Poison Null Byte (`%2500.md`) を試す
- 認証トークンを追加する

### Console でエラーが出る
**原因:** JavaScript の構文エラー、または CORS エラー
**対処:**
- コードをそのままコピペしているか確認
- シングルクォートとダブルクォートに注意
- `fetch` の URL が正しいか確認

### ログインできない
**原因:** パスワードが間違っている、またはセッションが切れている
**対処:**
- SQLi でログインを試す: `admin@juice-sh.op'--`
- ブラウザのCookieをクリアして再試行

### Playwright がブラウザを起動できない
**原因:** 既存の Chrome セッションと競合
**対処:**
```bash
# キャッシュをクリア
rm -rf ~/Library/Caches/ms-playwright/mcp-chrome-*
```

### CAPTCHAが失敗する
**原因:** CAPTCHA の値が古い
**対処:**
- 新しい CAPTCHA を取得してすぐに使う
- `fetch('/rest/captcha')` で取得

---

## ファイル構成

| ファイル | 内容 |
|---------|------|
| `README.md` | このファイル（ツールガイド） |
| `CLAUDE.md` | Playwright 攻略パターン、クイックリファレンス |
| `result.md` | 進捗確認（難易度別リンク） |
| `difficulty-1.md` | 難易度1 の解法（初心者向け） |
| `difficulty-2.md` | 難易度2 の解法（SQLi, XSS入門） |
| `difficulty-3.md` | 難易度3 の解法（XXE, CAPTCHA Bypass） |
| `difficulty-4.md` | 難易度4 の解法（UNION SQLi, NoSQLi） |
| `difficulty-5-6.md` | 難易度5-6 の解法（JWT, 2FA バイパス） |

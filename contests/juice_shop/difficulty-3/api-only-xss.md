# API-only XSS ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** XSS
**目標:** フロントエンドを使わずAPIでXSSペイロードを保存する

## ソースコード分析

### 脆弱なユーザーモデル

**ファイル:** `models/user.ts` (lines 57-72)

```typescript
email: {
  type: DataTypes.STRING,
  unique: true,
  set (email: string) {
    if (utils.isChallengeEnabled(challenges.persistedXssUserChallenge)) {
      // チャレンジ有効時はサニタイズしない!
      challengeUtils.solveIf(challenges.persistedXssUserChallenge, () => {
        return utils.contains(
          email,
          '<iframe src="javascript:alert(`xss`)">'
        )
      })
    } else {
      email = security.sanitizeSecure(email)  // 通常はサニタイズ
    }
    this.setDataValue('email', email)
  }
}
```

### 脆弱なレンダリング

**ファイル:** `frontend/src/app/administration/administration.component.html` (line 26)

```html
<!-- innerHTML で直接表示 = XSS 脆弱性 -->
<mat-cell *matCellDef="let user" [innerHTML]="user.email"></mat-cell>
```

### API エンドポイント

**ファイル:** `server.ts` (lines 402-416)

```typescript
app.post('/api/Users', (req: Request, res: Response, next: NextFunction) => {
  // email をトリムするだけ、サニタイズなし
  req.body.email = req.body.email.trim()
  next()
})
```

## 実行手順

### Step 1: XSS ペイロード付きユーザー登録

```javascript
fetch('/api/Users/', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: '<iframe src="javascript:alert(`xss`)">',
    password: 'test123',
    passwordRepeat: 'test123'
  })
}).then(r => r.json()).then(console.log);
```

### Step 2: 管理画面で XSS 発火

1. 管理者としてログイン
2. `/#/administration` にアクセス
3. ユーザー一覧が表示された時に XSS 実行

## 攻撃フロー

```
[攻撃者] → POST /api/Users (XSS payload in email)
              ↓
[データベース] ← email: '<iframe src="javascript:alert(`xss`)">' 保存
              ↓
[管理者] → GET /#/administration
              ↓
[Angular] → [innerHTML]="user.email" でレンダリング
              ↓
[ブラウザ] → XSS 実行!
```

## 解説

### なぜ API 直接呼び出しで XSS できるのか？

**日常的な例えで説明すると:**

空港のセキュリティを想像してください。

- 正面入口: 金属探知機、X線検査（厳重なチェック）
- 従業員入口: IDカードのみ（簡易チェック）

攻撃者は「従業員入口」（API）を見つけて侵入する。

```
┌─────────────────────────────────────────────────────┐
│              フロントエンド経由（正面入口）           │
├─────────────────────────────────────────────────────┤
│  登録フォーム → Angular バリデーション → API → DB    │
│                      ↑                             │
│              「<script>は使えません」               │
│                 XSS をブロック!                     │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│              API 直接呼び出し（従業員入口）          │
├─────────────────────────────────────────────────────┤
│  fetch('/api/Users', {...}) → API → DB              │
│                               ↑                     │
│                      チェックなし!                  │
│                   XSS ペイロード保存成功            │
└─────────────────────────────────────────────────────┘
```

### Stored XSS の流れ

```
1. 攻撃                              2. 発動
   ┌─────────┐                          ┌─────────┐
   │ 攻撃者  │                          │ 管理者  │
   └────┬────┘                          └────┬────┘
        │ POST /api/Users                    │ GET /#/administration
        │ email: "<iframe src=...>"          │
        ▼                                    ▼
   ┌─────────┐                          ┌─────────┐
   │   DB    │                          │ Angular │
   │         │ ─────────────────────▶   │         │
   │ XSS保存 │                          │innerHTML│
   └─────────┘                          └────┬────┘
                                             │
                                             ▼
                                        XSS 実行!
                                        管理者の権限奪取
```

### なぜ脆弱か

| 層 | 問題点 |
|---|--------|
| フロントエンド | バリデーションはあるが、バイパス可能 |
| API | 入力をトリムするだけ、サニタイズなし |
| データベース | 悪意あるHTMLをそのまま保存 |
| 表示 | `[innerHTML]` でエスケープなしに表示 |

### innerHTML が危険な理由

```html
<!-- innerHTML: HTML として解釈される -->
<div [innerHTML]="'<img src=x onerror=alert(1)>'"></div>
→ 画像読み込み失敗 → onerror 実行 → alert(1)!

<!-- テキスト補間: 文字として表示される -->
<div>{{ '<img src=x onerror=alert(1)>' }}</div>
→ 文字列「<img src=x...」が表示されるだけ
```

### 根本原因

**「フロントエンドの検証 = セキュリティ」という誤解**

フロントエンドの検証は UX のため（「入力エラーです」と即座に表示）。
セキュリティはサーバー側で担保すべき。

### 対策（多層防御）

| 層 | 対策 |
|---|------|
| 入力時 | サーバーで HTML タグを除去 |
| 保存時 | 許可された形式のみ保存 |
| 出力時 | `innerHTML` を避け、テキスト補間を使う |

```typescript
// 入力時: DOMPurify でサニタイズ
email = DOMPurify.sanitize(email);

// 出力時: innerHTML を使わない
<div>{{ user.email }}</div>
```

### 対策

```typescript
// 1. API でサニタイズ
import DOMPurify from 'dompurify';
req.body.email = DOMPurify.sanitize(req.body.email);

// 2. Angular で textContent を使用
<mat-cell>{{ user.email }}</mat-cell>

// 3. innerHTML が必要な場合は DomSanitizer
<mat-cell [innerHTML]="user.email | sanitizeHtml"></mat-cell>
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `models/user.ts:57-72` | メール設定ロジック |
| `server.ts:402-416` | API エンドポイント |
| `frontend/.../administration.component.html:26` | 脆弱な innerHTML |

## 関連チャレンジ

- [DOM XSS](../difficulty-1/dom-xss.md) - 検索フィールドの XSS
- [Reflected XSS](../difficulty-2/reflected-xss.md) - 注文追跡の XSS

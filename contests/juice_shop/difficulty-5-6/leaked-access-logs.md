# Leaked Access Logs ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Observability Failures
**目標:** インターネット上に漏洩したパスワードを見つけて、元のユーザーアカウントにログインする

## 思考プロセス

このチャレンジは **OSINT (Open Source Intelligence)** を使った攻撃。開発者がデバッグ目的でアクセスログをインターネット上に公開してしまい、その中にパスワードが含まれていたというシナリオ。

### なぜパスワードがログに残るのか？

**GET リクエストのパスワード変更 API**:
```
GET /rest/user/change-password?current=XXX&new=YYY&repeat=ZZZ
```

この設計の問題点:
1. GET リクエストのクエリパラメータはアクセスログに記録される
2. パスワードが平文でログに残る
3. ログが外部に漏洩すると認証情報が流出

## 実行手順

### 1. Stack Overflow で手がかりを発見

Juice Shop の作者 Björn Kimminich が Stack Overflow に質問を投稿:
- **URL**: https://stackoverflow.com/questions/57061271/less-verbose-access-logs-using-expressjs-morgan

質問の中で Pastebin へのリンクが言及されている。

### 2. Pastebin でアクセスログを発見

- **URL**: https://pastebin.com/4U1V1UjU

ログ内で `password` を検索すると、パスワード変更リクエストが見つかる:

```
161.194.17.103 - - [27/Jan/2019:11:18:35 +0000] "GET /rest/user/change-password?current=0Y8rMnww$*9VFYE§59-!Fg1L6t&6lB&new=sjss22%@€55jaJasj!.k&repeat=sjss22%@€55jaJasj!.k8 HTTP/1.1" 401
```

**重要なポイント:**
- `current` = 現在のパスワード: `0Y8rMnww$*9VFYE§59-!Fg1L6t&6lB`
- `new` と `repeat` が一致しない（末尾の `8` の有無）
- レスポンスコード `401` = パスワード変更は失敗
- つまり、`current` のパスワードはまだ有効

### 3. ユーザーを特定

Write-up によると、このパスワードは `J12934@juice-sh.op` というユーザーのもの。

実際には Password Spraying（パスワードスプレー攻撃）を行う:
1. 発見したパスワードを使用
2. 既存のユーザーリスト（SQLiで取得可能）に対して試行
3. ログインが成功したユーザーを特定

### 4. ログインで解決

```
Email: J12934@juice-sh.op
Password: 0Y8rMnww$*9VFYE§59-!Fg1L6t&6lB
```

## コード/ペイロード

```javascript
// 直接ログイン
await fetch('/rest/user/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'J12934@juice-sh.op',
    password: '0Y8rMnww$*9VFYE§59-!Fg1L6t&6lB'
  })
}).then(r => r.json());
```

## 解説

### なぜパスワードがログに残るのか？

**日常的な例えで説明すると:**

郵便と宅配便の違いを想像してください。

- **はがき (GET)**: 住所と内容が外から丸見え → 配達員も見れる
- **封筒 (POST)**: 内容は封筒の中 → 配達員には見えない

```
GET /change-password?current=Secret123&new=NewPass
                     ↑
              URL に丸見え!

POST /change-password
Body: { current: "Secret123", new: "NewPass" }
      ↑
      リクエストボディの中（ログに残らない）
```

### URL が記録される場所

```
┌─────────────────────────────────────────────────────┐
│     GET リクエストの URL はあちこちに記録される       │
├─────────────────────────────────────────────────────┤
│                                                     │
│  [ユーザー]                                         │
│      │                                             │
│      │ GET /change-password?current=Secret123      │
│      ▼                                             │
│  [ブラウザ履歴] ← ここに残る                        │
│      │                                             │
│      ▼                                             │
│  [会社のプロキシ] ← ここにも残る                    │
│      │                                             │
│      ▼                                             │
│  [CDN / ロードバランサ] ← ここにも残る              │
│      │                                             │
│      ▼                                             │
│  [Web サーバー] ← アクセスログに残る                │
│      │                                             │
│      ▼                                             │
│  [モニタリングツール] ← ここにも残る                │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### このチャレンジの流れ

```
1. 開発者: パスワード変更を GET で実装（設計ミス）
       ↓
2. サーバー: アクセスログにパスワードが記録される
   "GET /change-password?current=0Y8rMnww$*9..."
       ↓
3. 開発者: 「ログが多すぎる」→ Stack Overflow に質問
   ログを Pastebin にコピペして共有
       ↓
4. 攻撃者: Stack Overflow → Pastebin → パスワード発見!
       ↓
5. 攻撃者: そのパスワードでログイン試行
```

### 根本原因

| 問題 | 説明 |
|------|------|
| **GET の誤用** | 状態を変更する操作に GET を使用 |
| **ログの公開** | アクセスログを外部で共有 |
| **複合的なミス** | 小さなミスが連鎖して大きな漏洩に |

### GET と POST の正しい使い分け

| メソッド | 用途 | URL に情報 | ログに残る |
|---------|------|-----------|-----------|
| GET | データ取得 | 含む | 残る |
| POST | データ送信 | 含まない | 残らない |

```
❌ GET /change-password?current=xxx&new=yyy
✅ POST /change-password
   Body: {"current": "xxx", "new": "yyy"}
```

### 教訓

1. **パスワードは POST で送る**: URL に秘密情報を含めない
2. **ログを公開しない**: Stack Overflow にログをそのままコピペしない
3. **ログのマスキング**: センシティブなパラメータは `***` に置換

```javascript
// ログ出力時のマスキング
const maskedUrl = url.replace(/password=[^&]+/g, 'password=***');
```

### 対策

1. **POST を使用**: パスワード変更は POST リクエストで
2. **ログのサニタイズ**: センシティブな情報をマスク
3. **ログの保護**: アクセスログを適切に管理
4. **漏洩検知**: Have I Been Pwned 等でのモニタリング

## 参考リンク

- [Stack Overflow 質問](https://stackoverflow.com/questions/57061271/less-verbose-access-logs-using-expressjs-morgan)
- [Pastebin ログ](https://pastebin.com/4U1V1UjU)
- [Curiosity Kills Colby - Write-up](https://curiositykillscolby.com/2020/12/27/hacking-owasps-juice-shop-pt-61-leaked-access-logs/)
- [OWASP Credential Stuffing Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Credential_Stuffing_Prevention_Cheat_Sheet.html)

## ステータス

- [x] Stack Overflow で手がかりを発見
- [x] Pastebin でアクセスログを発見
- [x] パスワードを抽出
- [x] ログインで解決

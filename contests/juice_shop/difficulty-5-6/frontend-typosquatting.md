# Frontend Typosquatting ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 脆弱コンポーネント
**目標:** typosquatting されたフロントエンド依存関係を発見

## ヒント

- **typosquatting:** 有名パッケージに似た名前の悪意あるパッケージ
- **ターゲット:** Angular 関連のパッケージ
- **調査対象:** `package.json`, `main.js`

## typosquatting の例

```
正規: @angular/core
偽物: @angualr/core, @angular-core, angular-core

正規: lodash
偽物: lodahs, lodash-es-fake
```

## 調査方法

```bash
# package.json を取得
curl http://localhost:3000/package.json

# main.js からインポートを確認
curl http://localhost:3000/main.js | grep -i "require\|import"

# ソースマップから依存関係を確認
curl http://localhost:3000/main.js.map
```

## 確認ポイント

- npm パッケージ名のスペルミス
- 公式パッケージと似た名前の非公式パッケージ
- deprecated されたパッケージ

## 検証ポイント

- [ ] package.json の依存関係を確認
- [ ] 不審なパッケージ名を特定
- [ ] npm registry で公式パッケージと比較

## 解説

[未着手]

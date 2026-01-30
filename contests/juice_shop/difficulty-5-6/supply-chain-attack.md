# Supply Chain Attack ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 脆弱コンポーネント
**目標:** サプライチェーン攻撃の痕跡を発見

## ヒント

- **対象:** npm パッケージの依存関係
- **脆弱性:** 悪意のある依存パッケージ
- **調査:** `package.json`, `package-lock.json`

## サプライチェーン攻撃とは

```
1. 攻撃者が人気パッケージの依存関係に悪意のあるコードを混入
2. または typosquatting で偽パッケージを公開
3. プロジェクトがそれを依存関係として取り込む
4. ビルド時や実行時に悪意のあるコードが実行される
```

## 調査方法

```bash
# package.json を取得
curl http://localhost:3000/package.json

# 依存関係を確認
cat package.json | jq '.dependencies, .devDependencies'

# npm audit で脆弱性チェック
npm audit
```

## チェックポイント

- 非公式のパッケージ名
- 不自然なバージョン指定
- postinstall スクリプト
- deprecated パッケージ

## 検証ポイント

- [ ] package.json を取得・分析
- [ ] 不審な依存関係を特定
- [ ] npm registry で確認

## 解説

[未着手]

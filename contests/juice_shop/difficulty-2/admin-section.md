# Admin Section ✅

**難易度:** ⭐⭐
**カテゴリ:** アクセス制御
**目標:** 管理者画面を見つけてアクセスする

---

## 思考プロセス

**ステップ1: 管理画面の存在を推測**
```
「管理者としてログインできた」
    ↓
「管理者用の画面があるはず」
    ↓
「URLを推測してみよう」
```

**ステップ2: main.js で検索**
```
「DevTools → Sources → main.js を開く」
    ↓
「Ctrl+F で "admin" を検索」
    ↓
「"administration" というルートを発見」
```

## 実行手順

1. 管理者でログイン（SQLi: `' OR 1=1--`）
2. アドレスバーに以下を入力:
   ```
   http://localhost:3000/#/administration
   ```
3. 管理画面が表示されれば成功

## 解説

- 管理画面のURLがJavaScriptに埋め込まれている
- URLを知っていれば誰でもアクセスできる（適切なアクセス制御がない）
- 「Security through Obscurity」の失敗例

## 関連チャレンジ

- [Score Board](../difficulty-1/score-board.md)
- [Five-Star Feedback](five-star-feedback.md)

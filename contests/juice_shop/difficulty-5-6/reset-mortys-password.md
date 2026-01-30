# Reset Morty's Password ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 自動化/ブルートフォース
**目標:** Morty のセキュリティ質問をブルートフォース

## ヒント

- **ターゲット:** `morty@juice-sh.op`
- **方法:** セキュリティ質問の答えをブルートフォース
- **元ネタ:** Rick and Morty (アニメ)

## ツール

- Burp Suite Intruder
- OWASP ZAP Fuzzer
- ffuf / wfuzz

## 手順

1. パスワードリセットページでメールを入力
2. セキュリティ質問を確認
3. リクエストを傍受
4. Intruder で `securityAnswer` をブルートフォース

## 辞書ファイル

```
/usr/share/seclists/Passwords/Common-Credentials/best1050.txt
/usr/share/seclists/Passwords/Common-Credentials/10k-most-common.txt
```

## Rick and Morty 関連の答え候補

```
Rick
Morty
Summer
Beth
Jerry
Pickle Rick
Wubba Lubba Dub Dub
Meeseeks
Plumbus
```

## Burp Suite 設定

```
1. Proxy → Intercept でリクエスト傍受
2. 右クリック → Send to Intruder
3. Positions で securityAnswer を選択
4. Payloads で辞書ファイルを設定
5. Start Attack
6. レスポンス長や Status Code で成功を判定
```

## 検証ポイント

- [ ] セキュリティ質問の内容を確認
- [ ] ブルートフォース実行
- [ ] 正解を発見してパスワードリセット

## 解説

[未着手]

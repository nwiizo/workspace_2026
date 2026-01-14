# 三十秒の沈黙 - サンプルコード

ブログ記事「三十秒の沈黙」に登場するサンプルコード集。

## ファイル一覧

| ファイル | 物語の章 | 内容 |
|---------|---------|------|
| `error_examples.py` | 一〜四 | よくあるエラーメッセージの例と原因・対策 |
| `unsolvable_by_ai.py` | 八 | 月曜朝のタイムアウト問題（AIが解けない問題） |
| `order_service_buggy.py` | 九 | レースコンディションのあるバグコード |
| `order_service_fixed.py` | 九 | 修正後のコード（トランザクション+ロック） |
| `bad_exception_handling.py` | 五 | 悪い例外処理と良い例外処理の比較 |

## 実行方法

```sh
python3 error_examples.py
python3 unsolvable_by_ai.py
python3 order_service_buggy.py
python3 order_service_fixed.py
python3 bad_exception_handling.py
```

## 記事の核心

**仮説を持っていないから、問題が解けない。**

- 仮説がない → 検索しても答えが見つからない
- 仮説がない → AIに聞いても一般論しか返ってこない
- 仮説がない → 会議で質問されても答えられない

三十秒考える。それは「仮説を持つ」ための時間。

仮説を持てば：
- **検索の仕方が変わる**: エラーメッセージのコピペ → 仮説の検証
- **AIへの質問が変わる**: 「これ何？」→「○○だと思うけど合ってる？」
- **会議で話せるようになる**: 方向性を示せる

AIは「答え」ではなく「仮説の検証」に使う。

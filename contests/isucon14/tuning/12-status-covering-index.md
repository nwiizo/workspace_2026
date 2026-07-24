# Benchmark 12: 最新status検索のcovering INDEX

[チューニング目次へ戻る](../TUNING.md)

## 目的

次のSQLは通知、座標更新、状態遷移、ride作成時の検査から繰り返し使われます。

```sql
SELECT status
FROM ride_statuses
WHERE ride_id = ?
ORDER BY created_at DESC
LIMIT 1;
```

既存の `(ride_id, created_at)` INDEXは、対象rideの範囲を見つけ、
`created_at` の新しい順に読むところまで支援します。しかし、返したい `status` は
INDEXに含まれないため、InnoDBの主キー行を追加で参照します。`status` をINDEXの
末尾へ加え、INDEXだけで結果を返すcovering INDEXが総合性能を改善するか検証しました。

## covering INDEXとは

通常のsecondary INDEXには、指定したINDEX列と主キーが保存されます。検索に必要な
列がINDEX内で見つかっても、SELECTする列がなければ、主キーを使ってテーブル本体を
もう一度読みます。

必要な列をすべてINDEXへ含めると、テーブル本体へ戻らずに応答できます。これを
covering INDEXと呼びます。今回は既存INDEXを次のように変更しました。

```diff
- INDEX idx_ride_statuses_ride_created_at (ride_id, created_at)
+ INDEX idx_ride_statuses_ride_created_at (ride_id, created_at, status)
```

`status` はENUMなので大きな文字列全体ではなく内部の小さな数値として保持されます。
それでも、すべての `ride_statuses` INSERTでsecondary INDEXへstatusを書き込む
追加コストは発生します。

## 仮説

- 最新status検索は高頻度なので、主キー行の追加参照をなくす効果が積み上がる
- `ride_id, created_at` の並びは変えないため、既存の検索とsort回避を維持できる
- statusは小さいため、INSERT時のwrite amplificationよりread削減が上回る

反証条件は、実行計画がcoveringにならない、または60秒ベンチの処理量・スコアが
改善しないことです。

## 変更前後の実行計画

8,066行の `ride_statuses` から、status履歴が6件あるrideを指定して
`EXPLAIN ANALYZE` しました。

| 項目 | 変更前 | 変更後 |
|---|---:|---:|
| optimizer cost | 2.10 | 1.71 |
| 単発実測 | 0.415ms | 0.050ms |
| operator | Index lookup | Covering index lookup |
| `EXPLAIN Extra` | Backward index scan | Backward index scan; Using index |

`Using index` と `Covering index lookup` を確認できたため、実行計画は狙いどおり
変わりました。ただし単発時間はbuffer poolの温まり方や直前のI/Oに左右されます。
0.415ms→0.050msだけを根拠に採用せず、ベンチ全体で判断します。

## 60秒ベンチ

| 条件 | pass | スコア | 最終tick評価数 | エラー |
|---|---:|---:|---:|---:|
| 既存 `(ride_id, created_at)` | true | 53,198 | 745 | 0 |
| covering `(ride_id, created_at, status)` | true | 45,075 | 630 | 0 |

covering版は対照より8,123点、約15.27%低下しました。正当性エラーはありませんが、
最終評価数も115件減っています。

不満足度はmatching 15.0%、pickup 34.9%、drive 82.0%でした。対照はそれぞれ
9.5%、38.6%、84.3%です。pickupだけは良化していますが、総得点と処理量の低下を
覆す結果ではありません。

## 確認したログ

| 指標 | covering版 |
|---|---:|
| matcher | 98回 |
| app通知 | 36,602回 |
| chair通知 | 37,520回 |
| 座標更新 | 26,621回 |
| BEGIN | 103,652回、累積6.684秒 |
| COMMIT | 103,467回、累積426.580秒、平均4.123ms |
| ROLLBACK | 187回、累積0.031秒 |
| slow statement警告 | 6件 |
| pool警告 | 0件 |

対照の主要3経路は121,801回、covering版は100,743回でした。走行ごとの入力の
揺れを含むため、INDEX列追加だけが21,058回の差をすべて生んだとは断定できません。
一方で、総合ベンチに改善の証拠がなく、追加write costを恒久的に負う理由も
ありません。

## 判断

不採用とし、schemaを `(ride_id, created_at)` へ戻しました。

この実験から分かるのは「covering INDEXは無意味」ではありません。実行計画上の
readは明確に軽くなりました。しかし、次の理由から現在のデータ量と処理構成では
優先度が低いと判断しました。

- 1rideのstatus履歴は最大6件程度で、主キー行の追加参照範囲が小さい
- 変更前でもbuffer pool上の検索は十分短い
- status INSERTは状態遷移ごとに発生し、INDEX更新は必ず増える
- COMMIT累積時間が400秒前後あり、単発SELECTよりtransaction永続化が支配的

## 他の選択肢

### 現在statusを1行で持つ

履歴は仕様検証用に残し、ride本体またはcurrent-state表へ最新statusを持てば、
最新1件検索そのものを削除できます。ただし履歴INSERTと現在状態更新を同じ
transactionにし、initialize時のbackfillと不変条件を定義する必要があります。

### INDEXを増やさずSELECT回数を減らす

同じrequest内で最新statusを複数回取得しない、ride一覧のN+1をJOINまたはwindow
関数でまとめる方法です。1回を少し速くするより、SQL往復を丸ごと削る方が効果が
大きい可能性があります。

### COMMITの永続化設定を別に測る

この走行では `COMMIT` が103,467回、累積426.580秒でした。MySQLは
`innodb_flush_log_at_trx_commit=1`、`sync_binlog=1`、binary log有効です。
クラッシュ時の耐久性と引き換えにfsync回数を減らす設定は、INDEXとは分離した
次のベンチで検証します。

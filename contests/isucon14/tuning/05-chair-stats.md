# Benchmark 05: 椅子統計のN+1を集約SQLへ置き換える

[チューニング目次へ戻る](../TUNING.md)

## 結果

| 項目 | 結果 |
|---|---:|
| 60秒pass | false |
| スコア | 4,460 |
| エラー | `CODE=32` 2件 |
| 統計内容の不一致 | 0件 |
| 最大7 ridesの候補SQL | 約1.38ms |

この変更は椅子統計の正当性を保ちましたが、ベンチ全体はmatcherの期限超過で失敗しました。したがって「椅子統計だけで全体が成功した」とは評価せず、N+1削減の実装は採用し、次のBenchmark 06でmatcherを直接改善しました。

## どこから呼ばれる処理か

利用者通知 `/api/app/notification` は、椅子が割り当てられた後に名前・モデルと次の統計を返します。

- 乗車完了回数
- 完了rideの評価平均

通知は短い間隔でpollingされます。この内側で遅い処理を行うと、1 requestだけでなくMySQL connection poolを待つ他requestにも影響します。

## 変更前のN+1

変更前の `get_chair_stats` は次を行いました。

1. chairの全ridesを取得
2. rideごとに全status履歴を取得
3. Rustで `ARRIVED`、`CARRYING`、`COMPLETED` がすべてあるか確認
4. 条件を満たすrideの件数と評価合計を計算
5. Rustで平均を計算

chairに `R` 件のrideがあれば、SQLは `1 + R` 回です。通知transactionを開いたまま直列にqueryするため、履歴が増えるほどconnectionを長く専有します。

## 実装したSQL

```sql
SELECT COUNT(*) AS total_rides_count,
       CAST(COALESCE(AVG(completed_rides.evaluation), 0) AS DOUBLE)
           AS total_evaluation_avg
FROM (
    SELECT rides.id, rides.evaluation
    FROM rides
    INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
    WHERE rides.chair_id = ?
      AND rides.evaluation IS NOT NULL
    GROUP BY rides.id, rides.evaluation
    HAVING SUM(ride_statuses.status = 'ARRIVED') > 0
       AND SUM(ride_statuses.status = 'CARRYING') > 0
       AND SUM(ride_statuses.status = 'COMPLETED') > 0
) AS completed_rides
```

内側queryはride単位にstatusをまとめ、必要な3状態が1件以上あるrideだけを残します。外側queryが件数と平均を1行へ集約します。rideが0件でも `COUNT(*)` は0、`AVG` はNULLになるため、`COALESCE(..., 0)` で従来どおり平均0.0を返します。

`AVG(INT)` はMySQLではDECIMAL系になり得ます。response fieldはRustの `f64` なので、`CAST(... AS DOUBLE)` をSQLに明記し、sqlxのdecode型を曖昧にしません。

## 実行計画

ベンチ後に7 ridesを持つchairで計測しました。

```text
rides:
  idx_rides_chair_created_at で7行
ride_statuses:
  idx_ride_statuses_ride_created_at でrideごとに6行
全体:
  約1.38ms
```

一時表は7 ride分だけです。全ride・全status履歴を走査していないことを `EXPLAIN ANALYZE` のactual rowsで確認しました。

## ログをどう判断したか

最終結果は次のとおりです。

```text
ライドが長時間マッチングされませんでした (CODE=32)
結果 pass=false スコア=4460 種別エラー数=map[32:2]
```

ベンチマーカーには椅子統計の件数・平均を照合する検査があります。この走行では統計不一致のエラーがなく、失敗は未割当rideの30秒期限でした。そこで集約SQLを戻すのではなく、ログが直接示すmatcherへ次の調査対象を移しました。

## 他の選択肢

- 3本の `EXISTS` を使う: 読みやすいが、rideごとにstatus INDEXを複数回調べる可能性がある
- `COMPLETED` だけを見る: 現在の状態遷移が必ず正しいなら短いが、元実装の3状態確認より条件が弱くなる
- chair_statsを別表へ保持する: readは最速だが、評価完了時の更新、initialize、二重更新防止が必要
- メモリへcacheする: DB readをなくせるが、複数instanceや再起動時の整合方法が必要

今回は元実装と同じ3状態条件をSQLへ移すだけに留め、意味を変えず往復回数を減らしました。

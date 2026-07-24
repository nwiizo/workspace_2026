# 評価APIのride所有者認可

## 結論

`POST /api/app/rides/:ride_id/evaluation` が、認証済みユーザー本人のrideだけを評価できる
ように修正しました。rideをrow lockで取得するとき、主キー `id` だけでなく
middlewareが確定した `user_id` も条件へ含めます。

```sql
SELECT *
FROM rides
WHERE id = ?
  AND user_id = ?
FOR UPDATE;
```

別ユーザーのcookieで既知のride IDを指定するHTTP回帰テストを追加し、HTTP 404となる
こと、evaluation、`COMPLETED`、chair statsがすべて変化しないことを確認しました。
公式prevalidationも `pass=true`、error map空です。

この修正はスコア改善施策ではないため、60秒スコアの推定値は作りません。Benchmark 20の
最終3走を現在の性能値とし、認可修正では正当性だけを独立して検証しました。

## 発見した経路

Benchmark 20の最終Rustレビューで、評価handlerだけが
`axum::Extension<User>` を受け取っていないことを確認しました。

app用middlewareは `app_session` cookieからユーザーを検索し、request extensionへ
`User` を入れています。ほかのapp handlerはこのユーザーを使いますが、評価handlerの
locking readは次の形でした。

```sql
SELECT * FROM rides WHERE id = ? FOR UPDATE;
```

そのため、requestが「誰から来たか」は認証できても、「そのrideを操作してよいか」を
検査していませんでした。

## はじめに知っておく用語

### 認証

認証は、requestを送った主体が誰かを確かめる処理です。このアプリでは
`app_session` cookieのaccess tokenを `users` tableで検索し、ユーザーを特定します。

### 認可

認可は、認証した主体が対象resourceを操作してよいかを確かめる処理です。
ログイン済みであっても、他ユーザーのrideを評価する権限はありません。

認証が通ることと認可が通ることは別です。handlerがresource IDをpathから受け取る場合、
resource取得条件へ所有者IDも含まれているかを確認します。

### IDOR

IDORは、objectのIDを別の値へ変えることで、本来アクセスできないobjectを参照・変更
できる問題です。今回のride IDはULIDで推測しにくいものの、IDが漏れないことを認可の
代わりにはできません。ログ、画面、別API、監視情報などからIDを知る可能性があるため、
server側で所有者を検査します。

## 影響

別ユーザーがride IDを知っている場合、旧実装では次の処理へ到達できました。

1. `rides.evaluation` を更新する
2. `COMPLETED` statusを追加する
3. chair statsを加算する
4. ride所有者のpayment tokenで決済する

これは単なる表示漏えいではなく、ride状態と決済を変更するwrite認可の欠落です。
transactionで4処理がatomicでも、transactionへ入る主体が正しいとは限りません。
atomicityとauthorizationは別々に検証する必要があります。

## 修正

handlerでmiddlewareが挿入した認証ユーザーを受け取ります。

```rust
axum::Extension(user): axum::Extension<User>
```

次にride取得を主キーだけの検索から、主キーと所有者の検索へ変えました。

```rust
sqlx::query_as(
    "SELECT * FROM rides WHERE id = ? AND user_id = ? FOR UPDATE"
)
.bind(&ride_id)
.bind(&user.id)
```

対象が存在しない場合と、存在するが別ユーザー所有の場合は、どちらもHTTP 404として
扱います。HTTP 403で所有関係を区別すると、ride IDの存在確認に使われるためです。

## INDEXへの影響

`rides.id` は主キーです。MySQLはまず主キーで最大1行へ絞り、そのrowの `user_id` を
追加条件として確認できます。このqueryのために `(id, user_id)` の複合INDEXを追加する
必要はありません。

主キーはすでに一意なので、複合INDEXを増やしても候補行は1行より少なくなりません。
一方でINDEXを増やすとride INSERTの更新先とstorageが増えます。INDEXはWHERE句に列が
現れたという理由だけで追加せず、候補行数と実行計画から判断します。

## HTTP回帰テスト

`./scripts/test-chair-stats-transitions.sh` に、所有者とは異なる初期ユーザーのcookieで
評価を送るcaseを追加しました。

確認項目は次のとおりです。

- responseはHTTP 404
- `rides.evaluation` はNULLのまま
- `COMPLETED` statusは0件
- chair statsの件数・評価合計は不変
- 続けて本来の所有者が同じrideを評価できる

同じscriptで決済失敗時rollback、正常完了、再送時の非加算、`CARRYING` 欠損rideの
非集計も確認するため、認可失敗が後続transactionへ状態を残していないことまで
連続して検証できます。

## 他の選択肢

### ride取得後にRustで比較する

`id` だけでrow lockを取り、`ride.user_id != user.id` をRustで判定する方法です。
正しく実装すれば認可できますが、権限のないrequestでもrow lockを取得します。
SQL条件へ含めれば、取得と認可を1か所で表現できます。

### middlewareですべて認可する

認証middlewareは全app routeで共有されています。ride IDを持たないrouteもあり、
resourceごとの権限はhandlerによって異なります。共通middlewareへride認可を混ぜると
route判定とDB accessが複雑になるため、評価resourceを取得するhandlerで検査します。

### 推測困難なIDだけに頼る

ULIDの推測困難性は大量走査を難しくしますが、漏えいした1件への不正操作を防ぎません。
unpredictable IDは補助的な性質であり、所有者条件を省略する理由にはなりません。

## 今後の確認

path parameterでresourceを指定するwrite APIについて、次を同じ観点で監査します。

1. 認証主体をhandlerが受け取っているか
2. locking readまたはUPDATEのWHERE句に所有者条件があるか
3. 権限のないrequestが外部I/Oや副作用へ到達しないか
4. 存在有無をresponse差で過度に公開していないか
5. 別主体のHTTP回帰テストがあるか

# Benchmark 22: 認証主体をprocess内cacheへ保持する

## 結論

全APIのmiddlewareが毎回行っていたaccess token検索を、process内の
`HashMap` から読む方式へ変更しました。起動時と `POST /api/initialize` 後に
users、owners、chairsを再読込し、動的登録された主体は最初のcache missだけDBへ
fallbackして取り込みます。

60秒3走は102,887 / 104,612 / 109,454点で、推定代表値の中央値は104,612点でした。
直前のBenchmark 20中央値98,452点から+6,160点、約+6.3%です。

run 3の終了時snapshotでは、認証SQLの合計は変更前の139,690回・9.761秒から
657回・0.069秒へ減りました。呼出しは約99.5%、累積時間は約99.3%減っています。
全run `pass=true` なので採用します。

ただし、既知のnearby競合による `CODE=30` が6 / 15 / 20件発生しました。最大でも
soft error予算200件の10%ですが、エラー0ではありません。認証cacheの効果と混同せず、
次の最優先項目として別に修正・計測します。

## どのログから優先したか

Benchmark 21の不採用run終了直後に、
`performance_schema.prepared_statements_instances` をSQL本文ごとに集計しました。

| 認証SQL | calls | 累積 | 平均 | 最大 | rows examined |
|---|---:|---:|---:|---:|---:|
| chairs | 80,067 | 6.098秒 | 0.076ms | 11.160ms | 80,067 |
| users | 59,424 | 3.642秒 | 0.061ms | 16.549ms | 59,424 |
| owners | 199 | 0.021秒 | 0.107ms | 2.819ms | 199 |
| 合計 | 139,690 | 9.761秒 | - | - | 139,690 |

各queryはUNIQUEまたはINDEX lookupで、平均は0.1ms未満です。単発では速くても、
通知と座標APIが30ms間隔で呼ぶため約13.9万回へ増えます。INDEX追加では1行lookupより
根本的に減らせないので、同じtokenを繰り返し検証する処理自体を減らす方針にしました。

この選択はBenchmark 21の学びとも対応します。複雑な1 SQL化は呼出しを減らしても
MySQLの累積CPUを増やしました。認証cacheはDB側の仕事を複雑にせず、cache hitなら
queryを発行しません。

## 認証cacheで保持するもの

`AppState` に `AuthCache` を追加し、次の3つを分けて保持します。

```text
users:  access_token -> User
owners: access_token -> Owner
chairs: access_token -> Chair
```

access tokenは推測されにくいランダム値で、各表の認証用INDEXでも一意に検索できます。
middlewareはCookieからtokenを取り出し、cacheに対応する認証主体があればcloneして
request extensionへ渡します。handlerが受け取る型を変えていないため、既存の
`Extension<User>`、`Extension<Owner>`、`Extension<Chair>` はそのままです。

### `HashMap` を使う理由

`HashMap` はkeyのhash値から格納位置を探します。平均的にはデータ件数へ比例した全走査を
せず、tokenから値をほぼ一定時間で探せます。MySQLのB-tree lookupも十分速いのですが、
process内cacheには次の差があります。

- connection poolを取得しない
- MySQL protocolでqueryを送受信しない
- rowを毎回decodeしない
- MySQLのCPUとschedulerを使わない

cacheの利点はHashMapの数ナノ秒だけではなく、DBへ到達するまでの境界全体を省くことです。

### `Arc` を使う理由

Axumの `AppState` はrouterやrequest処理へcloneされます。`Arc` は値の所有権を複数の
handleで共有する参照カウンタです。`HashMap` 自体をrequestごとに複製せず、すべての
middlewareが同じcacheを参照できます。

### `std::sync::RwLock` を使う理由

認証はほとんどが読取りです。`RwLock` は複数のreaderを同時に通し、書込み時だけ排他
します。lock guardを保持するのは `HashMap::get` とcloneの短い同期処理だけで、
その間に `.await` しません。

Tokioの非同期lockは、lock待ち中にthreadを占有しない必要がある長いcritical sectionに
向きます。今回はDBやnetwork I/Oをlock内へ入れないため、短い同期lockを選びました。
今後profileでlock競合が上位になれば、mapをshardする案や `DashMap` を比較します。
型を変えるだけで速いとは判断せず、待機時間の計測を条件にします。

## cache hitとcache miss

cacheにtokenがあればhit、なければmissです。

```text
Cookieのtoken
  -> cache hit  -> 認証主体をrequest extensionへ追加
  -> cache miss -> DBのINDEX lookup
                   -> 見つかればcacheへ追加
                   -> 見つからなければ401
```

この方式はcache-asideと呼ばれます。cacheを正本にせず、miss時はMySQLを正本として
確認します。動的登録されたuser、owner、chairは登録handlerからcacheへ直接書かなくても、
最初の認証requestで1回だけDBへfallbackし、その後はhitします。

登録transactionがcommitする前にcacheへ入れる方式は採りませんでした。DB commitが
失敗したのに認証だけ成功する状態を避けるためです。commit後に明示追加すれば最初の
1 queryも消せますが、登録handler3箇所とtransaction終了順序を変更するため、今回は
小さなcache-aside版を先に単独計測しました。

### 無効tokenをcacheしない理由

存在しないtokenは401にしますが、「存在しない」という負の結果はcacheしません。
外部から多数の異なる不正tokenを送られたとき、負のentryが無制限に増えてmemoryを
消費するのを避けるためです。高頻度の攻撃対策が必要なら、認証cacheとは別に
rate limitと上限付きTTL cacheを設計します。

## 起動とinitialize

process起動時は、routerがlistenを開始する前に3表を読みます。したがって初期tokenの
最初のrequestからcache hitになります。

`POST /api/initialize` は表をdropして初期データを入れ直します。既存の
maintenance gateはinitializeがwrite lock、通常APIがread lockを持つため、reset中に
通常APIが古いcacheを見ることはありません。

initializeは、write lock取得後かつ初期化script実行前にauth cacheを空にします。
script、settings更新、cache再読込のどこかで失敗しても、前のrunだけに存在したtokenを
cache hitで認証しません。通常API再開後のmissはDBを正本として確認します。

成功時は初期化scriptと決済URL更新のあと、auth cacheを全置換してからAPIを再開します。
全置換には次の意味があります。

- 初期データを再読込する
- 前のrunで動的登録されたtokenを削除する
- DBには存在しない古い認証主体を残さない

users、owners、chairsのmapは順に置換されますが、その間はmaintenance write lockで
通常APIが止まっています。clientからは3種類が部分的に新旧混在する状態を観測できません。

### cacheをlogへ出さない

access tokenはcredentialです。`AuthCache` の自動 `Debug` をderiveすると、mapのkeyと
モデル内tokenをすべて展開できます。独自の `Debug` はusers / owners / chairsの件数だけを
出し、token、氏名、IDを表示しません。性能診断のためにstateをlogへ出しても、
認証情報を漏らさない境界にしています。

## 可変属性とcacheの境界

現在のcacheは既存handler型を保つため、IDだけでなくモデル全体を保持します。
userとownerの認証後に使う属性は現在のAPIでは変更されません。chairの `is_active` は
`POST /api/chair/activity` で変わるため、cache内の値は古くなり得ます。

ただし現在のchair handlerは認証extensionから `chair.id` だけを認証済み主体の識別に使い、
nearbyとmatcherのactive判定は毎回DBの `chairs.is_active` を読みます。このため、
古いcached `is_active` を認可や空車判定に使っていません。

今後handlerが可変属性を使う場合は、この前提を暗黙に広げません。cache値をIDなどの
不変な認証identityへ縮めるか、更新時に全processへinvalidateする必要があります。

### DBを直接変更した場合の失効

cache hitでは毎回DBを再確認しません。そのため、運用者がSQLで主体を削除したり
access tokenをrotationしたりしても、現在のprocessが持つ旧tokenは次のinitializeまで
cache hitできます。現行APIには主体削除やtoken rotation endpointがなく、公式ベンチ中に
この変更は起きないため、今回の競技スコープでは採用しました。

一般運用で失効を即時反映するには、削除・rotation経路からcache entryを消すか、短いTTL、
世代番号、共有invalidate eventのいずれかが必要です。「MySQLが正本」という説明は
cache missと再構築時には成立しますが、hit中の即時失効まで保証する意味ではありません。

## 複数processでの注意

今回検証した構成はwebapp 1 processです。別processで新しい主体が登録されても、
そのtokenは最初のrequestでDBへfallbackするので取り込めます。

一方、あるprocessだけがinitializeを受け、別processが古い有効tokenをcache hitできる
構成では、別processの古いentryを消せません。複数processへ拡張する場合は次のいずれかが
必要です。

1. initialize generationをDBへ保存し、request時に世代を確認する
2. Redisなど共有cacheへ置く
3. publish/subscribeで全processへinvalidateを通知する
4. initialize中はload balancerを含めて全instanceを停止・再構築する

現在のDocker Composeは単一webappなので、検証していない水平分割まで安全とは記載しません。

## 正当性テスト

### cache専用回帰テスト

`scripts/test-auth-cache.sh` は実際のHTTPとPerformance Schemaを使い、次を確認します。

1. 初期userの認証はDB認証queryを増やさない
2. 動的userは最初のrequestでDB queryを1回だけ実行する
3. 同じ動的userの2回目はcache hitになる
4. 初期化scriptを一時退避した故障注入後も、削除済みtokenを認証しない
5. initialize後は動的userの古いtokenが401になる
6. initialize後に初期userがDB queryなしで再び認証できる

結果:

```text
OK: initial user authentication is served from cache
OK: dynamic user uses one DB fallback and is cached
OK: failed initialize does not restore stale authentication entries
OK: initialize replaces stale entries and reloads initial users
```

故障注入は `webapp/sql/init.sh` を同じディレクトリの一時名へ退避し、
`Command::new` を即時失敗させます。trapと通常経路の両方で元のpathへ戻し、
最後に正常initializeで初期状態へ復元します。

### 全体検証

| 検証 | 結果 |
|---|---|
| `cargo check --all-targets --all-features` | 成功 |
| `cargo test --all --all-targets` | 7件成功 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 成功 |
| `shellcheck scripts/test-auth-cache.sh` | 成功 |
| 公式prevalidation | `pass=true`、error map空 |
| status順序回帰 | app / chairとも全項目成功 |
| smoke test | トップ200、initialize成功 |

公式prevalidationは動的user、owner、chairの登録と、その後の認証APIを含みます。
専用testではuserのquery回数とinitialize失敗時まで厳密に確認し、prevalidationで
3種類の全体フローを補完しました。

## 60秒ベンチマーク

ホストとColimaのCPU / memoryは変更していません。

| 項目 | 内容 |
|---|---|
| Colima | 4 CPU / 4 GiB |
| 走行時間 | 60秒 |
| 静的ファイル検証 | 有効 |
| MySQL | Benchmark 20と同じ設定 |

| run | pass | スコア | error map | matching不満 | pickup不満 | drive不満 |
|---|---|---:|---|---:|---:|---:|
| 1 | true | 109,454 | `CODE=30: 6` | 36.8% | 39.6% | 68.7% |
| 2 | true | 102,887 | `CODE=30: 15` | 45.8% | 39.3% | 66.5% |
| 3 | true | 104,612 | `CODE=30: 20` | 39.8% | 39.0% | 67.2% |

小さい順は102,887、104,612、109,454なので、推定代表値は104,612点です。
観測範囲は102,887–109,454点です。直前の中央値との差は次です。

```text
104,612 - 98,452 = 6,160
6,160 / 98,452 ≒ 6.3%
```

不満率はrunごとのworld展開で大きく変わり、認証cacheだけでmatching policyは変えて
いません。3走すべてでスコアが直前中央値を上回り、内部の認証SQL削減とも方向が一致
したため採用しました。

## 変更後のSQL

run 3終了直後のsnapshotです。

| 認証SQL | calls | 累積 | 平均 | 最大 | rows examined |
|---|---:|---:|---:|---:|---:|
| chairs fallback | 359 | 0.048秒 | 0.134ms | 5.534ms | 359 |
| users fallback | 294 | 0.021秒 | 0.072ms | 2.023ms | 294 |
| owners fallback | 4 | 0.000秒 | 0.030ms | 0.033ms | 4 |
| 合計 | 657 | 0.069秒 | - | - | 657 |

| 指標 | 変更前 | 変更後run 3 | 削減 |
|---|---:|---:|---:|
| calls | 139,690 | 657 | 約99.5% |
| 累積時間 | 9.761秒 | 0.069秒 | 約99.3% |
| rows examined | 139,690 | 657 | 約99.5% |

fallback回数が0でないのは、ベンチ中にuser、owner、chairが動的登録されるためです。
これはcache漏れではなく、最初の認証で正本を確認する設計どおりです。

run 3終了時の `Performance_schema_prepared_statements_lost=0` も確認しました。
ただしprepared statement snapshotは終了済みconnectionの情報を失う可能性があるため、
全期間の完全なtraceではありません。変更前後を同じ方法で比較し、3走の全体スコアと
回帰テストを合わせて判断しています。

## `CODE=30` をどう判断したか

ログ本文は次です。

```text
取得した付近の椅子情報に不備があります (CODE=30):
ID:...の椅子は既にライド中です
```

ベンチマーカーはnearby応答の椅子について、3秒より前にマッチ済みで未評価のrideが
残っていないかを検査します。現在のDB queryは `rides.evaluation IS NULL` を使い、
評価handler実行中はprocess trackerで除外しています。それでもthroughputが上がった3走で
6–20件再現したため、response body終了とベンチマーカー側の評価済み反映の境界を
改めて測る必要があります。

20件はfail閾値200件未満で、全runは `pass=true` です。しかしsoft errorは正当性予算を
消費し、さらにthroughputを上げると増える可能性があります。固定cooldownを推測で
足すのではなく、該当chair ID、評価response時刻、nearbyの `baseTime`、DB commit、
tracker解除を同じ時刻軸で採取してから別Benchmarkで修正します。

## 他の選択肢

### 登録handlerからcommit後に直接追加する

最初のfallbackも消せます。3種類の登録handlerで、commit成功後だけcacheへ入れる必要が
あります。今回の残り657回は累積0.069秒なので、実装範囲を増やす前に他のhot SQLを
優先します。

### IDだけをcacheする

memoryとclone量を減らし、可変属性のstale問題を明確にできます。一方、既存handlerの
extension型を変更する必要があります。CPU profileでcloneがhotになるか、可変属性を
認可へ使う変更が入るときに進めます。

### Redisなどの共有cache

複数processでinvalidateを共有できますが、network hopと別service運用が増えます。
単一processの現在はprocess内HashMapの方が単純で速く、再現環境も小さく保てます。

### token検証を署名付き形式へ変更する

JWTなどならDB lookupなしに署名を検証できますが、token形式と失効設計を変更します。
公式APIのopaque access tokenを保つ今回の範囲を越えるため採用しません。

## 次に行うこと

1. `CODE=30` の該当chairについて評価、tracker、nearby、world検査の時刻を採取する
2. fixed cooldownではなく、response完了境界または明示的な状態versionで競合を閉じる
3. エラー0の3走を確認してから、座標current rowのwrite amplificationへ戻る

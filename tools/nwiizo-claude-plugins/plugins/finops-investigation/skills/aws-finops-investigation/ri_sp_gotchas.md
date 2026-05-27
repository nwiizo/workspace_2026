# RI / Savings Plans の誤認パターン

購入判断やコスト見え方で頻発する誤解。

## カバー範囲の誤認

- **Savings Plans は RDS / ElastiCache / Redshift に効かない** — Compute SP は EC2 / Fargate / Lambda のみ、EC2 Instance SP は EC2 のみ
- データ層は **専用の Reserved Instance / Reserved Node** が必要（RDS RI / ElastiCache Reserved Node / Redshift Reserved Node）
- ダッシュボードで「Savings Plans Utilization」を見て安心しても、RDS はカバーされていないので別途チェック

## サイズフレキシビリティの誤認

- **RDS RI は同一ファミリー内でサイズフレキシブル**（MySQL / MariaDB / PostgreSQL / Aurora / Oracle BYOL）
- 例: `db.m6g.4xlarge` の 1 RI は `db.m6g.2xlarge` 2台分として機能する（Normalization Units ベース）
- ダウンサイズしても RI が完全に無駄になるわけではない。使用率が段階的に下がるだけ
- ただし別ファミリー（m6g→r6g）や別エンジン（MySQL→Oracle）間では適用されない
- EC2 RI も Standard RI かつ Regional scope ならサイズフレキシブル、Zonal は不可

## Cost Explorer で見えるタイミング

- **All Upfront**: 購入月に全額計上、翌月から使用料がほぼ $0 になる → 月次推移で一時スパイク発生
- **Partial Upfront**: 購入月に一部計上 + 月額 RI 料金
- **No Upfront**: 月額 RI 料金のみ
- 月次グラフで突然の急増があり、該当サービスで `HeavyUsage:*` USAGE_TYPE が出ていたら RI 購入と判断

## 購入順序の鉄則

1. **ゴミ掃除を先に行う** — 未使用リソース削除、停止中 EC2 整理、右サイジング
2. ワークロードのベースライン（steady state）を 60〜90 日観測
3. Savings Plans（Compute SP）でベースラインをカバー
4. データ層は個別 RI で追加購入
5. 最後にバースト分は On-Demand で対応

**早すぎる RI 購入はダウンサイズを阻害する** ため、施策の順序で最優先ではない。

## 購入したのに効いていないパターン

- **停止中インスタンスに RI が当たっている** — RI は稼働中のみに適用、停止中は機能しない
- **リージョン不一致** — RI はリージョンに紐づく。マルチリージョン展開では各リージョンで購入必要
- **プラットフォーム不一致** — Linux RI は Windows インスタンスに効かない、Aurora RI は PostgreSQL RDS に効かない
- **OS/テナンシー不一致** — Dedicated 指定の RI は Shared に効かない
- `describe-reserved-*-instances` で `State=active` の一覧と稼働中インスタンスの属性を突合して確認

## ダウンサイズ提案時のチェック

- 対象インスタンスに RI が当たっているかを先に確認
- RI が当たっている場合:
  - サイズフレキシブル適用可なら、ダウンサイズ後も RI は部分的に効く（効率低下）
  - 不可なら RI 満期まで待つか、RI Marketplace で売却
- Savings Plans（Compute SP）がかかっている場合、ダウンサイズ分は他の EC2 に吸収されるので影響小

## All Upfront の会計上の注意

- 購入月に全額が経費計上されるので、月次予実管理で「予算超過」に見える
- 経理・財務チームと事前に合意しておくと安全
- 年次換算で割引効果を説明する（月次では比較不能）

## レポート記載時の注意

- RI 購入による「月額削減」と書くと誤解を招く。正しくは「年間 TCO 削減」
- `月額削減 = 購入前の使用料 - 購入後の使用料` だが、前払い分を月割しないと実質的な削減額にはならない
- 1年 All Upfront なら `前払い額 ÷ 12` を月額コストとして加算した比較を併記する

## RI のタイプ不一致による空転（過去のダウンサイズ放置）

過去に稼働インスタンスをダウンサイズしたあと、RI を更新せず放置するパターン。RI が予約しているタイプと現在稼働中のタイプが一致せず、HeavyUsage の月額が空転コストとして発生し続ける。

検出手順:

1. `aws ec2 describe-reserved-instances --query 'ReservedInstances[?State==\`active\`]'` で active な RI のタイプと台数を取得
2. `aws ec2 describe-instances` で稼働中の InstanceType の集計を取る
3. RI のタイプごとに、稼働中で同タイプが何台か突合する
4. RI 台数 > 稼働台数 の差分が空転している可能性

サイズフレキシブルが効く場合（Standard RI かつ Regional scope）は完全な空転にはならず、Normalization Units で部分適用される。それでも稼働中ファミリーが完全に違うなら（例: r5 → c5）効かない。

判断:

- 残期間が短い（1 ヶ月以内）: 満期を待つ。Marketplace 売却の効果が薄い
- 残期間が長い: ファミリー一致なら次のサイズに集約、不一致なら諦めて満期待ち。**満期後は RI 更新せず Savings Plans に切替**

## RI 投資の正味の利益検証

「RI で月 -$X 削減」という数字をそのまま受け取らない。前払い額に対して年間で実利益が出ているかは別に検算する。

- RI コスト: $A（All Upfront 1 年なら一括）
- Pre-RI の対象部分の月額（時間料金 + 関連オンデマンド）: $B/月
- Post-RI の対象部分の月額（残るストレージ・バックアップなど時間料金以外）: $C/月
- 年間の実利益: `($B - $C) × 12 - $A`

これがマイナスだと、RI 投資が元を取れていない可能性がある。原因の代表例:

- 購入時に過剰な台数を見積もった
- 購入直後にダウンサイズ・廃止した（RI が空転）
- そもそもの割引率の見積もりが甘かった（Standard RI All Upfront は通常 30〜40% 引き、それ以上を期待していた）

実績の月額削減（-$X/月）が小さく見え、年換算で $A に届かない場合は、購入時の見積もりと突合して原因を特定する価値がある。レポートには見かけの「月 -$X」だけでなく、年間 TCO 比較も併記する。

## 用語メモ

- **時間料金（Hourly / オンデマンド料金）**: AWS の EC2 / RDS が何もしないと自動で発生する時間あたり課金（例: $0.50/h × 730h ≈ $365/月）。RI を入れるとこの分が割引、All Upfront なら $0 になる
- **HeavyUsage:\<type\>**: Cost Explorer の USAGE_TYPE で表現される RI 月額（Standard RI Partial Upfront などの月額分）。RI 効果が反映されていることのサイン
- **Amortized Cost**: 前払いを契約期間で按分した実効コスト
- **Unblended Cost**: 請求発生ベース（前払いは契約月に全額計上）
- **空転している RI**: 購入したが、対応するインスタンスが稼働していないため使われていない RI。HeavyUsage は出るが効果がない

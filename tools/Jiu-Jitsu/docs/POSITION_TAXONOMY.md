# BJJ Position Taxonomy

この文書は「このゲームで発生し得る体勢」を定義する。目的は全BJJをいきなり3D実装することではなく、全体分類を先に固定し、未分類・未実装の体勢がランダム生成やシナリオに混ざらないようにすること。

## 根拠

- IBJJF Rules Book v6.1 は、ガードパス、ニーオンベリー、マウント、バックコントロールなどの採点体勢を定義している。
- 初心者向けの基本整理では、クローズドガード、サイドコントロール、フルマウント、バックコントロールが基本4ポジションとして扱われる。
- ポジション階層の整理では、ガード、サイド、ニーオンベリー、マウント、バックに加え、オープンガード、ハーフガード、ノースサウス、タートル等を別カテゴリとして扱う。
- 既存の `docs/research-grappling.md` には、ガード体系、パス体系、フロントヘッドロック、足関節、タートル、エスケープ、ルールセット差を調査バンクとして集約している。

## 分類レベル

### Level 1: 全BJJポジションファミリー

`js/positionCatalog.js` の `BJJ_POSITION_FAMILIES` を正とする。ここには「既知だが未実装」の体勢も含める。

現在のファミリー:

- 立位/組み手
- クローズドガード
- オープンガード/パス入口
- ハーフガード/ニーシールド
- バタフライガード
- デラヒーバ/リバースデラヒーバ
- スパイダー/ラッソーガード
- Xガード/SLX
- ガードパス/上のベース
- サイドコントロール
- ノースサウス
- ニーオンベリー
- マウント/高いマウント
- テクニカルマウント
- バックコントロール
- タートル/背中露出
- フロントヘッドロック
- クルシフィックス
- 腕十字
- 三角絞め
- オモプラッタ
- キムラ/肩固め系
- ギロチン
- ダース/アナコンダ
- アシガラミ/足関入口
- インサイドアシ/サドル/411
- アウトサイドアシ
- 50/50/バックサイド50/50
- 足関節フィニッシュ

### Level 2: 実装済み pose role

`js/positionCatalog.js` の `POSE_ROLE_CATALOG` を正とする。`poseSpecs.js` の各 pose は必ずこの role catalog に属する。

実装済み role は、3Dで読める最低品質を満たしたものだけにする。名前だけ足してポーズ・支持基底・接触点・力方向が未定義のものは不可。

### Level 3: 許可された赤/青 role ペア

`js/positionCatalog.js` の `ALLOWED_ROLE_PAIR_RULES` を正とする。

例:

- `back-control-top / seated-front`: バック確保
- `mount-top / supine-bottom`: マウント確保
- `side-control-top / side-control-bottom`: サイド固定
- `closed-guard-bottom / closed-guard-top`: クローズドガード
- `guard-armbar-attacker / guard-armbar-defender`: ガード腕十字
- `triangle-attacker / triangle-defender`: 三角

このリストにない role ペアは、たとえ両方の pose が存在してもゲーム内で使ってはいけない。

## 実装ルール

新しい体勢を追加する順序:

1. `BJJ_POSITION_FAMILIES` にファミリーを追加または既存ファミリーを選ぶ。
2. 3D化する場合だけ `POSE_ROLE_CATALOG` に role を追加する。
3. `poses.js` に関節角を追加する。
4. `poseSpecs.js` に支持基底、接触点、荷重、力方向を追加する。
5. `ALLOWED_ROLE_PAIR_RULES` に赤/青の許可ペアを追加する。
6. `_audit.html` で視認し、`scripts/validate-data.mjs` を通す。
7. その後に `techniques.js` の scenario から参照する。

禁止:

- `poses.js` だけ追加して scenario から使う。
- role catalog にない role を `poseSpecs.js` に書く。
- `ALLOWED_ROLE_PAIR_RULES` にない赤/青ペアを scenario に書く。
- 「何となく近い」既存 role を別ポジションに流用する。
- ハーフガード、ニーオンベリー、足関など未実装ファミリーを、既存のマウント/サイド role で代用する。

## 現在の実装範囲

実装済み:

- 立位
- バックコントロール
- マウント
- クローズドガード
- オープンガードからのパス入口
- サイドコントロール
- タートル/背中露出
- 腕十字
- 三角

既知だが未実装:

- ハーフガード/ニーシールド
- バタフライ、DLR/RDLR、スパイダー/ラッソー、X/SLX
- ニーオンベリー、ノースサウス、テクニカルマウント
- フロントヘッドロック、クルシフィックス
- オモプラッタ、キムラ、ギロチン、ダース/アナコンダ
- アシガラミ、サドル/411、50/50、各種足関節

未実装ファミリーは分類には存在するが、`implemented: false` のため、現在の pose role と scenario には使えない。

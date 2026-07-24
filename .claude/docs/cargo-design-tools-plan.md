# cargo 設計診断ツール群 企画書

作成日: 2026-07-07 / 対象: `tools/` 配下に新規作成する cargo サブコマンド群
実装担当: codex (`codex exec`)。本書が codex への発注仕様の原本。

**公開方針**: 当面 workspace_2026 内のローカルツールとして保持する（`publish = false`）。
成果物・作業ファイルはすべて workspace_2026 内に収める。結果が良ければ外部公開を再判断する。

## 1. 目的

cargo-coupling (https://github.com/nwiizo/cargo-coupling) で確立した
「設計リスクをスコア化し、baseline・CI gate・AI 向け出力・blind spot 宣言まで持つ」
という型を、coupling 以外の設計観点に横展開する。単発 lint ではなく
**設計リスクの継続観測ツール群**として揃える。

## 2. 共通仕様テンプレート（全ツール共通・cargo-coupling 準拠）

### CLI 表面

```
cargo <name> [PATH]              # デフォルト: 重要 issue のみ表示
cargo <name> --summary           # サマリのみ
cargo <name> --all               # Low 含む全 issue
cargo <name> --json              # 機械可読出力
cargo <name> --ai                # AI coding agent 向け出力（修正手順つき）
cargo <name> --baseline <ref>    # git ref との差分
cargo <name> --check             # CI gate（exit 1）
cargo <name> --check --baseline <ref> --fail-on=<sev>   # ratchet gate
cargo <name> --blind-spots       # 未解析領域の宣言を全表示
cargo <name> --japanese | --jp   # 日本語出力
```

- issue key は `(issue_type, source, target)` 形式で安定させ、baseline diff に使う
- severity は Critical / High / Medium / Low の 4 段階 + 総合 grade (A–F)
- `--json` / `--ai` は blind-spot manifest を常に含める
- 設定は `<name>.toml`（例: `boundary.toml`）で allow/deny・閾値を上書き

### スコアモデル

各ツールは cargo-coupling の Strength/Distance/Volatility に相当する
**2〜3 軸の直交する次元**を定義し、その積・組合せで severity を導出する
（軸は各ツール企画に記載）。「検出した/しない」の二値にしない。

### 実装スタック・品質基準

- **パーサは `ra_ap_syntax` (rowan) で統一**（ローカル参照実装: `tools/rbp-lint`、ra_ap_syntax 0.0.331 + rowan 0.16）。理由:
  1. ロスレス CST なのでコメントが木に残り、**suppress 注釈**（`// <name>-allow: <issue_type>`）を全ツール共通仕様にできる（syn はコメントを落とす）
  2. エラー耐性パーサなので、コンパイルが通らない状態のコードも解析できる（CI gate 前の手元実行で有利）
  3. 正確な TextRange が取れ、diagnostic の行・列表示と `--ai` 出力の精度が上がる
  - 例外: cargo-api-drift のシグネチャ比較のみ `syn 2` 併用可（typed AST での比較が簡潔なため）
  - cargo-coupling（syn 実装）はスコアモデル・baseline・出力設計の参照。パーサ層の書き方は rbp-lint を参照する
- `cargo_metadata` + `walkdir` + `rayon` + `clap 4 derive` + `serde/serde_json` + `thiserror 2`
- git 履歴が要る場合は `git` コマンド呼び出し（cargo-coupling `volatility.rs`/`history.rs` 方式）
- リポジトリ規約: production コードに `.unwrap()` 禁止、`cargo fmt && cargo clippy -- -D warnings && cargo test` が green
- 各ツールに tempfile ベースの統合テスト（違反を含む fixture crate を生成して検出を検証）
- ドキュメントは各ディレクトリ README.md のみ

### 共通基盤の方針（rule of three）

Wave 1 の 2 本は**独立実装**とし、cargo-coupling から必要コードを参照移植する。
3 本目着手前に baseline store / severity・grade / --ai renderer / blind-spot manifest を
共通 crate `design-gate-core`（tools/ 配下, workspace 化）へ抽出するか判断する。
先に共通 crate を作る抽象先行は禁止。

## 3. ツール企画（8 本・優先順）

### 3.1 cargo-boundary — アーキテクチャ境界の破り検出

- **コンセプト**: DDD / Clean Architecture / レイヤード構成の「依存してはいけない方向」への依存を検出する
- **検出対象**:
  - layer 違反（`domain -> infrastructure` など、boundary.toml で層と許可方向を宣言）
  - internal / private 相当モジュールの越境参照
  - `pub` の漏れ（外部から実際には参照されていない過剰 `pub`、`pub(crate)` にすべきもの）
  - 禁止 import（例: domain 層での `sqlx` / `reqwest` 直接 use）
- **スコア軸**: 違反の深さ（何層飛び越えたか）× 違反箇所数 × 対象モジュールの volatility
- **固有 CLI**: `--layers`（推定した層構造の表示）、`boundary.toml` 未定義時はディレクトリ名ヒューリスティクスで層を推定し blind spot として宣言
- **競合と差別化**: Rust に ArchUnit 相当の定番なし（archunit_rs は停滞）。cargo-modules は可視化のみ。score + ratchet + --ai が差別化
- **リスク**: 層推定の誤検出。→ 推定根拠を必ず出力し、toml 宣言を推奨する導線にする

### 3.2 cargo-error-map — エラー設計の可視化

- **コンセプト**: エラー型の伝播グラフを描き、エラー設計の崩れを検出する
- **検出対象**:
  - `anyhow` が library 層（lib.rs から到達可能な public API）に漏れている
  - `thiserror` enum の肥大化（variant 数閾値、無関係ドメインの混在）
  - `?` 伝播チェーンで `context()` / `with_context()` が一定深さ以上欠落
  - `unwrap`/`expect`/`panic!` が境界層（main / handler / test）以外に残存
  - `Box<dyn Error>` の public API 露出
- **スコア軸**: 漏れの到達範囲（public API まで届くか）× 発生箇所の層 × 呼び出し頻度（fan-in）
- **固有 CLI**: `--graph`（エラー伝播グラフを text/DOT で出力）
- **競合と差別化**: clippy は点の検出のみ、rbp-lint は unwrap 地点検出のみ。**伝播グラフ + 層判定**が新規
- **リスク**: 型解決なし（syn のみ）での伝播追跡は近似になる。→ 追えなかった経路を blind spot 宣言

### 3.3 cargo-feature-doctor — Cargo feature の事故検出

- **コンセプト**: feature フラグ起因の「手元では気づけない壊れ方」を静的に見つける
- **検出対象**:
  - default feature の意図しない伝播（依存クレートの default を切り忘れ）
  - feature 組合せ爆発（2^n の警告と、相互排他 feature の未宣言）
  - `#[cfg(feature)]` 分岐のうち CI でコンパイルされていない経路（workflow ファイル参照は optional）
  - optional dependency の型が public API に漏れている（feature オフで下流が壊れる）
  - additive でない feature（機能を削る feature）の検出
- **スコア軸**: 影響 feature 組合せ数 × public API 露出有無 × 依存側クレート数
- **固有 CLI**: `--matrix`（feature 組合せとカバー状況の表）、`--suggest-hack`（cargo-hack 向けコマンド生成）
- **競合と差別化**: cargo-hack は全組合せ**ビルド**（重い）、cargo-udeps/machete は未使用検出のみ。**ビルドせず静的にリスク列挙**が差別化
- **リスク**: cfg 解決の網羅は不可能。→ 解析した cfg 集合を blind spot に明記

### 3.4 cargo-async-smell — async 運用事故リスク検出

- **コンセプト**: clippy より「本番運用の事故」に寄せた async 設計臭の検出
- **検出対象**:
  - `MutexGuard`（std / parking_lot）を `.await` 越しに保持
  - async fn 内の blocking I/O（`std::fs` / `std::net` / `reqwest::blocking` / `std::thread::sleep`）
  - `tokio::spawn` の unbounded 生成（loop 内 spawn で JoinHandle 破棄）
  - キャンセル不能タスク（select/timeout 配下にない長期タスク、Drop で leak する spawn）
  - timeout なしの外部通信（`connect` / `send` 系呼び出しに timeout ラップなし）
- **スコア軸**: 事故時影響（デッドロック > 飢餓 > レイテンシ）× 発生条件の踏みやすさ × 該当コードの volatility
- **固有 CLI**: `--runtime tokio|async-std|smol`（既定 tokio、他は blind spot）
- **競合と差別化**: clippy の `await_holding_lock` 等は個別 lint。**運用事故シナリオ単位のスコア + ratchet** が差別化
- **リスク**: false positive が最も出やすいツール。→ Wave 2 冒頭で共通 suppress 注釈（§2 の rowan コメントベース方式）を先に整備。`.await` 越しの guard 保持検出は式レベルの走査が必要で、ra_ap_syntax 採用が最も効くツール

### 3.5 cargo-trait-surface — 抽象境界の品質診断

- **コンセプト**: cargo-coupling が「結合の距離」なら、こちらは「抽象境界の品質」を見る
- **検出対象**:
  - 巨大 trait（メソッド数・関連型数の閾値超過）
  - 実装が 1 つしかない過剰抽象（テスト以外に impl が存在しない trait）
  - object safety を壊す変更リスク（dyn 利用箇所と非 object-safe メソッドの共存）
  - blanket impl の影響範囲（coherence 事故の予備軍）
  - mock 不能な境界（具象型直依存で trait 境界がない外部 I/O 呼び出し）
- **スコア軸**: 抽象の過不足方向（過剰/不足）× 利用箇所数 × public 露出
- **固有 CLI**: `--trait <Name>`（単一 trait の詳細診断）
- **競合と差別化**: 既存ツールほぼなし。cargo-coupling と対になる位置づけ
- **リスク**: 「実装 1 つ = 過剰抽象」は将来拡張の意図と衝突。→ severity を Low 起点にし、`trait-surface.toml` で intent 宣言可能にする

### 3.6 cargo-test-gap — テストが薄い危険箇所の特定

- **コンセプト**: 「どこからテストを書くべきか」を churn × 複雑度 × 露出 × カバレッジで順位付け
- **検出対象・入力**:
  - git churn（変更頻度・直近性）
  - 関数複雑度（分岐数ベースの近似 cyclomatic）
  - public API か否か、エラーパスか否か
  - `cargo llvm-cov --json` の結果（**optional 入力**。無ければ「テスト関数からの到達可能性」で近似し、その旨を blind spot 宣言）
- **スコア軸**: risk = churn × complexity × exposure ÷ (coverage + 1)
- **固有 CLI**: `--top N`（危険箇所ランキング）、`--llvm-cov <path>`（カバレッジ JSON 取り込み）
- **競合と差別化**: llvm-cov は網羅率のみ、cargo-mutants は重い。**リスク合成ランキング + PR 差分モード（--baseline）** が差別化
- **リスク**: 4 入力の合成で説明可能性が落ちる。→ 各軸の素点を必ず併記

### 3.7 cargo-api-drift — public API 差分の SemVer リスク判定

- **コンセプト**: git ref との差分から公開 API 変更を breaking / risky / safe に分類
- **検出対象**: 公開型・trait・関数シグネチャ・feature・derive（`Clone` 剥がし等）・error enum variant の追加削除
- **スコア軸**: 破壊の確度（breaking 確定 / risky = non-breaking だが下流を壊しうる）× API の推定利用度
- **固有 CLI**: `--against <ref>`（既定 main）、`--changelog`（CHANGELOG 断片生成）
- **競合と差別化**: **cargo-semver-checks が強競合**（rustdoc JSON ベース・lint 100 本超）。差別化は (1) rustdoc 不要で git diff + syn のみの高速動作 (2) semver 違反でない「risky」段階の検出 (3) --ai / changelog 生成
- **リスク**: 8 本中で最も既存被りが大きい。**Wave 3 着手前に「semver-checks ラッパー + risky 層追加」へ方針転換するか再判断する**
- **再判断 (2026-07-07)**: 独立ツールとして実装する。ただし strict semver audit は cargo-semver-checks の領分と blind spot / README に明記し、本ツールは (1) rustdoc 不要の git diff + CST 高速動作、(2) semver 的 minor でも下流を壊しうる「risky」層、(3) --changelog 生成、(4) design-gate-core 家族との CI 統合に特化する

### 3.8 cargo-agent-context — AI agent 向けリポジトリ要約生成

- **コンセプト**: Rust repo の構造・リスク・作法を AI coding agent 向けに要約する
- **出力内容**: module graph 要約、重要型と public API 一覧、test/build コマンド、known risks（**兄弟ツール群の --json 出力を取り込んで統合**）、blind spots
- **出力先**: `--format agents-md | claude-md | markdown`（AGENTS.md / CLAUDE.md 断片 / 単体レポート）
- **スコア軸**: なし（診断ツールではなく統合レポータ）。ただし取り込んだ各ツールの grade を集約表示
- **競合と差別化**: repomix 等は全文圧縮、/init は汎用。**Rust 特化 + 設計リスク統合**が差別化
- **リスク**: 単体価値が薄い。→ **意図的に最後**。他 7 本の --json スキーマが安定してから作る

## 4. 追加ツール提案（本企画の次候補・未着手）

| 名前 | コンセプト | 既存 8 本との関係 |
|---|---|---|
| cargo-panic-surface | public API から panic 到達地点（unwrap/expect/panic!/index/整数 div）への **call graph 経路**を列挙しスコア化 | rbp-lint は「地点」検出。こちらは「経路と到達可能性」。error-map と基盤共有可 |
| cargo-invariant | primitive obsession 検出。生 `String`/`u64` の ID 混在、newtype 欠落、"parse, don't validate" 違反（検証済みを型で表現していない） | trait-surface が抽象の過不足なら、こちらは**型不変条件**の過不足 |
| cargo-obs-lens | observability カバレッジ。エラーパスに `tracing` 記録なし、async 境界の span 欠落、高 cardinality なフィールド、`log` と `tracing` の混在 | async-smell の隣接領域。SRE 業務との親和性が高い |
| cargo-config-drift | workspace 内 crate 間の `[lints]`/edition/MSRV/profile/依存バージョン指定のドリフト検出。workspace 継承未使用の指摘 | 唯一 syn 不要（toml のみ）で MVP 最小。CI gate 需要が明確 |

推奨: 8 本完了後の最初の 1 本は **cargo-invariant**（設計リスク路線の一貫性が最も高い）か、
すぐ効く小物が欲しければ **cargo-config-drift**（1 日規模）。

## 5. 実装計画（codex 委任）

### Wave 構成

| Wave | ツール | 目的 |
|---|---|---|
| 0 | — | crates.io 名前衝突の確認（8 名 + 追加 4 名）。**完了 (2026-07-07)**: 12 名中 11 名は未使用。`cargo-invariant` のみ衝突 → 着手時に改名する（候補: cargo-newtype-lint 等） |
| 1 | cargo-boundary, cargo-error-map | **完了 (2026-07-07)**: codex 初版 → 5 agents × 2 レビュー（blocking 多数・全て fixture 再現済み）→ codex 差し戻し → 再検証 green。学びは下記「Wave 1 の学び」 |
| 2 | design-gate-core 抽出判断 → cargo-feature-doctor, cargo-async-smell, cargo-trait-surface | **完了 (2026-07-07)**: core 抽出 → 3 本並行実装 → 各 5 agents レビュー（全ツールで blocking 検出）→ 差し戻し → 家族横断バッチ（core suppress バグ・boundary 外れ値是正）。学びは「Wave 2 の学び」 |
| 3 | cargo-test-gap, cargo-api-drift（方針再判断あり）, cargo-agent-context | 外部入力（llvm-cov, git diff, 兄弟 JSON）を持つ応用層 |

### Wave 1 の学び（Wave 2 以降の codex prompt に必ず反映する）

1. **CST 使用は仕様に書くだけでは守られない**: codex 初版の cargo-boundary は ra_ap_syntax を入れつつ文字列走査で実装した（ブロックコメント誤検出・ジェネリクス内参照全滅の根本原因）。prompt に「受け入れテスト: ブロックコメント内コードを検出しない fixture / `Vec<crate::x::Y>` の参照を検出する fixture」を最初から含める
2. **パス・キーは初版から repo 相対で**: 絶対パスキーは baseline diff を構造的に壊す（同一コミット diff で unchanged=0 になる）。「同一コミット baseline diff で new=0/resolved=0」の回帰テストを完了条件に入れる
3. **可視性判定は `Visibility::visibility_inner()`**: `visibility().is_some()` は pub(crate) を public 扱いする
4. **negative fixture を issue type ごとに必須化**: 初版の統合テストは正常系のみで、blocking 級の精度バグを全て素通しした
5. **severity 分布の妥当性確認を dogfooding に含める**: 「実 repo で全 issue が Critical」のような退化分布は較正バグ（Critical 閾値は合計 9 以上に設定した）
6. **codex sandbox では cargo registry cache に書けず新規依存追加が失敗することがある**: `.gitignore` 尊重は ignore crate ではなく `git check-ignore` で実装された。新規依存が必要な仕様は事前に把握しておく
7. **codex exec はプロンプトを heredoc stdin (`-`) で渡す**: 引数渡し + background は stdin 待ちでハングする

### Wave 2 の学び（Wave 3 の codex prompt に反映する）

1. **仕様に学びを書いても検出器の精度バグは防げない**: Wave 1 の学びを全部 prompt に入れた結果、構造・CLI・core 利用の一貫性は「byte-for-byte」評価まで改善したが、検出ロジックの精度バグ（tokio Mutex 誤検出、blanket impl 判定の狭さ、cfg 極性の取り違え等）は 3 ツールとも review の fixture 実測でしか捕まらなかった。Wave 3 では**検出器ごとに adversarial fixture（誤検出させたい正しいコード / 見逃させたい悪いコード）を仕様に列挙**し、codex にそれを先にテスト化させる
2. **「dogfooding で issue 少数」は精度の証明にならない**: kuroko で async-smell 1 件 / trait-surface 3 件は、対象 repo が検出対象パターンを踏んでいない（または全部偽陽性）だけだった。dogfooding には検出対象を実際に含む repo か合成 stress fixture を追加する
3. **識別子ベースのキーは必ず修飾する**: ベア名キーの衝突（dedup での実 issue 消失・baseline の偽 diff）が 3 ツールで独立に再発。キーは `rel_path:Type::item` を家族標準とし、リスト由来の target は join 前にソートする
4. **属性は path() で判定する**: `doc(cfg(...))` の誤認、async_trait / cfg(test) の直接付与の見逃しはすべて「属性の raw テキスト検索」起因。attr.path() の確認を必須とする
5. **suppress 機構のバグは家族全体を汚染する**: core の item_start_before_line が隣接アイテムへ漏れるバグは 5 ツール全部に影響した。core の変更には専用回帰テストを必須とする
6. **レビュー 5 体（code/rust/simplify/cli-ux/codex）× fixture 実測の体制は機能している**: Wave 1・2 とも、codex の自己申告 green と一次検証（fmt/clippy/test + dogfooding）を通過した実装から、レビューが再現可能な blocking を毎回発見した。この工程は省略しない

### Wave 3 の学び

1. **core の Grade / IssueKey は「離散的な違反件数」モデル前提**: test-gap のような「全関数をランキングする」型のツールに core の絶対閾値 Grade をそのまま使うと全 repo が F になる（実測）。ランキング型は割合ベースの相対 Grade に正規化が必要。IssueKey.target に揮発的なバケットラベル（complexity-medium 等）を入れると ratchet gate が偽 FAIL する — target は安定した同一性情報のみ
2. **adversarial fixture 列挙方式は「列挙した箇所は守られ、列挙しなかった箇所が全滅」**: api-drift は仕様に列挙した 12 ケース（non_exhaustive / derive / trait default 等）は全部正しく、列挙しなかった struct フィールド追加・ジェネリク比較・re-export 差し替え・const/static が全部欠落した。fixture リストは「変更カテゴリ × アイテム種別」のマトリクスで機械的に洗い出す
3. **「diff ツール」に家族の baseline 語彙を輸入するとハリボテが生まれる**: api-drift の resolved/unchanged は構造的に常に 0 のダミーだった。共通仕様の適用は「そのツールの意味論に存在する機能か」を先に判定する

### 家族横断の残項目（次回の family batch で対応）

- design-gate-core: BrokenPipe を quiet exit 0 に（全ツールで `| head` が error になる）/ 「1 issues suppressed」の単数形 / Breakdown 表示順の共通ヘルパー（Critical→Low の明示順）/ hidden-low hint の core 化 / sort_dedup_by_key の利用統一
- error-map / trait-surface / feature-doctor: hidden-low hint 追加。async-smell / trait-surface: select_mode の core 版へ移行
- 全ツール: Error::Baseline(String) の source チェーン喪失（Baseline(#[source] Box<Error>) 化）

### codex 実行方法

- 1 ツール = 1 実行: `codex exec --full-auto -C tools/<name> "<本書 §2 + 該当ツール節 + 検証ゲート>"`
- 並行は最大 2〜3 本（cargo build の CPU 競合を避ける）
- prompt には必ず含める: 共通仕様 §2 全文 / 該当ツールの企画節 / 参照実装 2 つのパス（スコア・baseline・出力設計 → `~/ghq/github.com/nwiizo/cargo-coupling`、rowan パーサ層と lint 構造 → `tools/rbp-lint`）/ 検証ゲート

### 検証ゲート（各ツール完了条件）

1. `cargo fmt && cargo clippy -- -D warnings && cargo test` green
2. fixture crate に対する統合テストで各 issue type の検出を確認
3. dogfooding: `cargo-coupling`, `kuroko`, `rustlean` の実 repo に実行し、出力が説明可能であること
4. `--json` / `--ai` / `--baseline` / `--check` / `--blind-spots` の動作確認
5. README.md（Quick Start + スコアモデル + blind spot 方針）
6. Claude 側で `home-code-reviewer` / `home-simplify-reviewer` / `home-codex-reviewer` + `home-rust-reviewer` / `home-cli-ux-reviewer` の並行レビュー → `home-fix-review-comments`

## 6. リスク・未決事項

- **名前衝突**: crates.io 未確認（Wave 0 で解消）。特に boundary / feature 系は一般名詞で衝突しやすい
- **false positive 管理**: 全ツール共通の suppress 注釈（`// <name>-allow: <issue_type>`、rowan のコメント保持を利用）を Wave 2 冒頭で確定する
- **メンテ負荷**: 8 本 + core crate。ra_ap_syntax はリリース頻度が高く API が動くため、全ツールでバージョンを揃えて pin し、パーサ依存箇所を各ツールの `parser` モジュールに閉じ込める（rbp-lint と同時に上げる）
- **cargo-api-drift の存在意義**: cargo-semver-checks との差が「risky 分類 + 速度」だけなら独立ツール化しない判断もある（§3.7）

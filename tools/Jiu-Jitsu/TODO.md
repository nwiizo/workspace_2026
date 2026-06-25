# TODO — 柔術ディフェンス道場

タスク全件と状態。要件定義・技術選定は [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)、ポジション分類は [docs/POSITION_TAXONOMY.md](docs/POSITION_TAXONOMY.md)、詳細設計・研究は [docs/DESIGN.md](docs/DESIGN.md)、調査バンクは
[docs/research-grappling.md](docs/research-grappling.md) / [docs/research-strength-anatomy.md](docs/research-strength-anatomy.md)。

状態凡例: ✅完了 / 🟡実装中 / 🔵調査完了・実装待ち / ⬜未着手

## コア（波0：開発エージェント）

実装再開順は [docs/REQUIREMENTS.md#9-実装順](docs/REQUIREMENTS.md#9-実装順) を正とする。特に 3D ポーズ可読性が未達の間は、ゲーム性追加より #1/#11 を優先する。

- [ ] 🟡 **#1 ポーズの向き修正**（背中合わせ/直立の解消）
  - `poses.js` を座標規約付きで再構築・`_audit` で目視検証済み。`techniques.js` とのポーズID整合は #11 で要確認。
	  - 現状評価: 方向・上下関係は改善。`poseSpecs.js` で各ポーズの役割・支持基底・接触点・力の方向を定義し、`_audit` と `validate-data` の検証対象にした。`validate-data` は root 座標の高さレンジ、scenario ペアの水平距離、マウント/ガード上/サイド上/腕十字/三角の高さ関係、バック/マウント/ガード/サイド/下からの極めの向き関係、相手に荷重する pose の支持中心と力ベクトルが相手方向へ向いていることも落とす。サイドコントロール、ガードパス、マウント腕隔離、ガード腕十字・三角の `poseSpecs.vector` を相手へ圧/制御をかける方向へ再調整済み。`_audit` の重心投影/支持エリア/力ベクトル補助表示で、上側の人が支点から外れていないかを確認できるようにした。力ベクトルは root ではなく支持中心から出し、相手に荷重する pose は支持中心→相手 root の荷重線も表示する。`validate-data` は `invalidAuditBiomechanics` で監査表示の退行も落とす。ただし腕十字の肘支点、三角の膝裏/足首ロック、細かい接触はプリミティブ骨格ではまだ記号的なので継続修正する。
	  - `positionCatalog.js` を追加し、全BJJの主要ポジションファミリー、実装済み pose role、許可された赤/青 role ペアを分離した。`validate-data` は role が実装済み family に属すること、scenario ペアが `ALLOWED_ROLE_PAIR_RULES` にあることを検証する。Game 実行時も未許可ペアを拒否する。
	  - 首/肩/肘/股関節/膝/足首の関節角レンジを `validate-data` に追加。首66度、肩172度、肘/膝140度級など、簡易リグで反転して見えるポーズ角を安全側へ丸めた。膝は [docs/HUMAN_BODY_MODEL.md](docs/HUMAN_BODY_MODEL.md) の規約に従い、`shin*.x` の負方向屈曲を禁止してヒンジ関節として扱う。`anatomy.js` を追加し、検証と `fighter.js` のレンダリング時 clamp で同じ制約を共有する。
- [ ] 🟡 **#2 連続フロー：防御→継続攻撃の1本のロール**
  - `game.js` に正解後の自動前進（`ADVANCE_DELAY`）・連続正解ボーナス（streak）実装。防御/攻撃/ミックスの3フォーカスでランダムロールを生成。
  - 各シナリオに `timeLimitSec` と `pressure.{early,urgent}` を追加し、相手の能動アクションを局面別に表示。
  - `setup` 中は選択肢と数字入力をロックし、`attack` ポーズへ切り替わった判断フェーズから回答できるようにした。`validate-game-logic.mjs` で攻撃前入力が無効なことを検証。
  - 各シナリオに `readCues`（読む線）を追加し、首・肘・腰・フレームなど、力関係として見るべき対象を短いチップで表示。`scripts/validate-data.mjs` で全シナリオの存在と長さを検証。
  - 回答後フィードバックに「読めた線 / 見落とした線 / 遅れた線」を表示し、終了レビューにもその局面で見た `readCues` を残す。`validate-game-logic.mjs` で履歴保存を検証。
  - 防御シナリオにクローズドガード内の姿勢防御を追加。既存の `blueGuardPass` / `redGuardOpened` ポーズを使い、下からの腕十字/三角を消してパスへ進む局面を増やした。
  - 正解 choice の `next` 候補を使い、次スロットを直前結果と矛盾しにくい局面へ差し替える初期シナリオグラフを実装。`next` は `{ id, weight }` 形式も受け付け、自然な展開ほど出やすくした。差し替え時は過去に解いた局面と未来スロットの重複を避ける。
  - 正解 choice の `reaction` を追加し、回答直後と終了レビューで「相手の反応 → 次局面」の理由を表示。
  - 不正解 choice に `consequence` と `next` を追加し、失敗後も「相手の追撃 → 不利な次局面」としてロールが続くようにした。
  - 正解/不正解/実戦での時間切れで `flow`（流れ）を変動させ、実戦モードの次局面判断時間と混合ロールの `next` 候補重みに軽く反映。ロール終了時に各局面の選択・正誤・反応/追撃・選ばれた次局面・分岐理由・原則・流れをレビュー表示。
  - ロール終了時に `runStats` と流れから次の稽古ポイントを出すコーチ評価を追加。時間切れ、守りから攻めへの接続不足、防御構造不足、主導権維持を分岐して表示。
  - 時間切れは実戦モードのみ有効。時間切れ時はシャッフル後の先頭不正解ではなく、不正解 choice の `next` を見て相手スタイル preferred と現在の流れに沿う悪手結果を重み付きで選ぶ。入門モードは時間制限なし。
  - ロールごとに `ROLL_MISSIONS`（今回の狙い）をランダム付与し、時間切れなし/守って攻めへ/防御正解/連続正解などの達成条件と終了時ボーナスを追加。`scripts/validate-data.mjs` / `validate-game-logic.mjs` で定義と達成判定を検証。
  - ロールごとに `ROLL_TACTICS`（今回の制約）をランダム付与し、生存優先/位置を上げる/極めの連鎖/速いスクランブルで実戦時の判断時間と `next` 重みを変える。`scripts/validate-data.mjs` / `validate-game-logic.mjs` で定義と重み反映を検証。
  - 各 scenario に `opponentActions`（相手の初動）を2件以上追加。現在は 25 初動、3件以上の初動を持つ局面が7件。相手スタイル/戦術制約で出やすい初動が変わり、同じ局面でも attack ポーズ、読む線、pressure 表示、回答後の「初動の読み」cue が変化する。`scripts/validate-data.mjs` / `validate-game-logic.mjs` で定義・pose参照・cue保存・重み反映を検証。
  - `requiresAction` / `forbiddenAction` を追加し、相手初動によって正解選択肢そのものを出し分けられるようにした。バック防御では、絞め手=首防御、腰フック=フックを外して肩を床へ、シートベルト=肩と腰の線をずらす、へ分岐する。マウント脱出では、腕隔離=アッパ、高いマウント=膝肘、脚絡み=足を外して膝肘へ分岐する。攻撃側では、マウント中の橋=先にベース回復、バック中の腰抜け=マウントへ変換、へ分岐する。サイド攻撃では、フレーム回復=マウントへ、背を向ける=バックへ、膝盾=潰し直しへ分岐する。クローズドガード姿勢防御では、姿勢折り=姿勢回復、腰角度=肘を中心線へ戻す、起き上がり=手をマットにつかず腰を制する、へ分岐する。`scripts/validate-data.mjs` で action/state/gi-nogi ごとの正解数、action 参照、action-gated 局面数を検証し、`validate-game-logic.mjs` で初動別正解を検証。
  - `setup` 中は選ばれた初動を先出しせず、`attack` ポーズへ切り替わった判断フェーズで初めて初動タグ・専用 `readCues`・専用 `pressure` を表示する。`validate-game-logic.mjs` で非表示→表示の遷移を検証。
  - リプレイ時は同じ局面の最新回答で上書き。ただし点数・連続正解・流れは初回回答のみ反映し、復習でロール状態が歪まないようにした。
  - `rollState` を追加し、選択結果の `stateEffects` を次局面へ引き継ぐ。`requiresState` / `forbiddenState` で選択肢を出し分け、前局面の結果が次の判断に残るようにした。
  - 各 scenario に `stateBias` を追加し、引き継ぎ状態が噛み合う次局面候補と時間切れ時の追撃候補が出やすくなるようにした。次局面理由にも一致した引き継ぎ状態を残す。
- [ ] 🟡 **#3 オフェンスモード**（正しい攻め手を選ぶ）
  - `ui.js` に role（攻/守）タグ、`index.html` にミックス/防御/攻撃トグル。攻めシナリオ（位置前進→極め）の拡充を継続。
- [ ] 🟡 **#4 ギ/ノーギ 切替**
  - `fighter.setMode`／`Dojo.setUniformMode`／`game._visibleOptions`（giOnly/nogiOnly フィルタ）／UIトグル実装。バック防御でギ=襟防御、ノーギ=手首/前腕ハンドファイトに分岐。
  - `scripts/validate-data.mjs` で gi/nogi それぞれの表示選択肢に正解が1つだけあること、最低1シナリオにモード分岐があることを検証。
- [ ] 🟡 **#5 3Dモデル精密化**（納得できるレベル）
  - 四肢テーパー・胴/肩/首造形・手足ディテール・gi/nogi 見た目実装済み。スクショ反復で詰める。
  - `_audit.html?q=back|guard|triangle` のように監査対象を絞れるようにし、重心投影/支持エリア/支持中心起点の力ベクトル/荷重線と `poseSpecs.js` の接触点ラベルで支持点/接触点のスクショ反復をやりやすくした。表示が邪魔な場合は `markers=off` を使う。
  - 現MVPのプリミティブ骨格では細かい関節接触に限界があるため、ポーズ可読性が安定した後に glTF/IK/物理拘束への移行判断を行う。
  - 人体らしさの次段階は、角度手打ちの追加ではなく IK と skinned glTF リグへの移行で判断する。膝/肘のヒンジ制約と肩/股関節の多軸制約は `docs/HUMAN_BODY_MODEL.md` を正とする。現MVPには floor probe による軽量 grounding を入れ、マット抜け/浮きを抑える。
- [ ] 🟡 **#6 スクショ駆動でプレイ感を作り込み**（プロが楽しい）
  - 入門/実戦の難易度トグルを実装。入門は時間制限なし＋具体的な相手意図、実戦は時間制限あり＋抽象的な相手意図。
  - 相手スタイル（ランダム/プレッシャーパサー/絞めハンター/ガードプレイヤー）を UI で選択可能にし、シナリオ順の優先度・正解後の `next` 優先・実戦モードの相手意図に反映する最小実装を追加。ガードプレイヤーは `closed-guard-posture` も優先。
  - 相手スタイルを `opponentActions` の重みにも反映し、同じ scenario でも「首を狙う」「腰を追う」「姿勢を折る」など初動と attack ポーズが変わるようにした。
  - 流れメーター（優勢/前進/五分/危険/劣勢）を追加し、単発クイズではなく一本のロール内の力関係として見せる。混合ロールでは優勢なら攻め、危険なら守りへ繋がる next を少し優先する。
  - 今回の狙いミッションを追加し、同じ局面列でも「安全第一」「守って攻めへ」「反応を追う」など練習目的が変わるようにした。
  - 今回の制約を追加し、同じ局面列でも「生存優先」「位置を上げる」「極めの連鎖」「速いスクランブル」でテンポと分岐が変わるようにした。
  - 終了レビューに次の稽古カードを追加し、負けた局面を「何を反復するか」へ変換するようにした。
  - ミス/時間切れがあった場合、終了画面から最初の苦手局面と同じ相手初動を先頭にした新しいロールへ入れる導線を追加。通常のランダム再ロールとは分け、固定問題集化せず反復練習だけを補助する。`validate-game-logic.mjs` で scenario/action の固定を検証。
  - 通常の再ロールでは、直近のミス/時間切れから最頻の `readCues` を1つ拾い、関連局面を初期順で少し前へ出す軽い適応補正を追加。苦手局面ドリルではこの補正を切り、固定反復を優先する。`validate-game-logic.mjs` で補正とドリル優先を検証。
  - 実戦の判断フェーズの残り時間からテンポ評価を追加。速い初回正解は小さなボーナスを得て、終了レビューにも「先手/安定/ぎりぎり」の判断テンポが残る。入門とリプレイでは加点しない。
  - 回答後 `Enter` で次へ、`R` でリプレイできるようにし、反復練習のテンポを改善。
  - テンポ・連続性・モード選択・フィードバック品質を反復。README/CLAUDE 更新済み、継続して同期する。

## 拡張（波1：土台確定後、worktree隔離の実装＋codecレビューのペア）

- [ ] 🟡 **#7 設問のパズル化**（多段・分岐・順序・先読み・制約・状態引き継ぎ）
  - 初期実装: `rollState` / `stateEffects` / `requiresState` / `forbiddenState` / `stateBias` を追加。前局面でガード回復した場合、サイド攻撃の正解選択肢が「膝を潰し直す」へ変わり、サイド攻撃自体も次局面候補として出やすくなる。`scripts/validate-data.mjs` で state 付き正解数・state 定義・stateBias 参照、`validate-game-logic.mjs` で state 依存選択肢・次局面重み・履歴保存を検証。
  - 次段階: SCENARIOS を ID付きマップ化し `result.nextScenarioId` で分岐、`steps[]` で多段化。素材は #8。
- [ ] 🔵 **#8 反応連鎖シナリオバンク**（柔術の連続性）
  - 調査完了：`docs/research-grappling.md` Appendix A に約45個の `STATE|反応→技` チェーン。これを実装に落とす。
- [ ] ⬜ **#9 相手アーキタイプ**（レスラー/足関おじさん 等で状況と正解を分岐）
  - 6類型（プレッシャーパサー/レッグロッカー/ガードプレイヤー/レスラー/柔道家/絞めハンター）。`fighter.js` で体格・構え差、`techniques.js` でタイプ別シナリオ。
  - 初期土台: `game.js` に 3 類型のロールスタイルと UI トグルを追加済み。本格実装では類型ごとの専用 pressure / next 重み / シナリオ分岐 / 見た目差へ拡張する。
- [ ] 🔵 **#10 筋トレ＆解剖モード**（人体標本で使用筋を解説）
  - 調査完了：`docs/research-strength-anatomy.md`（動作→主働筋→種目をリグ領域にマップ）。誠実な留保（断定しない表現）を維持。`fighter.js` 確定後に実装。

## 修正・品質（codex独立レビュー指摘）

- [ ] 🟡 **#11 P1/P2 バグ修正**
  - **P1**: `techniques.js` のポーズID参照は `scripts/validate-data.mjs` で検証中。ガード腕十字の赤/青ポーズ逆転を修正済み。`timeLimitSec` / `pressure` / gi-nogi表示別正解数も検証対象化。
	  - 各 pose に `poseSpecs.js` の身体制約仕様があること、参照 pose に仕様漏れがないこと、各 scenario ペアが上/下または攻/守の役割を持つことも `scripts/validate-data.mjs` で検証対象化。
	  - 全BJJのポジション family と実装済み role は `positionCatalog.js` で管理し、未実装 family を scenario から参照しないこと、赤/青 role ペアが明示許可済みであることも `scripts/validate-data.mjs` で検証対象化。
	  - 首/肩/肘/股関節/膝/足首の関節角レンジも `scripts/validate-data.mjs` で検証対象化。`invalidJointLimits` が0でない pose は追加しない。
	  - 膝は `shinL` / `shinR` の `x` を `0..150` 度に制限し、逆関節に見える負方向屈曲を検証で落とす。`invalidRuntimeAnatomy` で runtime clamp / grounding solver の退行も検証する。
  - 各 pose の root 座標が role ごとの高さレンジに収まること、scenario ペアが近接していること、上位/下位や下からの極めの高さ関係が矛盾しないことも `scripts/validate-data.mjs` で検証対象化。
  - バックは同方向かつ背後、マウント/ガード上は相手の頭側、サイドは横圧、下からの腕十字/三角は角度切り、相手に荷重する pose は支持中心が外れすぎず、力ベクトルが相手方向へ向くこと、`_audit` が支持中心起点の力矢印と荷重線を維持することも `scripts/validate-data.mjs` で検証対象化。
  - `next` 参照の存在・参照先 scenario id、正解 choice の `reaction`、不正解 choice の `consequence` も `scripts/validate-data.mjs` で検証対象化。
  - `next` の文字列/重み付き形式、weight の正数性、重み付きエントリ数も `scripts/validate-data.mjs` で検証対象化。
  - `opponentActions` の件数、3件以上の初動を持つ局面数、label、cue、attack pose、weight、style/tactic 参照、専用 `pressure` / `readCues` も `scripts/validate-data.mjs` で検証対象化。
  - `stateEffects` / `requiresState` / `forbiddenState` / `stateBias` / `requiresAction` / `forbiddenAction` の形と、空状態/単一状態/action/gi-nogi ごとの正解 choice 数も `scripts/validate-data.mjs` で検証対象化。
  - `scripts/validate-game-logic.mjs` でラン生成の重複サンプル、未来スロット重複回避、文字列 next の後方互換を検証。
  - 速い初回正解のテンポボーナスと runStats 記録も `scripts/validate-game-logic.mjs` で検証。
  - 相手スタイルの preferred scenario id も `scripts/validate-data.mjs` で検証対象化。
  - **P2**: リプレイのスコア/連続正解/流れの二重反映は `resolvedIndexes` 相当の状態で抑止済み。継続ロール中の境界ケースを追加確認する。
  - **P3/P4**: `renderBadge` の raw innerHTML 安全化 ／ role 反映の徹底は実装済み。UI回帰をスクショで継続確認する。
  - **確認済み**: モードトグルは `mode-mixed` / `mode-defense` / `mode-offense` / `uniform-gi` / `uniform-nogi` / `difficulty-beginner` / `difficulty-live` の id ベースで配線済み。

## 検証手順

```sh
cd Jiu-Jitsu && python3 -m http.server 8080 --bind 127.0.0.1   # → http://localhost:8080
for f in js/*.js; do node --check "$f"; done                    # 構文
node scripts/validate-data.mjs                                  # データ整合
node scripts/validate-game-logic.mjs                            # ラン生成/next重複回避
```
3D確認はヘッドレスChrome（`--headless=new --enable-unsafe-swiftshader --virtual-time-budget=1500 --screenshot=...`）。
`_audit.html?q=triangle` のように `q` パラメータで監査対象を絞れる。初動別ポーズは `_audit.html?q=action` で確認する。既定では重心投影/支持エリア/力ベクトルと接触点ラベルが出る。純粋な見た目だけを見る場合は `markers=off` を付ける。
UI確認は `_ui_audit.html` で、setup 中の選択肢ロック、判断フェーズの番号表示、数字キー対応、今回の補正、初動別に正解が変わる表示、回答後の初動cueの見た目を確認する。
最終確認: 防御/攻撃/ギ/ノーギ/連続フロー/（実装後）パズル・解剖の各モードをスクショで目視。

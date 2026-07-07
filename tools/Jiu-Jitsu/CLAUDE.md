# CLAUDE.md - Jiu-Jitsu Defense Dojo

> **現行の実装は `v2/` と `native/` の 2 系統。** `v2/` は TypeScript strict + Vite + Vitest + three/npm の Web 参照実装。解剖モデル (`v2/src/anatomy/`) がレンダリング clamp・ポーズ検証・関節ラボ教育の 3 役を駆動し、ロールは台本なしのスクランブル式 (ポジショングラフ歩行 + Leitner SRS)。検証は `cd v2 && npm run check && npm run test`、ポーズ目視は `v2/_audit.html?one=<red>+<blue>`。詳細は [v2/README.md](v2/README.md)。
> `native/` は Rust + Bevy + Avian で物理関節へ移行する試作。検証は `cd native && cargo test -p anatomy && cargo check`。詳細は [native/README.md](native/README.md)。以下は v1 (このディレクトリ直下) の記述。
> 注意: 座標規約のうち「仰向け頭+Z = rot[-90,180,0]」は v1 の Euler 合成順バグ。THREE の XYZ order では [-90,0,180] が正 (v2 で修正済み)。

赤・青の 3D 人体で柔術を学ぶブラウザ教育ゲーム。Three.js (CDN, ビルド不要)。
一本の連続ロールとして進行し、ミックス/防御(自分=青)/攻撃(自分=赤)、ギ/ノーギ、入門/実戦を切替可能。
正解すると短い結果表示後に自動前進。各 scenario は複数の `opponentActions` を持ち、現在は25件の初動と7件の3初動以上シナリオがある。相手スタイル/戦術制約で出やすい初動が変わる。各 `opponentActions` は専用 attack pose と cue も持ち、判断フェーズの3D・読む線・pressure と回答後の「初動の読み」が初動ごとに変わる。choice は `requiresAction` / `forbiddenAction` で初動別に出し分けられ、マウント脱出では腕隔離/高いマウント/脚絡みで、サイド攻撃ではフレーム回復/背を向ける/膝盾で、クローズドガード防御では姿勢折り/腰角度/起き上がりで正解が変わる。正解 choice の `reaction` で相手の反応を表示し、不正解 choice の `consequence` で相手の追撃を表示する。choice の `stateEffects` は `rollState` として次局面へ残り、`requiresState` / `forbiddenState` で一部選択肢を出し分ける。scenario の `stateBias` は現在の `rollState` に噛み合う次局面候補と実戦時間切れ時の追撃候補を出やすくする。回答後はその初動で読むべき `readCues` を「読めた線 / 見落とした線 / 遅れた線」として返す。`reaction` / `consequence` は重み付き `next` 候補で次スロットを差し替え、直前結果と矛盾しにくい局面へ寄せる。差し替え時は過去に解いた局面と未来スロットの重複を避ける。相手スタイル（ランダム/プレッシャーパサー/絞めハンター/ガードプレイヤー）は UI で選べ、シナリオ優先度、相手初動、実戦時間切れ時の悪手選択、実戦モードの相手意図に影響する。入門は時間制限なし、実戦は判断制限時間あり。実戦の速い初回正解はテンポボーナスを得る。正解/不正解/実戦時間切れで流れメーターが変わり、実戦モードでは次局面の判断時間にも軽く影響する。ロールごとのミッション（今回の狙い）は終了ボーナス、戦術制約（今回の制約）は実戦判断時間と next 重みに影響する。通常再ロールでは直近ミスの readCue を拾って関連局面を少し出やすくする。ロール終了時に各局面の判断・相手初動・読む線・テンポ・反応/追撃・選ばれた次局面・引き継ぎ状態・次の稽古ポイントをレビューする。ミスがあれば、最初の苦手局面と同じ相手初動を先頭にした新しいロールを任意で開始できる。
入力は相手の attack ポーズ表示後に数字キー 1〜4 で選択、回答後 Enter で次へ、R でリプレイ。setup 中・入力欄フォーカス中・修飾キー付き入力は無視する。選ばれた `opponentAction` の label / 専用 readCues / 専用 pressure は setup 中に先出しせず、attack 後の判断フェーズで表示する。

## 動かし方 / 検証

`file://` 不可。静的サーバー必須: `python3 -m http.server 8080` → http://localhost:8080
構文チェック: `for f in js/*.js; do node --check "$f"; done` (three は CDN 解決なので実行確認はブラウザで)
データ整合: `node scripts/validate-data.mjs`
状態機械: `node scripts/validate-game-logic.mjs`
UI監査: `_ui_audit.html` で setup ロック、番号付き選択肢、今回の補正、初動別正解、初動cueを確認

## 設計上の制約・前提

- **教育原則が主役**: ポジション階層 / position before submission / 「タップ=学び・安全装置」を軸にする。全BJJの分類は `docs/POSITION_TAXONOMY.md` と `js/positionCatalog.js` に持つが、ゲームに出すのは検証済みの実装済み role ペアだけ。
- **研究の裏付けは誠実に**: 「position before submission」は上半身サブミッション限定で統計的に成立 (Spanias 2022, r=0.50)。下半身関節技 (ヒールフック) には当てはまらない点を歪めない。出典は README 参考文献に集約。
- **人体はスタイライズ優先**: 解剖学的厳密さより「どのポジションか読めるか」。`positionCatalog.js` は全体分類/実装済み role/許可 role ペア、`poses.js` は関節オイラー角データ、`poseSpecs.js` は役割・支持基底・接触点・力の方向。座標規約: 立位 root.y=0.92・顔はローカル +Z・+Y上。開始姿勢の赤青は互いの root 方向を向く。仰向け(腹が上)=root.rot[-90,0,0] で頭-Z、うつ伏せ=[+90,0,0] で頭+Z。寝技ペアは「下=仰向け / 上=膝立ち」で胴上(z方向)に重ねて噛み合わせる。膝は簡易リグ上ではヒンジ関節として扱い、`shinL` / `shinR` の `x` は `0..150` 度だけを許す。脚の開きや三角の角度は `thigh*` と root 回転で作る。関節制約は `anatomy.js` を正とし、`fighter.js` はレンダリング時にも clamp する。床付近の pose は floor probe grounding で root 高さを小さく補正する。人体モデル方針は `docs/HUMAN_BODY_MODEL.md` を正とする。新規 pose は必ず `positionCatalog.js`、`poses.js`、`poseSpecs.js` をセットで追加し、`validate-data` の実装済み family、明示許可 role ペア、root 座標高さレンジ、ペア水平距離、高さ関係、向き関係、支持中心、相手へ荷重する力ベクトル、関節角レンジのチェックを通す。
- **ポーズ検証**: 寝技は数値だけでは噛み合いが読めない。`_audit.html` と強制シナリオURLをスクショし、1 ペアずつ目視確認する。`_audit.html?q=triangle` のように対象ラベルを絞れる。既定で重心投影/支持エリア/支持中心起点の力ベクトル/相手への荷重線/接触点ラベルを表示し、純粋な見た目だけを確認する場合は `markers=off` を付ける。方向・上下関係だけでなく、胸/腰/膝/足の支持点と相手への圧が見えるかを確認する。Three.js の無限 rAF があるため、Playwright CLI では `--wait-for-timeout` を短く指定して撮る。
- 全データは作者管理の静的定数。ユーザー入力経路なし (innerHTML の XSS 経路は存在しない)。

## ファイル責務

fighter(リグ+補間+gi/nogi描画) / positionCatalog(全BJJ分類+実装済みrole+許可ペア) / poses(角度データ) / poseSpecs(身体制約) / scene(3D環境+モード一括切替) / techniques(教育内容: SCENARIOS=防御, OFFENSE_SCENARIOS=攻撃, opponentActions=相手初動, requiresAction/forbiddenAction=初動別choice, stateBias=状態に合う局面重み) / game(状態機械: 連続ロール・相手初動選択・未許可ポーズペア拒否・初動別選択肢・苦手局面再ロール・自動前進・入門=時間制限なし・実戦=判断制限時間/テンポ評価・流れ・流れ/状態による next 補正・実戦時間切れの相手スタイル/状態追従・ロールミッション・戦術制約・採点・回答履歴) / ui(DOM) / main(起動)。
局面の追加は該当配列に 1 エントリ + 必要なら `poses.js` と `poseSpecs.js` にポーズ。`timeLimitSec`、`pressure.{early,urgent}`、`readCues`（首/肘/腰など 2〜4 個）、`opponentActions` 2件以上、`stateBias` を必ず検討する。主要局面は3件以上の初動を目標にし、`validate-data` の `actionVariationScenarios` と `stateBiasScenarios` を落とさない。各 `opponentActions` は `cue`、`attack.{red,blue,badge}`、専用 `pressure`、専用 `readCues` を持たせる。正解 choice は `reaction` と `next` 候補 1 件以上、不正解 choice は `consequence` と `next` 候補を持たせる。必要なら `stateEffects.add/remove`、`requiresState`、`forbiddenState`、`requiresAction`、`forbiddenAction` を使うが、空状態/単一状態/action/gi-nogiの各組み合わせで正解 choice が1つだけ残るようにする。`stateBias` は既存の state flag だけを、action 条件は同じ scenario の `opponentActions[].id` だけを参照する。`next` は文字列または `{ id, weight }`。自然な展開ほど weight を高くする。giOnly/nogiOnly で選択肢をモード別に出し分け可。相手スタイルを増やす場合は `game.js` の `OPPONENT_STYLES` に preferred scenario id を明示し、単なる表示ラベルにしない。現在は防御4 + 攻撃5 = 9シナリオ。

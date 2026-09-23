# v3.0.0 の技術選定

2026年9月7日。対象は、人形でルートを編集・確認し、青と赤の攻防を観察できるWebアプリ。骨が伸びないこと、床と身体の整合、編集内容の保護、途中の動作の検査を優先します。

## modeling-playground から取り入れた考え方

[mizchi/modeling-playground](https://github.com/mizchi/modeling-playground) のコミット `e4c69b8e11c1bea3c3f7a4e5b1cc04839f5fe2eb` を確認しました。ソースやモデル素材の転載はなく、設計の参考として使っています。

| 確認したもの | v3への反映 |
|---|---|
| [asset-architecture.md](https://github.com/mizchi/modeling-playground/blob/e4c69b8e11c1bea3c3f7a4e5b1cc04839f5fe2eb/docs/asset-architecture.md) の形状・実行時処理・表示の分離 | 姿勢データ、固定長IKとメッシュ、ルート再生、DOM入力を既存の各ファイルで分離 |
| [solvers.mjs](https://github.com/mizchi/modeling-playground/blob/e4c69b8e11c1bea3c3f7a4e5b1cc04839f5fe2eb/runtime/solvers.mjs) の二節IK | 骨長を固定し、到達範囲と曲げ方向を検査。既存の解析的IKを継続 |
| [animation-player.mjs](https://github.com/mizchi/modeling-playground/blob/e4c69b8e11c1bea3c3f7a4e5b1cc04839f5fe2eb/runtime/animation-player.mjs) の再生状態 | DOM非依存の `RoutePlayback`。余った時間を次の区間へ持ち越し、途中からの再開と区間を飛び越す更新を検証 |
| [modeling-retake-guide.md](https://github.com/mizchi/modeling-playground/blob/e4c69b8e11c1bea3c3f7a4e5b1cc04839f5fe2eb/docs/modeling-retake-guide.md) の多方向での確認 | 斜め・横・真上と骨格表示で接地と重なりを確認。陰影で形状の不整合を隠さない |

毎回の姿勢更新で人形全体のgeometry/materialを破棄・作成していた処理を修正し、同じメッシュの位置・向き・スケールを更新します。接点表示も含めた無割り当て描画ではなく、性能の数値比較は未実施です。

## 採用と見送り

| 項目 | 判断と理由 |
|---|---|
| Three.js + TypeScript strict | 継続。直接操作、カメラ、模式的な骨格を扱え、更新後の形状をテストできる |
| Vite + Vitest | 継続。開発・ビルド・検証に必要な機能が揃っている。依存は既存のlockに基づく |
| ページ遷移 | 標準のhashとHistory API。作成・確認を独立した状態とURLにし、ルーター依存を追加しない |
| 形状 | 手続き的メッシュを継続。身体と骨格を同じ関節データから更新。GLB移行は形状制作・リグ対応が別途必要で、今回の不整合の直接の解決にはならない |
| React / React Three Fiber | 今回は追加しない。画面と描画は既存クラスで分けられ、UI全体の置き換えは不要 |
| Rapier等の物理エンジン | 今回は追加しない。接触・拘束・摩擦を設計した全身モデルなしでは、柔術の組み方や解剖学的な妥当性を得られない |
| Rust / Bevy / Avian | `native/` の試作を残す。Web版v3の起動・編集に別の実行環境は要求しない |

[Rapierの公式説明](https://rapier.rs/docs/user_guides/javascript/rigid_bodies/)でも、kinematic bodyは接触力によって自動的に動きを修正せず、床や壁を通過し得ます。「エンジンを入れるだけで接地が直る」とは扱えない根拠です。上表の判断は、このアプリの現状を踏まえたものです。

## データと動作の範囲

- v3は別ディレクトリ・別保存キー。v2のデータを変更せず、自作ルートの保存形式を維持する。読み込み時に有限値・版・件数・秒数・座標を検証する。
- 作成は登録済み姿勢を保存する。確認はコピーや登録例を再生し、作成中データへ書き戻さない。
- 作成中の自動保存と、最大20件の保存一覧は別のキーにする。一覧は明示的に残した時点の姿勢を保持し、編集用に開いた後の操作で書き換わらない。移動・バックアップには既存の単体ルートJSONと標準のFile / Blob APIを使う。
- 再生は固定長の骨を保ち、到達距離・屈曲・床・内部侵入・曲げ方向の急変を検査する。24点の事前検査を連続衝突検出の証明にはしない。
- 連続化できていない返しやパスは、説明を伴う局面切り替えとする。対戦も状態遷移で、物理演算による自由な組み合いとは区別する。

この判断を見直す条件は、連続した組み合いそのものが必要になり、接触箇所・関節拘束・摩擦・姿勢制御を検証できるモデルと試験例が揃うことです。v3では検証可能な模式図と戦術の進行を提供します。

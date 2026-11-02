# Rust Grokking AI Algorithms

『Grokking AI Algorithms, Second Edition』の全12章を対象に、本文と図に登場するアルゴリズムをRustで独立に実装した学習用crateです。探索からニューラルネットワーク、Transformer、拡散モデルの学習・生成まで動かせます。依存crate、外部API、学習済み重みのダウンロードはありません。

参照範囲はローカルの `grokking-ai-algorithms/content/en/01.md` から `12.md` と対応する図です。図8.30、本文中で名前が紹介されるランダムフォレスト、ベイズ最適化なども対象です。書籍本文・図版・コードは収録していません。

## 実行

Rust Edition 2024、最低対応Rust 1.85。crateのルートで実行します。

```sh
cargo run --release --example all_chapters
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
cargo +1.85.0 test --all --all-targets
```

入力・seed・学習回数は [`examples/all_chapters.rs`](examples/all_chapters.rs) にあります。以下はローカル実測で、損失は人工データに対する値です。

| 対象 | 結果 |
| --- | --- |
| ミニマックス / αβ枝刈り | 評価値5、調べた葉は8個中5個 |
| ナップサック / ACO / PSO | 価値11、巡回長4、Booth関数の解 `(1, 3)` |
| 決定木 / t-SNE | XORを全件分類、KL損失 `0.693070 → 0.000003` |
| MLP / Q-learning | XORのMSE `0.252432 → 0.000655`、開始状態の最良行動価値0.9 |
| Transformer / U-Net | CE `1.844870 → 0.004897`、ノイズMSE `0.143263 → 0.020503` |

Transformerは `[0, 1, 0, 1, 0, 1]` を生成します。U-Netは4×4の人工画像1件でノイズを学習し、逆拡散で16画素を返します。自然言語能力や画像品質を示す評価ではありません。

## 全章の実装対応表

名称が広い手法は、採用した具体的な方式を記載しています。

### 第1〜3章: 探索

実装: [`search.rs`](src/search.rs)。検証: 同ファイルの単体テスト、[`search_regressions.rs`](tests/search_regressions.rs)、[`classical.rs`](tests/classical.rs)。

| 項目 | 実装と確認内容 |
| --- | --- |
| 数当て / 二分探索 | ソート済み配列の探索、空配列 |
| BFS / DFS | 有向グラフの経路復元、閉路、到達不能、DFSの枝をたどる順序 |
| A* | 非負重み、許容的ヒューリスティックを前提に最短経路。`u32::MAX` の経路も表現し、範囲を超える経路を除外 |
| ミニマックス / αβ枝刈り | 完全二分木の3段の既知解、ランダムな木100件で値の一致、訪問葉数の削減 |

隣接リストを採用しています。隣接行列・接続行列はグラフの表現方法として扱い、別の探索アルゴリズムには数えていません。

### 第4〜7章: 進化と群知能

実装: [`evolution.rs`](src/evolution.rs)、[`optimization.rs`](src/optimization.rs)、[`swarm.rs`](src/swarm.rs)。検証: [`classical.rs`](tests/classical.rs)、[`workflows.rs`](tests/workflows.rs)。

| 項目 | 実装と確認内容 |
| --- | --- |
| 二値GA、選択、世代交代 | ルーレット・順位・トーナメント・最良個体の選択。世代全体の交代、定常状態での一部交代、エリート保存。OneMaxとナップサックの探索 |
| 二値・実数の交叉と変異 | 1点・2点・一様・算術交叉、1ビットの反転、全ビットの反転、境界・算術変異。`crossover` の交換区間が末尾に達すれば1点交叉 |
| 順列・木の表現 | PMX・OX・CX、要素の交換、部分木交叉、演算子の変更。都市が1回ずつ現れる性質を確認 |
| 遺伝的プログラミング / 進化的プログラミング / ニューロエボリューション | 式木の `x²+x`、固定関数の実数パラメータ、MLPの重みをそれぞれ探索。式木は深さ6まで |
| ACO / PSO | 巡回路、フェロモンの蒸発・付加、確率的な次地点選択。粒子の速度・位置・自己最良・全体最良を更新し、Booth関数の最小値を確認 |

`flip_bit_mutation` は第4章の用語に合わせて全ビットを反転します。乱数は [`random.rs`](src/random.rs) のSplitMix64とBox–Muller変換です。暗号用途には使いません。

### 第8章: 前処理と機械学習

実装: [`learning.rs`](src/learning.rs)、[`supervised.rs`](src/supervised.rs)、[`boosting.rs`](src/boosting.rs)、[`clustering.rs`](src/clustering.rs)。検証: [`classical.rs`](tests/classical.rs)、[`additional_algorithms.rs`](tests/additional_algorithms.rs)、[`numeric_boundaries.rs`](tests/numeric_boundaries.rs)。

| 項目 | 実装した方式 |
| --- | --- |
| 欠損補完、カテゴリ変換、スケーリング | 平均・中央値・最頻値・KNN補完、ラベル・one-hot変換、min-max。KNNは共通の観測特徴量で距離を計算。全欠損の列はエラー |
| 線形・多項式・Lasso回帰 | 単回帰の閉形式、多項式の正規方程式、重回帰の勾配降下、L1近接更新 |
| ロジスティック回帰 / KNN / SVM / Naive Bayes | 二値分類、近傍多数決・平均、線形hinge損失、Gaussian Naive Bayes |
| 決定株 / 決定木 / ランダムフォレスト / XGBoost | CART形式の再帰分割、Gini・エントロピー・情報利得、検証データによる剪定。森林はbootstrap標本と分割ごとの特徴量抽出。XGBoostは二乗誤差の勾配・Hessian、正則化、欠損値の分岐方向の学習 |
| k-means / mean shift / DBSCAN / 階層クラスタリング / PCA / t-SNE | k-means++初期化とLloyd更新、平坦カーネル、密度到達性、単連結法、共分散の固有分解、全点対を計算するStudent-t埋め込み |

分類評価はaccuracy・precision・recall・specificity・NPVを計算し、分母がゼロの指標は `None` にします。t-SNEは解析勾配を数値微分と比較します。XGBoostは手法を指し、同名ライブラリのAPIや全機能との互換性はありません。

### 第9〜10章: ニューラルネットワークと強化学習

実装: [`autodiff.rs`](src/autodiff.rs)、[`neural.rs`](src/neural.rs)、[`reinforcement.rs`](src/reinforcement.rs)、[`vision.rs`](src/vision.rs)。検証: [`neural_models.rs`](tests/neural_models.rs)、[`vision_models.rs`](tests/vision_models.rs)、[`workflows.rs`](tests/workflows.rs)。

| 項目 | 実装と検証 |
| --- | --- |
| Perceptron / MLP / 逆伝播 / 勾配降下 | AND、XORの学習。スカラー演算を記録したテープを逆向きにたどり、全パラメータをSGDで更新 |
| 活性化、損失、正規化 | step・sigmoid・tanh・ReLU・Leaky ReLU・SiLU、MSE、安定化softmax/交差エントロピー、層正規化・群正規化。微分可能な演算の勾配を検証 |
| RNN / LSTM / CNN / GAN | BPTTによる系列記憶、畳み込みと平均・最大pooling、画像分類。GANは生成器と識別器を交互更新し、1次元分布を学習 |
| Q-learning / SARSA | 決定的MDP上のエピソード学習、ε-greedy、終端で将来価値を0にする更新 |
| モデルベース / 深層強化学習 | 既知の遷移を使う価値反復と、経験再生・ターゲットネットワークを持つDQN。学習した行動価値と到達方策を確認 |

### 第11章: Transformerと周辺手法

実装: [`language.rs`](src/language.rs)、[`training.rs`](src/training.rs)、[`tuning.rs`](src/tuning.rs)。検証: [`neural_models.rs`](tests/neural_models.rs)、[`workflows.rs`](tests/workflows.rs)、[`additional_algorithms.rs`](tests/additional_algorithms.rs)。

| 項目 | 実装した方式 |
| --- | --- |
| bigram / BPE / バッチ化 / 埋め込み / 位置符号化 | 加算平滑化bigram、byte単位のBPE、1トークンずらした教師信号、学習可能な埋め込みと正弦波位置符号化。日本語・未学習文字の往復も確認 |
| Transformer | causal multi-head attention、射影、残差、層正規化、FFN、複数ブロック、出力射影。全体の逆伝播と未来の入力を参照しない性質を検証 |
| 生成 / 学習管理 / チューニング | 温度付きサンプリング、保存と復元、early stopping、直積grid・random・GPによるBayesian search |
| RLHF / MoE | 人工的な選好対からBradley–Terry報酬モデルを学習し、REINFORCEで1トークンの方策を更新。MoEはtop-kルータと独立したMLP experts |
| RAG | 単語の一致による文書検索とプロンプトへの追加。検索部分を独立に検証 |

ベイズ最適化は有限の候補集合に固定RBFカーネルのGaussian processを当てはめ、`平均 − 2 × 標準偏差` が小さい候補を評価します。few-shot / zero-shotは入力の作り方であり、小規模Transformerが未知の自然言語タスクを解けるとはしていません。

### 第12章: 拡散モデルと画像制御

実装: [`vision.rs`](src/vision.rs)、[`diffusion.rs`](src/diffusion.rs)。検証: [`vision_models.rs`](tests/vision_models.rs)、[`workflows.rs`](tests/workflows.rs)。

| 項目 | 実装した方式 |
| --- | --- |
| 画像正規化 / 順拡散 / ノイズスケジュール | `[-1,1]` への変換、Gaussian noise、linear・cosine・sigmoidスケジュール、時刻の埋め込み |
| CNN / U-Net / 逆拡散 | 通常・転置畳み込み、down/up sampling、skip結合、時刻と4次元ラベルによる条件付け。1段のU-Netでノイズを学習し、DDPMの平均・事後分散で生成 |
| attention / inpainting / outpainting | 汎用cross-attention、mask、文脈領域への同じ時刻のノイズ付加、最終画素の保持、周囲を広げたcanvas |
| LoRA / 高解像度化 / ControlNet / IP-Adapter | 凍結した重みに低ランク差分を学習。最近傍2倍拡大後の再拡散、学習可能な転置畳み込みupscaler。ゼロ初期化した出力を持つ制御分岐、画像用cross-attentionを分けたadapter |
| Canny / FID計算部分 / 選好による調整 | Gaussian・Sobel・非極大抑制・hysteresis。特徴量の平均と全共分散によるFréchet距離。報酬モデルと確率的な1段のノイズ除去によるREINFORCE |

条件付き生成にはラベルベクトルを渡します。学習済みテキストエンコーダは同梱していません。群正規化とcross-attentionは独立した演算として用意し、最小U-Net本体は畳み込みと条件ベクトルの加算で構成しています。

ControlNetは小さな追加エンコーダからbridgeへ残差を渡す構成で、論文の大きな学習済みエンコーダを複製する構成は再現していません。IP-Adapterも画像の特徴抽出器を呼び出し側に任せています。ゼロ出力時の不変性、追加部分だけの学習、元の重みの保持をテストします。

`frechet_feature_distance` は入力された特徴空間の距離です。Inceptionの所定の特徴量を使うとFIDの計算になりますが、テストは人工特徴を使い、生成画像のFIDスコアとは呼んでいません。

## 実装上の前提と検証記録

小規模なCPU計算を対象に、`Vec<f64>` とスカラー自動微分を使います。GPU、外部データでの汎化性能、学習速度、生成画像の品質は検証範囲外です。公開されたパラメータ配列を交換する時は、モデルの形状を維持してください。`Tape` の `Var` は作成したテープ内でのみ使います。低水準の演算はこの前提を持ち、高水準の学習・推論メソッドは入力を検査します。

2026-10-13、Rust 1.99.0と1.85.0で検証。テストは57件で、既知解、数値微分、不変条件、学習損失の低下、生成と保存・復元を確認します。乱数のseed固定は再現用であり、確率的探索の成功率の評価には別途複数seedでの測定が必要です。

`similarity-rs` の候補はコードで確認し、BFS/DFSの訪問時点、異なるネットワークの構成など、別々に読めることが有益な処理は統合していません。`cargo coupling --exclude-tests --no-git` は数値関数への依存集中を挙げました。内積、行列の形状検査、連立方程式などの共通計算を使う依存であり、交換可能な実装がないためtraitは追加していません。Git履歴からの同時変更の分析は行っていません。

### チェックポイント形式

`transformer_checkpoint` は文字列を返し、保存先は呼び出し側が決めます。第1行は `RGA_TRANSFORMER_V1`、第2行は `vocabulary width heads feed_forward layers context` の6整数です。以降は重み1個につき `f64::to_bits()` の16進数を1行に保存します。復元時はバージョン、形状と重み数、有限値を検査します。乱数状態やデータ読み込み位置は含みません。

## 主要な一次資料

- 決定木の集約と埋め込み: [Random Forests](https://www.stat.berkeley.edu/users/breiman/randomforest2001.pdf)、[t-SNE](https://www.jmlr.org/papers/v9/vandermaaten08a.html)、[XGBoost](https://arxiv.org/abs/1603.02754)
- attentionと最適化: [Attention Is All You Need](https://arxiv.org/abs/1706.03762)、[Gaussian Process Optimization](https://arxiv.org/abs/0912.3995)
- 拡散: [DDPM](https://arxiv.org/abs/2006.11239)、[Improved DDPM](https://arxiv.org/abs/2102.09672)
- 追加学習と制御: [LoRA](https://arxiv.org/abs/2106.09685)、[ControlNet](https://arxiv.org/abs/2302.05543)、[IP-Adapter](https://arxiv.org/abs/2308.06721)

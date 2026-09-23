# プロダクトと機能判断の研究ノート

調査日: 2026-08-18

競合機能を採用するか、どの方法で価値を検証するかを決めるときに使う。ここで挙げる手法を一律に適用せず、研究の対象範囲と限界を確認して対象プロダクトへ合わせる。

## 現時点の知見

| 論点 | 観察された知見 | 実務への含意 | 限界 |
|---|---|---|---|
| プロダクトマネジメントの難所 | 顧客が必要とする真の価値の特定、頻繁な優先順位変更、技術的負債、サイロ化が、頻度・深刻度とも高い問題として報告された | 競合差分だけで採否を決めず、ユーザー価値、戦略、技術的負債を同じ判断に含める | 文献レビュー、10人への面接、89人への調査に基づく主観評価。業界全体の発生率ではない |
| プロダクトディスカバリー | 継続的で文脈適応的な活動として、定性・定量の知見、反復学習、職能横断の協働を組み合わせる。活動回数のような虚栄指標より、仮説検証の周期と成果指標を区別する | 調査件数を成果にせず、何を学び、どの判断が変わったかを記録する | 2025年のレビューは46件のグレー文献が中心で、手法の人気は示せても有効性の強い比較証拠ではない |
| 開発前の分析 | 2024年の体系的レビューでは、低リスクのアジャイル開発は詳細すぎない前工程と成功の関連が示された。最適な分析量は文脈依存 | 可逆で小さな変更は短いブリーフで進め、高リスク・不可逆な変更だけ調査と合意を厚くする | 因果関係の頑健性は弱く、分析不足を推奨する結果ではない |
| ユーザーフィードバック | 13社21人の調査では、定性的な明示フィードバックと行動・テレメトリの暗黙フィードバックを併用していた。一方、収集後の解釈、指標選択、資源配分が弱点だった | 「なぜ」には面接や観察、「何が起きたか」には利用データを使い、重要判断では相互検証する | 少数の産業事例による質的研究であり、製品文脈による差が大きい |
| A/Bテスト | 2024年の143研究のレビューでは、機能選択、ロールアウト、継続改善が主要用途だったが、設計・評価には人の関与と統計上の判断が残る | 十分な利用者、安定した指標、可逆な変更、因果を問う必要が揃う場合に使う | 小規模・ニッチ・複雑な業務ではデータ量や測定可能性が制約になる |
| データの少ない実験 | 2025年のレビューでは、中小企業や初期製品は利用者数不足により統計的有意性を得にくく、指標自体にも制約がある | 小標本ではA/Bテストを装わず、プロトタイプ、タスク観察、面接、段階的公開を選ぶ | レビュー対象自体がまだ限られ、解決策の多くは今後の検証課題 |
| 優先順位付け | 2026年の文献レビューと実務家調査では、MoSCoW、RICE、WSJFが広く使われる一方、満足度は一様でなかった。AI/MLを頻繁に使う回答者は7.3%だった | スコアは議論と比較の補助にし、前提、依存関係、不確実性、最終判断理由を残す | 新しい単一調査であり、手法間の因果的な優劣を示していない |
| 過剰機能 | 過剰開発はscope creep、overspecification、feature creepを含み、使いやすさ、品質、プロジェクト成果を損なう。古典的実験では、利用前は能力、利用後は使いやすさを重く見るずれが示された | 機能追加と同時に学習・操作・保守コストを評価し、簡素化や非実装を必ず比較対象にする | feature fatigueの基礎実験は2005年。対象製品への適用は利用状況で再検証する |
| 競合志向 | 2022年の306社の研究では、競合へ反応する姿勢は学習を介して企業業績へ、競争を先回りする姿勢は技術志向を介してイノベーション成果へ、異なる経路で関連した | 競合を学習源として使い、表層的な同等機能ではなく自社ユーザーに適した差へ変換する | 複数企業の観察研究であり、個別機能の成功を保証しない |

## 判断規則

1. 競合の機能名ではなく、対象ユーザー、課題、期待する行動変化を先に書く。
2. 能力の追加だけでなく、利用後の学習、操作、保守、信頼へのコストを比べる。
3. RICEなどの点数を使う場合も、入力値の根拠と不確実性を残し、点数だけで決めない。
4. 可逆性と損失上限に合わせて開発前の証拠量を変える。低リスク変更の詳細な先行計画を目的化しない。
5. 定性調査で理由を、行動データで発生状況を確かめる。片方だけで重要な結論を断定しない。
6. A/Bテストは十分なデータ量と安定した指標がある場合だけ使う。小標本ではプロトタイプや観察を優先する。
7. 実装の受け入れ条件と、価値仮説の成功・反証条件を分ける。
8. 出荷後に観測できない価値は「検証済み」と呼ばない。観測手段を追加しない場合は限界として報告する。

## 出典

- Springer & Miler, “A comprehensive overview of software product management challenges” (2022): https://doi.org/10.1007/s10664-022-10134-5
- Canhoto, Almeida & Mira da Silva, “Phases, metrics, and techniques of product discovery” (2025): https://doi.org/10.1186/s13731-024-00454-9
- Jørgensen, “A systematic literature review on characteristics of the front-end phase of agile software development projects and their connections to project success” (2024): https://doi.org/10.1016/j.jss.2024.112155
- Tkalich et al., “User feedback in continuous software engineering: revealing the state-of-practice” (2025): https://doi.org/10.1007/s10664-024-10557-2
- Quin et al., “A/B testing: A systematic literature review” (2024): https://doi.org/10.1016/j.jss.2024.112011
- Chren et al., “Data-limited Continuous Experimentation (dlCE): a literature review” (2025): https://doi.org/10.1007/978-3-031-85849-9_26
- Belčević & Pamučar, “The great divide: An empirical study of the gap between AI-driven and traditional product backlog prioritization” (2026): https://doi.org/10.1016/j.array.2026.100895
- Marzi, “On the nature, origins and outcomes of Over Featuring in the new product development process” (2022): https://doi.org/10.1016/j.jengtecman.2022.101685
- Thompson, Hamilton & Rust, “Feature Fatigue: When Product Capabilities Become Too Much of a Good Thing” (2005): https://doi.org/10.1509/jmkr.2005.42.4.431
- Schulze, Townsend & Talay, “Completing the market orientation matrix: The impact of proactive competitor orientation on innovation and firm performance” (2022): https://doi.org/10.1016/j.indmarman.2022.03.013

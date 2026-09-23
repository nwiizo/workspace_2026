# 柔術道場 3 — v3.0.0

骨格を動かして作用を考え、人形でルートを組み、青と赤の攻防を試すブラウザアプリです。技名のクイズを入口にせず、観察・編集・対戦を分けています。

## 起動と検証

```sh
cd v3
npm ci
npm run dev -- --host 127.0.0.1 --port 5174
```

http://127.0.0.1:5174/ を開きます。各ページは下表のURL末尾を付けます。再読込・ブラウザの戻る／進むにも対応します。

```sh
npm run check
npm test
npm run build
npm run preview -- --host 127.0.0.1 --port 4174
```

TypeScript strict・Three.js・Vite・Vitestを使用。`modeling-playground` の構成、IK、再生、モデルの見直し方を調べ、既存の描画基盤を継続しました。[技術選定](docs/TECH_SELECTION.md)に理由と参照箇所を記載しています。

[v3の要件と対応](docs/REQUIREMENTS.md)、[検証結果](docs/VERIFICATION.md)も参照できます。

## ページと使い方

| ページ | URL | できること |
|---|---|---|
| 動きの道場 | `#lab` | 腕のテコ、支持範囲と重心投影、股関節と膝の3実験。ドラッグ、スライダー、骨格／筋肉、前後比較 |
| ポジション練習 | `#practice` | 14課題。相手の2種類の反応を見て一手ずつ練習し、前後を比較 |
| ルート確認 | `#route-watch` | 11ルート・4つの連続例題。自動再生、停止・再開、速度、繰り返し、途中の局面へ移動 |
| ルート作成 | `#route` | 二人の手足・腰を動かして姿勢を登録し、名前・秒数・順序を編集 |
| 応用ロール | `#dojo` | 青が自分、赤が相手の12手の対戦。行動への反応、上下の交代、一本、動きの履歴と再生 |
| 稽古記録 | `#records` | 身体の発見、ポジション練習の確認・復習時期 |
| 心得 | `#notes` | 用語、ポジション、参考資料 |

練習はサイドの攻守、マウント防御、ガード内の姿勢、バック防御、マウント攻撃、ガード攻撃、マウントの維持、バック攻撃、ハーフガードの上下、タートルの攻守、オープンガードの維持の14課題です。後半5課題には英語資料へのリンク・参照箇所・確認日があります。公開説明から着眼点を採用し、具体的なボタン操作・相手の反応・3D配置はアプリ用に設計しています。

練習はヒントありと任意の確認を選べ、数字キー1〜4、次へはEnter、タップはT。観察だけの手では人形を動かしません。確認済みの記録は実技の習得や帯の判定ではありません。

連続例題は、サイドからマウント・片腕保持・腕十字へ、ガードから三角へ、青と赤のトップ交代、上での姿勢回復とトップ維持の4例です。各局面に見る点と次へ進む理由を表示します。短いルート内は補間して再生しますが、返し・パス・脚を回す動作など未実装の移行は**局面を切り替え、省略した動きを明示**します。

### 作成と確認

作成ページでは手・足・腰をドラッグできます。肘・膝の点は曲げる方向の指定です。座標入力・胴体回転も使用可能。1ルート40姿勢まで、各区間0.5〜10秒。姿勢を登録してから「このルートを確認する」で確認ページへ進みます。

作成中のルートは自動保存します。別の状況を作る前に「一覧に保存」すると、現在の内容を独立したルートとして残せます（最大20件）。一覧から確認・編集・削除でき、編集用に開いて変更しても一覧の内容は変わりません。変更後も残す場合は再び一覧に保存します。

「書き出す」でJSONファイルに保存し、「ファイルを読み込む」で一覧へ追加できます。ファイルは200KB以内。読み込みは作成中データを置き換えず、形式・版・座標が不正なファイルや保存上限超過は元データを保ったまま拒否します。名前は文字として表示します。ファイルの形式検査と動きの検査は別で、読み込んだ姿勢・区間も表示と再生時に検査します。

確認ページは編集内容を保存しません。登録例を再生しても自作の編集中データは残ります。「このルートを編集する」で取り込むときは既存データの置き換えを確認します。未登録の編集、保存失敗、読めない保存データを表示し、無言で上書きしません。

到達距離・屈曲範囲・床・自分や相手の身体内部への侵入を検査し、問題のある姿勢の登録を止めます。区間の24分割検査と再生中の検査を併用し、途中の干渉や肘・膝の急な切り替わりでも停止して理由を示します。再生は0.25〜2倍速。途中から再開でき、ページを離れると止まります。

### 応用ロール

開始位置はガードの下／上、マウントの下、サイドの上。相手は「前へ出る」「返しを狙う」の2種類。全12種類の行動から上下関係・位置・余力に合うものを表示します。上では土台、姿勢回復、返しの阻止、腕の保持、位置の前進、極め。下ではフレーム、ブリッジ、腰の移動、腕の防御、返し・ガード回復。双方とも余力を回復できます。

行動には違う効果と消費があり、相手も一手返します。準備不足で進もうとすると止められます。12手またはタップで終了。表示番号のキーでも操作できます。「動きの変遷」で過去の局面を選ぶか順番に再生し、「対戦の現在へ戻る」で続けられます。過去の表示中は行動を受け付けません。

支配・準備・余力・展開回数はゲーム用の指標で、筋力や大会の採点ではありません。対戦履歴は現在のページ内だけに保持し、再読込では消えます。稽古記録への対戦成績の保存は行いません。

## 身体モデルが扱う範囲

2026年9月8日、プロット→3面図の画像生成→構造設計→モデリング→最適化の順で人形を更新しました。[制作記録](docs/model-production/README.md)に生成画像・プロンプト・変更前後・測定値があります。「心得」から3面表示の確認画面を開けます。

人形は胸郭・骨盤と固定長の手足で作る模式図です。支持位置を先に置き、手足の目標と曲げる側から二節のIKで肘・膝を求めます。届かない目標で骨を伸ばしません。身体／骨格、透過、斜め／横／真上の視点を切り替えられます。毎回人形を作り直さず、同じメッシュを更新します。

胴体と頭は連続した断面形状、四肢は太さが変わる面を使います。固定部分の結合と左右の形状共有により、全形状は1体15,520三角形・11種類のgeometryです。足先の向きと股関節まわりの接地検査も更新しています。

- テコは固定した肘と一定の下向きの外力の例。筋力や接触圧は推定しません。
- 支持範囲は入力した重心投影と比較します。姿勢から重心を推定せず、摩擦や相手の反応は計算しません。
- 脚の実験は骨盤を固定した平面内の二節モデル。筋肉図は位置・役割の模式図で、活動量ではありません。
- 3Dの検査は体幹の楕円体と四肢の代表点による近似です。全表面の接触、手足同士の衝突、肩・股関節の回旋、組織の変形、筋力は網羅しません。
- 対戦は状態に応じた局面表示です。物理演算で自由に組み合う連続シミュレーションではありません。

指摘がないことは実技の可否や安全の判定ではありません。数値は人体の安全限界やタップ時点を表しません。実技は指導者のもとで行い、痛みや苦しさを待たずにタップしてください。

## 保存とコード

`v2/` は残し、v3の保存キーを `jiu-jitsu-dojo-v3/progress`（稽古記録）、`jiu-jitsu-dojo-v3/route`（作成中）、`jiu-jitsu-dojo-v3/route-library`（保存一覧）に分けました。v2からの自動移行はありません。作成中データの従来形式は引き続き読み込めます。

保存形式は従来と同じ `version: 1` / `name` / `frames`。各姿勢に `label` / `seconds` / `blue` / `red`、身体に `pelvis`（XYZ・m）/ `rotation`（XYZW quaternion）/ `arms` / `legs`。左右の腕・脚は `[先端のXYZ, 曲げる方向のXYZ]`。骨と指摘は再計算します。読み込み時に検証し、未対応の版や不正なデータは保持したまま拒否します。名前は `textContent` で表示します。

書き出したファイルも同じ単体ルート形式です。保存一覧は `{ "version": 1, "routes": [{ "id": "…", "route": { "version": 1, "name": "…", "frames": [] } }] }` の形で、各 `frames` は実際には1〜40件必要です。一覧は20件・JSON 200万文字以内、単体ルートはJSON 20万文字以内。更新前に一覧を読み直し、不正な一覧の追加・削除はしません。ブラウザの保存容量が先に上限になることもあります。

| 担当 | ファイル |
|---|---|
| ページとURL | `src/ui/app.ts` |
| 身体の実験 | `src/anatomy/mechanics.ts`, `src/labs/bodyLab.ts` |
| 練習の内容と進行 | `src/content/practice.ts`, `advancedPractice.ts`, `situationalPractice.ts`, `src/engine/practice.ts` |
| 二人の配置 | `src/render/practicePose.ts`, `rollPose.ts`, `duelPose.ts` |
| 固定長IK、形状、描画 | `src/render/mannequin.ts`, `bodyScene.ts` |
| ルートの検査、再生、保存 | `src/engine/route.ts`, `routePlayback.ts`, `routeStorage.ts` |
| 登録例と作成／確認UI | `src/content/routes.ts`, `src/ui/routeTab.ts` |
| 対戦と履歴 | `src/engine/duel.ts`, `src/ui/duelTab.ts` |

全課題の分岐、保存拒否、骨長、接地、身体内部への侵入、区間を飛び越える再生、メッシュの再利用、64通りの対戦を自動検証します。旧エンジンのコードとテストは比較用に残っていますが、`#dojo` が使うのは `DuelEngine` / `DuelTab` です。

人形一覧は開発時の `/_practice-audit.html`。`?lesson=half-bottom&node=shield&view=side`、`?roll=attack-triangle-guard&stage=1` などで指定できます。ここでの `roll` は登録例に使う従来のポーズ一覧です。数値検査に加えて横・上・斜めから形を確認します。

## 参考資料と調査日

2026年9月7日に[IBJJFの2026年世界選手権総括](https://ibjjf.com/news/2026-world-championship-black-belt-recap)を確認し、パス後の支配、バックの手の対応、ガードの腰角度の3例を登録しました。選手の試合動作の再現や流行の統計ではありません。[ADCCの9月6日付発表](https://adcombat.com/official-competitor-list-for-the-adcc-world-championship-2026-updated/)は確認時点で9月12〜13日の大会前です。自動巡回による継続更新は行いません。残る8ルートと4例題はアプリ用の練習設計です。

- [Stephan Kesting — A Roadmap for Brazilian Jiu-jitsu](https://www.grapplearts.com/wp-content/uploads/2018/03/Roadmap-for-BJJ-1.5-1.pdf)：位置と攻防の整理、タートルからガードへの防御方針の例。
- [VR Jiu-Jitsu — Maintaining Knee Shield Half Guard](https://www.vrjiujitsu.online/packages/vr-jiu-jitsu-fundamentals/videos/kneeshield-half-guard)：横向きと膝の配置の公開説明。
- [VR Jiu-Jitsu — Top Position on Opponent's Turtle](https://www.vrjiujitsu.online/packages/vr-jiu-jitsu-fundamentals/videos/position-turtle-top)：タートルの上の配置の公開説明。
- [Gracie University — Blue Belt Stripe 2](https://www.gracieuniversity.com/Pages/Public/course?enc=pZ3uf5EaC9beRyE3Wwgx0w%3D%3D)：Lesson 19と34の公開概要。教材動画は再現していません。
- [IBJJF — The Most Effective Submissions in Jiu Jitsu](https://ibjjf.com/news/the-most-effective-submissions-in-jiu-jitsu)：複数の位置からの腕十字など。具体的な接続順序は本アプリの設計。
- [OpenStax — Forces and Torques in Muscles and Joints](https://openstax.org/books/college-physics-2e/pages/9-6-forces-and-torques-in-muscles-and-joints)、[Stability](https://openstax.org/books/college-physics-2e/pages/9-3-stability)、[Types of Body Movements](https://openstax.org/books/anatomy-and-physiology-2e/pages/9-5-types-of-body-movements)：テコ、支え、関節運動。

Friend License (MIT-equivalent)

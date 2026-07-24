# ISUCON14 Rust Docker 環境

ISUCON14 の公式リポジトリを基に、Rust リファレンス実装と公式ベンチマーカーを Docker Compose だけで動かすローカル環境です。

- 取得元: <https://github.com/isucon/isucon14>
- 取得コミット: `53f8b627e040c30ebec600457c6c97da008b84b0`
- アプリ: Rust 1.83 / Axum
- データベース: MySQL 8
- 公開サーバー: nginx
- ベンチマーカー: Go 1.23

公式の `development/compose-rust.yml` を土台に、フロントエンドのコンテナビルドとローカル用ベンチマーカーを追加しています。公式の競技環境は 3 台の競技者 VM と専用ベンチマーカーであり、この 1 ホスト構成のスコアは本番スコアと直接比較できません。

## 初期状態

このディレクトリは、公式リポジトリの特定コミットをそのまま基準にした未チューニング環境です。Rust アプリ本体と初期 SQL には性能改善を加えていません。

| 項目 | 初期状態 |
|---|---|
| ソース | 公式 `isucon/isucon14` のコミット `53f8b627e040c30ebec600457c6c97da008b84b0` |
| アプリ | 公式 Rust/Axum リファレンス実装 |
| DB | 公式 `webapp/sql/` の初期データ。初回起動時に MySQL ボリュームへ投入 |
| フロントエンド | 公式ソースを Docker ビルド時に pnpm でビルド |
| ベンチマーカー | 公式 Go 実装。フロントエンドと同時生成した静的ファイルハッシュを使用 |
| チューニング | インデックス追加、SQL変更、キャッシュ導入などは未実施 |
| 起動前 | コンテナ、ネットワーク、MySQL ボリュームは未作成 |

公式スナップショットは、作業時に次の方法で入れ子の `.git` を含めず取り込みました。通常の利用時にこの操作をやり直す必要はありません。

```sh
source_dir=$(mktemp -d)
git clone https://github.com/isucon/isucon14.git "$source_dir/isucon14"
git -C "$source_dir/isucon14" checkout 53f8b627e040c30ebec600457c6c97da008b84b0
mkdir -p contests/isucon14
git -C "$source_dir/isucon14" archive HEAD | tar -x -C contests/isucon14
```

公式スナップショットへ追加したローカル用ファイルは次のとおりです。構成の正本は各ファイルに置き、この README では役割だけを示します。

| ファイル | 役割 |
|---|---|
| `compose.yaml` | Rust、MySQL、nginx、matcher、benchmark のサービス定義 |
| `docker/Dockerfile` | フロントエンド、nginx、公式ベンチマーカーのコンテナビルド |
| `docker/nginx.conf` | 静的ファイル配信と Rust API へのプロキシ |
| `docker/client-config/config.json` | 公開イメージ取得用のプロジェクト専用 Docker 設定 |
| `scripts/compose.sh` | Compose plugin / standalone Compose の差を吸収 |
| `scripts/up.sh` / `down.sh` | 起動、停止、DBを含む完全初期化 |
| `scripts/smoke-test.sh` | トップ画面と初期化 API の疎通確認 |
| `scripts/benchmark.sh` | 決済モックを含む公式ベンチマーカーの実行 |

## 初期構築方法

### 1. 必要なもの

- Docker Engine または Docker Desktop
- Docker Compose v2（`docker compose` または `docker-compose`）
- 初回ビルド用のインターネット接続

Rust、Go、Node.js、pnpm をホストへインストールする必要はありません。

この環境が取得するコンテナイメージはすべて公開イメージです。操作スクリプトは `docker/client-config/config.json` を使うため、ホスト側のレジストリ認証情報を読み込んだり変更したりしません。

### 2. ビルドと起動

```sh
cd contests/isucon14

# Rust アプリ、MySQL、nginx、マッチャーをビルドして起動
./scripts/up.sh
```

初回は Rust とフロントエンドをビルドし、MySQL ボリュームへ公式初期データを投入するため時間がかかります。2 回目以降は Docker のビルドキャッシュと既存の DB ボリュームが使われます。

起動後は次のサービスが作成されます。

```sh
./scripts/compose.sh ps
```

`db`、`webapp`、`nginx`、`matcher` が起動し、アプリは <http://localhost:8080/> で確認できます。

### 3. 初期状態の検証

```sh
# フロントエンドと Rust 初期化 API の疎通確認
./scripts/smoke-test.sh

# 公式ベンチマーカーによる短い動作確認
./scripts/benchmark.sh 10

# 公式と同じ 60 秒で本計測
./scripts/benchmark.sh 60
```

走行時間は引数または環境変数で指定します。省略時は公式と同じ 60 秒です。

```sh
./scripts/benchmark.sh 10
BENCHMARK_DURATION=10 ./scripts/benchmark.sh
```

ベンチマーカーは実行開始時に `POST /api/initialize` を呼ぶため、DB は初期データへ戻ります。フロントエンドの静的ファイル検証だけを省略したい場合は、公式オプションを次のように有効化できます。

```sh
SKIP_STATIC_SANITY_CHECK=1 ./scripts/benchmark.sh
```

### 4. 完全な初期状態へ戻す

アプリケーションデータだけを戻す場合は `./scripts/smoke-test.sh` またはベンチマーカーが呼び出す `POST /api/initialize` を使用します。MySQL ボリュームから作り直す場合は次を実行します。

```sh
RESET=1 ./scripts/down.sh
./scripts/up.sh
./scripts/smoke-test.sh
```

`RESET=1` はこの Compose プロジェクトの MySQL ボリュームを削除します。次回起動時に `webapp/sql/` から公式初期データが再投入されます。

## ローカルでの検証結果

2026-07-24 に Colima（Apple Silicon、4 CPU / 4 GiB）で次を確認しました。スコアはホスト性能に依存します。

| 確認 | 結果 |
|---|---|
| `./scripts/smoke-test.sh` | `GET /` が 200、`POST /api/initialize` が `{"language":"rust"}` |
| `./scripts/benchmark.sh 10` | `pass=true`（最終確認時のスコア 394） |
| `./scripts/benchmark.sh 60` | 実行完了。初期実装が高負荷で遅延し、`CODE=32`（長時間マッチングされない）で `pass=false` |

60 秒走行では MySQL のクエリが十数秒以上へ遅延し、ベンチマーカーの期限を超えました。コンテナ停止や初期化失敗ではなく、未チューニングの初期実装を 1 ホスト上でアプリ・DB・ベンチマーカーと同居させた際の性能限界です。まず 10 秒走行で環境を検証し、その後 60 秒走行のボトルネックを改善するのがローカルチューニングの開始点です。

## 構成

```text
localhost:8080
      |
    nginx ───── static files (Remix/Vite)
      |
  Rust/Axum :8080 ───── MySQL :3306
      |                    ^
      |                    |
      +── payment ── benchmark
      ^
      |
   matcher (0.5 秒間隔)
```

| Compose サービス | 役割 | ホスト公開 |
|---|---|---|
| `nginx` | 静的ファイルと `/api/*` のリバースプロキシ | `127.0.0.1:8080` |
| `webapp` | Rust/Axum リファレンス実装 | なし |
| `db` | MySQL 8 | `127.0.0.1:13306` |
| `matcher` | `/api/internal/matching` の定期実行 | なし |
| `benchmark` | 公式 Go ベンチマーカーと決済モック | 実行時のみ |

公開ポートは変更できます。

```sh
APP_PORT=18080 MYSQL_PORT=23306 ./scripts/up.sh
APP_PORT=18080 ./scripts/smoke-test.sh
```

## 確認できるエンドポイント

ブラウザの入口は <http://localhost:8080/> です。主な画面は `/client`、`/owner`、`/simulator` にあります。

Rust 実装が提供する API は次のとおりです。

| Method | Path | 用途 | 認証 |
|---|---|---|---|
| `POST` | `/api/initialize` | DB 初期化、決済 URL 設定 | なし |
| `POST` | `/api/app/users` | 利用者登録 | なし |
| `POST` | `/api/app/payment-methods` | 決済トークン登録 | 利用者 Cookie |
| `GET` / `POST` | `/api/app/rides` | 配車履歴取得、配車依頼 | 利用者 Cookie |
| `POST` | `/api/app/rides/estimated-fare` | 料金見積もり | 利用者 Cookie |
| `POST` | `/api/app/rides/:ride_id/evaluation` | 乗車評価 | 利用者 Cookie |
| `GET` | `/api/app/notification` | 利用者向け通知（SSE） | 利用者 Cookie |
| `GET` | `/api/app/nearby-chairs` | 周辺の椅子検索 | 利用者 Cookie |
| `POST` | `/api/owner/owners` | オーナー登録 | なし |
| `GET` | `/api/owner/sales` | 売上取得 | オーナー Cookie |
| `GET` | `/api/owner/chairs` | 所有する椅子一覧 | オーナー Cookie |
| `POST` | `/api/chair/chairs` | 椅子登録 | 登録トークン |
| `POST` | `/api/chair/activity` | 稼働状態更新 | 椅子 Cookie |
| `POST` | `/api/chair/coordinate` | 座標更新 | 椅子 Cookie |
| `GET` | `/api/chair/notification` | 椅子向け通知（SSE） | 椅子 Cookie |
| `POST` | `/api/chair/rides/:ride_id/status` | 乗車状態更新 | 椅子 Cookie |
| `GET` | `/api/internal/matching` | 配車マッチング | ローカル環境内（nginx 制限） |

初期化 API は次のように直接確認できます。

```sh
curl -sS \
  -X POST \
  -H 'Content-Type: application/json' \
  -d '{"payment_server":"http://example.invalid"}' \
  http://localhost:8080/api/initialize

# {"language":"rust"}
```

## Playwright CLI での画面確認

`@playwright/cli` を使い、次の画面が描画できることを確認しました。

| 画面 | URL | 確認結果 |
|---|---|---|
| トップ | `/` | 3 種類の画面へのリンクを表示。コンソールエラーなし |
| 利用者 | `/client` | 地図、配車操作、下部ナビゲーションを表示 |
| オーナー | `/owner` | 未ログインのため `/owner/register` へ正常に遷移 |
| 椅子シミュレーター | `/simulator` | 登録フォームとシミュレーターを表示 |

- [トップ画面](artifacts/playwright/top.png)
- [利用者画面](artifacts/playwright/client.png)
- [オーナー登録画面](artifacts/playwright/owner.png)
- [椅子シミュレーター画面](artifacts/playwright/simulator.png)

オーナー画面と椅子シミュレーターでは、未認証の新規ブラウザーセッションから認証必須 API を呼ぶため 401 が記録されます。登録・ログイン前の想定どおりの応答です。

再確認するときは、サービスを起動した状態で次を実行します。

```sh
npx --yes @playwright/cli@latest open http://localhost:8080
npx --yes @playwright/cli@latest screenshot --filename=artifacts/playwright/top.png
npx --yes @playwright/cli@latest close
```

## 日常操作

```sh
# 状態
./scripts/compose.sh ps

# ログ
./scripts/compose.sh logs -f webapp
./scripts/compose.sh logs -f db

# Rust コード変更後に再ビルド
./scripts/compose.sh up -d --build webapp nginx matcher

# 停止（DB データは保持）
./scripts/down.sh

# 停止して DB ボリュームも削除
RESET=1 ./scripts/down.sh
```

Rust 実装は `webapp/rust/`、初期 SQL は `webapp/sql/`、ローカル Compose 拡張は `compose.yaml` と `docker/` にあります。

## トラブルシューティング

### 8080 または 13306 が使用中

`APP_PORT` または `MYSQL_PORT` を変更してください。ベンチマーカーは Docker ネットワーク内の `http://nginx` を使うため、ホスト側の `APP_PORT` を変更しても動作します。

### DB を完全に作り直す

[完全な初期状態へ戻す](#4-完全な初期状態へ戻す)の手順で、この Compose プロジェクトの MySQL ボリュームを再作成してください。

### ベンチマーカーの静的ファイル検証に失敗する

まずキャッシュを使わずフロントエンドとベンチマーカーを再ビルドします。

```sh
./scripts/compose.sh --profile benchmark build --no-cache nginx benchmark
./scripts/benchmark.sh
```

調査中だけ検証を省略する場合は `SKIP_STATIC_SANITY_CHECK=1` を使用してください。静的ファイルへの負荷も省略されるため、通常のスコア計測では使用しません。

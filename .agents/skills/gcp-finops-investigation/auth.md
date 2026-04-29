# GCP Auth Learnings

非対話環境での Application Default Credentials (ADC)、サービスアカウント impersonation、Workload Identity Federation の罠。

## ADC の非対話突破

- `gcloud auth application-default login` はブラウザ対話が必要 → 非対話環境では失敗
- 非対話環境での選択肢:
  1. サービスアカウント鍵ファイル（`GOOGLE_APPLICATION_CREDENTIALS` 環境変数）
  2. ユーザー側で対話シェルで鍵を取得し、Claude が読む
  3. gcloud auth print-access-token で短命トークンを一時的に取得

## Impersonation パターン

- 組織ポリシーで SA 鍵のダウンロードが禁じられている場合、`gcloud auth application-default login --impersonate-service-account=SA_EMAIL` で impersonation
- 必要権限: `roles/iam.serviceAccountTokenCreator` を対象 SA に対して自ユーザーに付与
- `gcloud config set auth/impersonate_service_account` でも代替可
- トークンの有効期限はデフォルト1時間

## 組織・フォルダ・プロジェクト階層

- 認証は1つのプロジェクト/ユーザーで確立できても、調査対象が別プロジェクト/組織の場合は別途権限が必要
- Billing Account は組織とは独立の管理単位。Billing Account Viewer 権限は別レイヤー
- `gcloud projects list` で見えない ≠ 権限ゼロ。Folder 階層で区切られていることがある
- クロスプロジェクト調査では `--project` を都度明示、または Config configurations で切り替え

## よくあるエラーパターン

- `PERMISSION_DENIED: The caller does not have permission` — IAM ポリシー未付与、または組織ポリシーで拒否
- `RESOURCE_EXHAUSTED` — プロジェクトの API クォータ超過、大量 describe 時に注意
- `FAILED_PRECONDITION: Billing must be enabled` — 対象プロジェクトに Billing Account が紐づいていない
- トークン切れの症状は `UNAUTHENTICATED` と `invalid_grant` の両方あり得る

## Workload Identity Federation（WIF）

- GitHub Actions / CI から GCP への認証は WIF で SA 鍵レス化が 2026 のベストプラクティス
- Provider + Pool 設定を先に作り、`roles/iam.workloadIdentityUser` で CI の OIDC トークンを特定 SA にマップ
- WIF 経由でも `print-access-token` でトークン取得できれば Claude の bash で利用可能

## コピペ時の罠

- サービスアカウント鍵 JSON はそのまま ADC としては使えない。`GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json` で環境変数にパスを渡す
- 鍵 JSON の改行コード（CRLF/LF）がエディタで壊れると無効になる
- 鍵ファイルのパーミッションは `chmod 600`、gcloud が警告を出すことがある

## 最小権限で調査する場合

- Viewer ロール相当の組合せ:
  - `roles/viewer`（全リソース read）— 粒度が粗い
  - `roles/billing.viewer`（Billing Account）
  - `roles/recommender.viewer`（Active Assist）
  - `roles/bigquery.dataViewer` + `roles/bigquery.jobUser`（Billing Export 分析）
- より粒度細かくするなら Custom Role を作成

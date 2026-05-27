# Auth Learnings: MFA + AssumeRole in Non-Interactive Env

Claude の bash 実行環境は TTY ではないので、CLI の MFA プロンプトは発火せず即失敗する。公式ドキュメントにはない突破パターンが必要。

## 非対話環境で詰むパターン

- `aws configure export-credentials` はプロファイル解決で MFA プロンプト発火 → NG
- `aws sts get-session-token` → 一時 credentials → `source_profile` 経由 AssumeRole の3段構えは、MFA クレームが AssumeRole に伝播せず `AccessDenied` になることがある
- プロファイル側で `mfa_serial` を定義し直接 AssumeRole の方が通りやすい
- fish と bash の構文混在（`set -x VAR value` は bash では xtrace 有効化）で意図しない動作になる

## 成功パターン

1. ユーザーが対話シェルで `aws sts assume-role` に `--token-code` を引数で渡し、結果 JSON をファイルに保存
2. Claude 側が `jq` で `AccessKeyId` / `SecretAccessKey` / `SessionToken` を抽出
3. 環境変数 export 文を sourceable なファイルに書き出し、以降の bash 呼び出しで `source` する

## 事前クリーンアップの罠

- 前セッションの `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN` が env に残ると、新規 MFA コードを使っても `InvalidClientTokenId` で失敗する
- assume-role の前に必ず `unset` する

## 認証情報コピペの罠

- シェル出力の末尾 `%` は zsh/fish の「改行なし」マーカー。Slack や Notion 経由でコピペすると混入して無効なキーになる
- TOTP は**同一コード2回使用不可**。連続リトライ時は次の30秒コードを待つ
- TOTP 生成直後に実行。残り5秒以下なら次のコード待ち（通信+API処理でコード切替を跨ぐ）

## ARN 記述の罠

- ロール ARN に IAM パスが含まれる組織では、短縮形 `role/xxx` ではなく完全形 `role/<path>/xxx` が必要
- AWS は存在しないロールでも `AccessDenied` を返すため、パス誤りを権限不足と誤認しやすい
- 誤記を疑うべきタイミング: 信頼ポリシーと IAM ユーザー側ポリシーが両方揃っているのに `AccessDenied` が続くとき

## セッション有効期限

- 一時 credentials はデフォルト1時間。長時間調査では再取得計画を事前に持つ
- `--duration-seconds` は信頼ポリシー側の `MaxSessionDuration` を超えられない

#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <pr-number> [commit-sha]" >&2
  exit 1
fi

PR_NUMBER=$1
COMMIT_SHA=${2:-manual}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DEMO_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
WORKSPACE_ROOT=$(cd "$DEMO_DIR/.." && pwd)
DEFAULT_MIRROR_REPO_DIR="$(cd "$WORKSPACE_ROOT/../.." && pwd)/argocd-pr-preview-demo-mirror"
MIRROR_REPO_DIR=${MIRROR_REPO_DIR:-$DEFAULT_MIRROR_REPO_DIR}
PREVIEW_ID="pr-${PR_NUMBER}"
PREVIEW_FILE="$MIRROR_REPO_DIR/preview-metadata/$PREVIEW_ID.yaml"
LOCK_DIR="${TMPDIR:-/tmp}/argocd-pr-preview-demo-mirror.lock.d"

mkdir -p "$MIRROR_REPO_DIR"

if [[ -z "${ARGOCD_PREVIEW_MIRROR_LOCKED:-}" ]]; then
  while ! mkdir "$LOCK_DIR" 2>/dev/null; do
    sleep 0.1
  done
  trap 'rmdir "$LOCK_DIR" >/dev/null 2>&1 || true' EXIT
  export ARGOCD_PREVIEW_MIRROR_LOCKED=1
fi

"$SCRIPT_DIR/create-mirror-repo.sh" >/dev/null

cat <<EOF >"$PREVIEW_FILE"
prNumber: $PR_NUMBER
commitSha: $COMMIT_SHA
EOF

git -C "$MIRROR_REPO_DIR" add "$PREVIEW_FILE"

if ! git -C "$MIRROR_REPO_DIR" diff --cached --quiet; then
  git -C "$MIRROR_REPO_DIR" -c user.name=Codex -c user.email=codex@example.invalid commit -m "feat(preview): upsert $PREVIEW_ID" >/dev/null
fi

"$SCRIPT_DIR/publish-mirror-repo.sh" >/dev/null

if kubectl get applicationset -n argocd pr-previews >/dev/null 2>&1; then
  kubectl annotate applicationset -n argocd pr-previews argocd.argoproj.io/application-set-refresh=true --overwrite >/dev/null 2>&1 || true
fi

printf 'preview host: http://%s.preview.localtest.me:8080\n' "$PREVIEW_ID"

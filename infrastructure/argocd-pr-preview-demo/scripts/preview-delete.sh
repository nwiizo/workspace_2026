#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <pr-number>" >&2
  exit 1
fi

PR_NUMBER=$1

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

if [[ -f "$PREVIEW_FILE" ]]; then
  rm -f "$PREVIEW_FILE"
  git -C "$MIRROR_REPO_DIR" add -A "$PREVIEW_FILE"
  if ! git -C "$MIRROR_REPO_DIR" diff --cached --quiet; then
    git -C "$MIRROR_REPO_DIR" -c user.name=Codex -c user.email=codex@example.invalid commit -m "chore(preview): delete $PREVIEW_ID" >/dev/null
  fi
  "$SCRIPT_DIR/publish-mirror-repo.sh" >/dev/null
  if kubectl get applicationset -n argocd pr-previews >/dev/null 2>&1; then
    kubectl annotate applicationset -n argocd pr-previews argocd.argoproj.io/application-set-refresh=true --overwrite >/dev/null 2>&1 || true
  fi
fi

printf 'deleted preview %s\n' "$PREVIEW_ID"

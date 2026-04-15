#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DEMO_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
WORKSPACE_ROOT=$(cd "$DEMO_DIR/.." && pwd)
DEFAULT_MIRROR_REPO_DIR="$(cd "$WORKSPACE_ROOT/../.." && pwd)/argocd-pr-preview-demo-mirror"
MIRROR_REPO_DIR=${MIRROR_REPO_DIR:-$DEFAULT_MIRROR_REPO_DIR}
RUNTIME_DIR=${RUNTIME_DIR:-"$DEMO_DIR/runtime"}
BARE_REPO_DIR="$RUNTIME_DIR/git-mirror/argocd-pr-preview-demo-mirror.git"
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

git -C "$MIRROR_REPO_DIR" push origin main >/dev/null
git --git-dir="$BARE_REPO_DIR" update-server-info

printf 'published %s to %s\n' "$MIRROR_REPO_DIR" "$BARE_REPO_DIR"

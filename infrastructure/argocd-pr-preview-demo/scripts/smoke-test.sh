#!/usr/bin/env bash
set -euo pipefail

PR_NUMBER=${1:-101}
COMMIT_SHA=${2:-smoke}
PREVIEW_ID="pr-${PR_NUMBER}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

"$SCRIPT_DIR/bootstrap-kind.sh" >/dev/null
"$SCRIPT_DIR/preview-upsert.sh" "$PR_NUMBER" "$COMMIT_SHA" >/dev/null

for _ in $(seq 1 60); do
  if kubectl get application -n argocd "$PREVIEW_ID" >/dev/null 2>&1; then
    break
  fi
  sleep 5
done

kubectl wait -n argocd \
  --for=jsonpath='{.status.sync.status}'=Synced \
  "application/${PREVIEW_ID}" \
  --timeout=300s >/dev/null

kubectl rollout status deployment/frontend -n "$PREVIEW_ID" --timeout=300s >/dev/null
kubectl rollout status deployment/backend -n "$PREVIEW_ID" --timeout=300s >/dev/null

echo "frontend:"
curl --fail --silent --show-error -H "Host: ${PREVIEW_ID}.preview.localtest.me" http://127.0.0.1:8080/ | sed -n '1,20p'
echo
echo "backend:"
curl --fail --silent --show-error -H "Host: ${PREVIEW_ID}.preview.localtest.me" http://127.0.0.1:8080/api

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DEMO_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
RUNTIME_DIR=${RUNTIME_DIR:-"$DEMO_DIR/runtime"}
RENDER_DIR="$DEMO_DIR/.rendered"
CLUSTER_NAME=${CLUSTER_NAME:-argocd-preview}
ARGOCD_VERSION=${ARGOCD_VERSION:-v3.3.6}
KIND_CONFIG_TEMPLATE="$DEMO_DIR/kind/cluster.yaml.tmpl"
KIND_CONFIG_RENDERED="$RENDER_DIR/cluster.yaml"
GIT_MIRROR_HOST_PATH="$RUNTIME_DIR/git-mirror"

mkdir -p "$RUNTIME_DIR/git-mirror" "$RENDER_DIR"
"$SCRIPT_DIR/create-mirror-repo.sh" >/dev/null

python3 - <<PY
from pathlib import Path
template = Path("$KIND_CONFIG_TEMPLATE").read_text()
Path("$KIND_CONFIG_RENDERED").write_text(template.replace("__GIT_MIRROR_HOST_PATH__", "$GIT_MIRROR_HOST_PATH"))
PY

if ! kind get clusters | grep -qx "$CLUSTER_NAME"; then
  kind create cluster --name "$CLUSTER_NAME" --config "$KIND_CONFIG_RENDERED"
fi

kubectl cluster-info >/dev/null

kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml >/dev/null
kubectl rollout status deployment/ingress-nginx-controller -n ingress-nginx --timeout=300s >/dev/null

kubectl apply -f "$DEMO_DIR/manifests/git-mirror/namespace.yaml" >/dev/null
kubectl apply -f "$DEMO_DIR/manifests/git-mirror/deployment.yaml" >/dev/null
kubectl apply -f "$DEMO_DIR/manifests/git-mirror/service.yaml" >/dev/null
kubectl rollout status deployment/git-mirror -n git-mirror --timeout=180s >/dev/null

kubectl create namespace argocd >/dev/null 2>&1 || true
kubectl apply -n argocd --server-side --force-conflicts \
  -f "https://raw.githubusercontent.com/argoproj/argo-cd/${ARGOCD_VERSION}/manifests/install.yaml" >/dev/null

kubectl patch configmap argocd-cmd-params-cm -n argocd \
  --type merge \
  -p '{"data":{"reposerver.repo.cache.expiration":"15s","reposerver.default.cache.expiration":"15s"}}' >/dev/null
kubectl rollout restart deployment/argocd-repo-server -n argocd >/dev/null

kubectl rollout status deployment/argocd-server -n argocd --timeout=300s >/dev/null
kubectl rollout status deployment/argocd-repo-server -n argocd --timeout=300s >/dev/null
kubectl rollout status deployment/argocd-applicationset-controller -n argocd --timeout=300s >/dev/null
kubectl rollout status statefulset/argocd-application-controller -n argocd --timeout=300s >/dev/null

kubectl apply -f "$DEMO_DIR/manifests/argocd/project.yaml" >/dev/null
kubectl apply -f "$DEMO_DIR/manifests/argocd/applicationset-directory.yaml" >/dev/null

cat <<EOF
cluster: $CLUSTER_NAME
argocd version: $ARGOCD_VERSION
mirror repo served from: git://git-mirror.git-mirror.svc.cluster.local/argocd-pr-preview-demo-mirror.git
next:
  $DEMO_DIR/scripts/preview-upsert.sh 101
  curl -H 'Host: pr-101.preview.localtest.me' http://127.0.0.1:8080/
  curl -H 'Host: pr-101.preview.localtest.me' http://127.0.0.1:8080/api
EOF

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

mkdir -p "$MIRROR_REPO_DIR" "$RUNTIME_DIR/git-mirror"

if [[ ! -d "$MIRROR_REPO_DIR/.git" ]]; then
  git -C "$MIRROR_REPO_DIR" init -b main >/dev/null
fi

if [[ ! -f "$MIRROR_REPO_DIR/README.md" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/README.md"
# argocd-pr-preview-demo-mirror

This repository is the mirrored GitOps source for the Argo CD preview demo.
EOF
fi

mkdir -p "$MIRROR_REPO_DIR/base/app"

if [[ ! -f "$MIRROR_REPO_DIR/base/app/kustomization.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/kustomization.yaml"
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - namespace.yaml
  - backend-deployment.yaml
  - backend-service.yaml
  - frontend-deployment.yaml
  - frontend-service.yaml
  - ingress.yaml
  - frontend-configmap.yaml
commonLabels:
  app.kubernetes.io/part-of: pr-preview-demo
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/namespace.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/namespace.yaml"
apiVersion: v1
kind: Namespace
metadata:
  name: preview
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/backend-deployment.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/backend-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: backend
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: backend
  template:
    metadata:
      labels:
        app.kubernetes.io/name: backend
    spec:
      containers:
        - name: backend
          image: hashicorp/http-echo:1.0.0
          args:
            - -listen=:8080
            - -text=preview-backend
          ports:
            - containerPort: 8080
              name: http
          readinessProbe:
            httpGet:
              path: /
              port: http
          livenessProbe:
            httpGet:
              path: /
              port: http
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/backend-service.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/backend-service.yaml"
apiVersion: v1
kind: Service
metadata:
  name: backend
spec:
  selector:
    app.kubernetes.io/name: backend
  ports:
    - name: http
      port: 80
      targetPort: http
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/frontend-configmap.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/frontend-configmap.yaml"
apiVersion: v1
kind: ConfigMap
metadata:
  name: frontend-assets
data:
  default.conf: |
    server {
      listen 80;
      server_name _;
      root /usr/share/nginx/html;

      location / {
        try_files $uri /index.html;
      }
    }
  index.html: |
    <!doctype html>
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Argo CD PR Preview</title>
        <style>
          body {
            font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
            background: linear-gradient(135deg, #f7f3e8 0%, #dce9f6 100%);
            color: #16202a;
            margin: 0;
            min-height: 100vh;
            display: grid;
            place-items: center;
          }
          main {
            width: min(720px, calc(100vw - 32px));
            background: rgba(255, 255, 255, 0.88);
            border: 1px solid rgba(22, 32, 42, 0.12);
            border-radius: 20px;
            padding: 32px;
            box-shadow: 0 20px 60px rgba(22, 32, 42, 0.16);
          }
          h1 {
            margin-top: 0;
            font-size: clamp(2rem, 6vw, 3.5rem);
            line-height: 1;
          }
          p {
            line-height: 1.6;
          }
          pre {
            padding: 16px;
            border-radius: 14px;
            background: #111827;
            color: #e5f3ff;
            overflow: auto;
          }
        </style>
      </head>
      <body>
        <main>
          <h1>PR Preview</h1>
          <p id="host"></p>
          <p>This frontend calls the preview backend through the same host using <code>/api</code>.</p>
          <pre id="payload">loading...</pre>
        </main>
        <script>
          document.getElementById("host").textContent = "Host: " + window.location.host;
          fetch("/api")
            .then((response) => response.text())
            .then((text) => {
              document.getElementById("payload").textContent = text;
            })
            .catch((error) => {
              document.getElementById("payload").textContent = String(error);
            });
        </script>
      </body>
    </html>
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/frontend-deployment.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/frontend-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: frontend
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: frontend
  template:
    metadata:
      labels:
        app.kubernetes.io/name: frontend
    spec:
      containers:
        - name: nginx
          image: nginx:1.29-alpine
          ports:
            - containerPort: 80
              name: http
          volumeMounts:
            - name: assets
              mountPath: /usr/share/nginx/html/index.html
              subPath: index.html
            - name: assets
              mountPath: /etc/nginx/conf.d/default.conf
              subPath: default.conf
      volumes:
        - name: assets
          configMap:
            name: frontend-assets
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/frontend-service.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/frontend-service.yaml"
apiVersion: v1
kind: Service
metadata:
  name: frontend
spec:
  selector:
    app.kubernetes.io/name: frontend
  ports:
    - name: http
      port: 80
      targetPort: http
EOF
fi

if [[ ! -f "$MIRROR_REPO_DIR/base/app/ingress.yaml" ]]; then
  cat <<'EOF' >"$MIRROR_REPO_DIR/base/app/ingress.yaml"
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: preview
spec:
  ingressClassName: nginx
  rules:
    - host: placeholder.preview.localtest.me
      http:
        paths:
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: backend
                port:
                  name: http
          - path: /
            pathType: Prefix
            backend:
              service:
                name: frontend
                port:
                  name: http
EOF
fi

mkdir -p "$MIRROR_REPO_DIR/preview-metadata" "$MIRROR_REPO_DIR/previews"

if [[ ! -d "$BARE_REPO_DIR" ]]; then
  git init --bare "$BARE_REPO_DIR" >/dev/null
fi

git --git-dir="$BARE_REPO_DIR" symbolic-ref HEAD refs/heads/main

git -C "$MIRROR_REPO_DIR" remote remove origin >/dev/null 2>&1 || true
git -C "$MIRROR_REPO_DIR" remote add origin "$BARE_REPO_DIR"

if [[ -n "$(git -C "$MIRROR_REPO_DIR" status --short)" ]]; then
  git -C "$MIRROR_REPO_DIR" add .
  git -C "$MIRROR_REPO_DIR" -c user.name=Codex -c user.email=codex@example.invalid commit -m "chore: initialize mirror repo" >/dev/null
fi

git -C "$MIRROR_REPO_DIR" push -u origin main >/dev/null
git --git-dir="$BARE_REPO_DIR" update-server-info

printf 'mirror repo: %s\n' "$MIRROR_REPO_DIR"
printf 'bare repo: %s\n' "$BARE_REPO_DIR"

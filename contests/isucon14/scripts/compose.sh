#!/bin/sh

set -eu

project_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
diagnostic_compose_file="$project_dir/compose.diagnostics.yaml"

if docker compose version >/dev/null 2>&1; then
  compose_command=docker-plugin
elif command -v docker-compose >/dev/null 2>&1; then
  compose_command=standalone
else
  echo "Docker Compose v2 が見つかりません。Docker Desktop または Compose plugin をインストールしてください。" >&2
  exit 1
fi

if [ -z "${DOCKER_HOST:-}" ]; then
  docker_endpoint=$(docker context inspect --format '{{.Endpoints.docker.Host}}')
  export DOCKER_HOST="$docker_endpoint"
fi

# この環境が取得するイメージはすべて public。壊れたグローバル credential
# helper の影響を受けないよう、認証情報を含まないプロジェクト専用設定を使う。
if [ -z "${DOCKER_CONFIG:-}" ]; then
  DOCKER_CONFIG="$project_dir/docker/client-config"
  export DOCKER_CONFIG
fi

if [ "$compose_command" = docker-plugin ]; then
  if [ "${ISUCON_DIAGNOSTIC:-0}" = "1" ]; then
    exec docker compose \
      --project-directory "$project_dir" \
      -f "$project_dir/compose.yaml" \
      -f "$diagnostic_compose_file" \
      "$@"
  fi
  exec docker compose --project-directory "$project_dir" -f "$project_dir/compose.yaml" "$@"
fi

if [ "${ISUCON_DIAGNOSTIC:-0}" = "1" ]; then
  exec docker-compose \
    --project-directory "$project_dir" \
    -f "$project_dir/compose.yaml" \
    -f "$diagnostic_compose_file" \
    "$@"
fi
exec docker-compose --project-directory "$project_dir" -f "$project_dir/compose.yaml" "$@"

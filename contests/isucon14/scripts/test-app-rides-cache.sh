#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
container_suffix=$$
payment_container="isucon14-app-rides-cache-payment-$container_suffix"

initialize() {
  payment_server=$1
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --request POST \
    --header "Content-Type: application/json" \
    --data "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize"
}

cleanup() {
  initialize "http://benchmark:12345" >/dev/null 2>&1 || true
  docker rm --force "$payment_container" >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 1
fi

"$compose" run \
  --detach \
  --rm \
  --no-deps \
  --name "$payment_container" \
  --entrypoint sh \
  matcher \
  -c "while true; do printf 'HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n' | nc -l -p 18080; done" \
  >/dev/null

payment_url="http://$payment_container:18080"
response=$(initialize "$payment_url")
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac

fixture=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT users.id,
       users.access_token,
       chairs.id
FROM users
CROSS JOIN chairs
ORDER BY users.id, chairs.id
LIMIT 1
"
)
user_id=$(printf '%s\n' "$fixture" | cut -f 1)
user_token=$(printf '%s\n' "$fixture" | cut -f 2)
chair_id=$(printf '%s\n' "$fixture" | cut -f 3)

if [ -z "$user_id" ] || [ -z "$user_token" ] || [ -z "$chair_id" ]; then
  echo "cache fixtureに使うuser / chairを取得できませんでした" >&2
  exit 1
fi

ride_id=$(printf 'H%025d' "$$")
carrying_id=$(printf 'I%025d' "$$")
arrived_id=$(printf 'J%025d' "$$")

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride <<SQL
INSERT INTO payment_tokens (user_id, token)
VALUES ('$user_id', 'app-rides-cache-payment-token')
ON DUPLICATE KEY UPDATE token = VALUES(token);

INSERT INTO rides (
  id,
  user_id,
  chair_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude
) VALUES (
  '$ride_id',
  '$user_id',
  '$chair_id',
  0,
  0,
  2,
  3
);

INSERT INTO ride_statuses (id, ride_id, status, created_at)
VALUES
  ('$carrying_id', '$ride_id', 'CARRYING', NOW(6)),
  (
    '$arrived_id',
    '$ride_id',
    'ARRIVED',
    TIMESTAMPADD(MICROSECOND, 1, NOW(6))
  );
SQL

get_rides() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=$user_token" \
    "$base_url/api/app/rides"
}

before=$(get_rides)
before_count=$(printf '%s\n' "$before" | jq -er '.rides | length')
if printf '%s\n' "$before" | jq -e --arg ride_id "$ride_id" '
  any(.rides[]; .id == $ride_id)
' >/dev/null; then
  echo "未評価rideが履歴cacheへ含まれています" >&2
  exit 1
fi

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride \
  -e "UPDATE rides SET evaluation = 1 WHERE id = '$ride_id'"

before_cached=$(get_rides)
if [ "$before_cached" != "$before" ]; then
  echo "DB変更後の2回目GETがcache済みJSONを返しませんでした" >&2
  exit 1
fi

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride \
  -e "UPDATE rides SET evaluation = NULL WHERE id = '$ride_id'"

evaluation_response=$(
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=$user_token" \
    --request POST \
    --header "Content-Type: application/json" \
    --data '{"evaluation":5}' \
    --write-out '\n%{http_code}' \
    "$base_url/api/app/rides/$ride_id/evaluation"
)
evaluation_status=$(printf '%s\n' "$evaluation_response" | tail -n 1)
if [ "$evaluation_status" != "200" ]; then
  echo "評価APIが成功しませんでした: HTTP $evaluation_status" >&2
  exit 1
fi

after=$(get_rides)
after_count=$(printf '%s\n' "$after" | jq -er '.rides | length')
if [ "$after_count" -ne "$((before_count + 1))" ]; then
  echo "評価後の履歴件数が1件増えていません: before=$before_count after=$after_count" >&2
  exit 1
fi
printf '%s\n' "$after" | jq -e --arg ride_id "$ride_id" '
  [
    .rides[] |
    select(.id == $ride_id) |
    select(.evaluation == 5) |
    select(.fare == 1000) |
    select(.completed_at >= .requested_at)
  ] |
  length == 1
' >/dev/null

after_cached=$(get_rides)
if [ "$after_cached" != "$after" ]; then
  echo "評価後の2回目GETが同じJSONを返しませんでした" >&2
  exit 1
fi

printf 'app ride history cache: PASS before=%s after=%s ride=%s\n' \
  "$before_count" \
  "$after_count" \
  "$ride_id"

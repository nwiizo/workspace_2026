#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
app_host=${APP_HOST:-127.0.0.1}
app_port=${APP_PORT:-8080}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-20}
payment_delay_seconds=${PAYMENT_DELAY_SECONDS:-8}
container_suffix=$$
slow_container="isucon14-owner-sales-slow-payment-$container_suffix"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/isucon14-owner-sales.XXXXXX")
evaluation_pid=

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
  if [ -n "$evaluation_pid" ] && kill -0 "$evaluation_pid" 2>/dev/null; then
    kill "$evaluation_pid" 2>/dev/null || true
    wait "$evaluation_pid" 2>/dev/null || true
  fi
  initialize "http://benchmark:12345" >/dev/null 2>&1 || true
  docker rm --force "$slow_container" >/dev/null 2>&1 || true
  rm -rf "$tmp_dir"
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 1
fi

# The delay is the behavior under test: it keeps the external payment in
# progress after the short preparation transaction has released its ride lock
# and DB connection.
"$compose" run \
  --detach \
  --rm \
  --no-deps \
  --name "$slow_container" \
  --entrypoint sh \
  matcher \
  -c "response_fifo=/tmp/payment-response; mkfifo \"\$response_fifo\"; exec 3<>\"\$response_fifo\"; rm \"\$response_fifo\"; while true; do nc -v -l -p 18080 <&3 | { IFS= read -r request_line || exit; sleep '$payment_delay_seconds'; printf 'HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n' >&3; }; done" \
  >/dev/null

slow_payment_url="http://$slow_container:18080"
response=$(initialize "$slow_payment_url")
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
       owners.id,
       owners.access_token,
       chairs.id
FROM users
JOIN owners
JOIN chairs ON chairs.owner_id = owners.id
ORDER BY users.id, owners.id, chairs.id
LIMIT 1
"
)
user_id=$(printf '%s\n' "$fixture" | cut -f 1)
user_token=$(printf '%s\n' "$fixture" | cut -f 2)
owner_id=$(printf '%s\n' "$fixture" | cut -f 3)
owner_token=$(printf '%s\n' "$fixture" | cut -f 4)
chair_id=$(printf '%s\n' "$fixture" | cut -f 5)

initial_total=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "owner_session=$owner_token" \
    "$base_url/api/owner/sales" |
    jq -er '.total_sales'
)

pending_ride_id=$(printf 'P%025d' "$$")
pending_carrying_id=$(printf 'C%025d' "$$")
pending_arrived_id=$(printf 'A%025d' "$$")
known_ride_id=$(printf 'K%025d' "$$")
known_carrying_id=$(printf 'D%025d' "$$")
known_arrived_id=$(printf 'B%025d' "$$")
known_completed_id=$(printf 'E%025d' "$$")

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride <<SQL
INSERT INTO payment_tokens (user_id, token)
VALUES ('$user_id', 'owner-sales-boundary-payment-token')
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
  '$pending_ride_id',
  '$user_id',
  '$chair_id',
  0,
  0,
  1,
  1
);

INSERT INTO ride_statuses (id, ride_id, status, created_at)
VALUES
  ('$pending_carrying_id', '$pending_ride_id', 'CARRYING', NOW(6)),
  (
    '$pending_arrived_id',
    '$pending_ride_id',
    'ARRIVED',
    TIMESTAMPADD(MICROSECOND, 1, NOW(6))
  );

INSERT INTO rides (
  id,
  user_id,
  chair_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude,
  evaluation,
  created_at,
  updated_at
) VALUES (
  '$known_ride_id',
  '$user_id',
  '$chair_id',
  10,
  10,
  11,
  11,
  5,
  NOW(6),
  NOW(6)
);

INSERT INTO ride_statuses (id, ride_id, status, created_at)
VALUES
  ('$known_carrying_id', '$known_ride_id', 'CARRYING', NOW(6)),
  (
    '$known_arrived_id',
    '$known_ride_id',
    'ARRIVED',
    TIMESTAMPADD(MICROSECOND, 1, NOW(6))
  ),
  (
    '$known_completed_id',
    '$known_ride_id',
    'COMPLETED',
    TIMESTAMPADD(MICROSECOND, 2, NOW(6))
  );
SQL

# Send the complete request immediately, but do not parse the response until
# after owner sales is checked. Keeping stdin open also keeps the HTTP/1.1
# connection available without throttling the request upload.
{
  printf 'POST /api/app/rides/%s/evaluation HTTP/1.1\r\n' "$pending_ride_id"
  printf 'Host: %s:%s\r\n' "$app_host" "$app_port"
  printf 'Cookie: app_session=%s\r\n' "$user_token"
  printf 'Content-Type: application/json\r\n'
  printf 'Content-Length: 16\r\n'
  printf 'Connection: keep-alive\r\n'
  printf '\r\n'
  printf '{"evaluation":5}'
  sleep "$curl_max_time"
} |
  nc "$app_host" "$app_port" >"$tmp_dir/evaluation.out" &
evaluation_pid=$!

# Wait for the mock server's accept log so the known ride is moved after the
# evaluation has reached the delayed payment boundary.
payment_request_seen=0
attempt=0
while [ "$attempt" -lt 200 ]; do
  if docker logs "$slow_container" 2>&1 | grep -q 'connect to'; then
    payment_request_seen=1
    break
  fi
  attempt=$((attempt + 1))
  sleep 0.025
done
if [ "$payment_request_seen" -ne 1 ]; then
  echo "遅延決済mockが評価requestを受理したことを期限内に確認できませんでした" >&2
  exit 1
fi

# The preparation transaction must be complete before the payment request.
# Observe the first 500ms of the eight-second payment delay and fail if the
# pending ride row remains locked.
attempt=0
while [ "$attempt" -lt 20 ]; do
  lock_count=$(
    "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
      --batch \
      --skip-column-names \
      -uroot \
      -e "
SELECT COUNT(*)
FROM performance_schema.data_locks
WHERE OBJECT_SCHEMA = 'isuride'
  AND OBJECT_NAME = 'rides'
  AND LOCATE('$pending_ride_id', COALESCE(LOCK_DATA, '')) > 0
"
  )
  if [ "$lock_count" -ne 0 ]; then
    echo "遅延決済中もpending rideのrow lockが保持されています" >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 0.025
done

# Move the already-completed ride's timestamp while the pending evaluation is
# waiting outside a DB transaction. Updating a pre-existing, unrelated row
# avoids making fixture INSERT lock waits part of the timing condition.
"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride <<SQL
UPDATE rides
SET updated_at = NOW(6)
WHERE id = '$known_ride_id';
SQL

known_until_ms=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT TIMESTAMPDIFF(
         MICROSECOND,
         '1970-01-01 00:00:00',
         updated_at
       ) DIV 1000
FROM rides
WHERE id = '$known_ride_id'
"
)

get_total_sales() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "owner_session=$owner_token" \
    "$base_url/api/owner/sales?until=$known_until_ms" |
    jq -er '.total_sales'
}

baseline_total=$(get_total_sales)
known_ride_sale=700
expected_baseline=$((initial_total + known_ride_sale))
if [ "$baseline_total" -ne "$expected_baseline" ]; then
  echo "評価の完了transactionが基準売上の取得前にcommitしました" >&2
  echo "initial_total=$initial_total expected_baseline=$expected_baseline baseline_total=$baseline_total" >&2
  exit 1
fi

evaluation_committed=0
attempt=0
while [ "$attempt" -lt 400 ]; do
  evaluation_committed=$(
    "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
      --batch \
      --skip-column-names \
      -uisucon \
      isuride \
      -e "
SELECT evaluation IS NOT NULL
FROM rides
WHERE id = '$pending_ride_id'
"
  )
  if [ "$evaluation_committed" -eq 1 ]; then
    break
  fi
  attempt=$((attempt + 1))
  sleep 0.025
done
if [ "$evaluation_committed" -ne 1 ]; then
  echo "評価の完了transaction commitを期限内に確認できませんでした" >&2
  exit 1
fi

timestamp_order=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT pending.updated_at > known_ride.updated_at
FROM rides AS pending
JOIN rides AS known_ride ON known_ride.id = '$known_ride_id'
WHERE pending.id = '$pending_ride_id'
"
)
visibility=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT pending.updated_at,
       known_ride.updated_at,
       SUM(
         ride_statuses.status = 'COMPLETED'
         AND pending.updated_at BETWEEN
             FROM_UNIXTIME(0)
             AND FROM_UNIXTIME($known_until_ms DIV 1000)
                 + INTERVAL ($known_until_ms % 1000 + 999) MICROSECOND
       )
FROM rides AS pending
JOIN rides AS known_ride ON known_ride.id = '$known_ride_id'
JOIN ride_statuses ON ride_statuses.ride_id = pending.id
WHERE pending.id = '$pending_ride_id'
GROUP BY pending.id, known_ride.id
"
)
after_total=$(get_total_sales)

response_seen=0
attempt=0
while [ "$attempt" -lt 200 ]; do
  if grep -q 'HTTP/1.1 200 OK' "$tmp_dir/evaluation.out"; then
    response_seen=1
    break
  fi
  attempt=$((attempt + 1))
  sleep 0.025
done
if [ "$response_seen" -ne 1 ]; then
  echo "評価APIのHTTP 200 responseを確認できませんでした" >&2
  exit 1
fi

evaluation_body=$(awk 'body { print } /^\r$/ { body = 1 }' "$tmp_dir/evaluation.out")
response_completed_at=$(printf '%s\n' "$evaluation_body" | jq -er '.completed_at')
stored_completed_at=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT TIMESTAMPDIFF(
         MICROSECOND,
         '1970-01-01 00:00:00',
         updated_at
       ) DIV 1000
FROM rides
WHERE id = '$pending_ride_id'
"
)

if [ "$timestamp_order" -ne 1 ]; then
  echo "pending evaluationのupdated_atが既知完了rideより前です" >&2
  echo "initial_total=$initial_total baseline_total=$baseline_total after_total=$after_total owner_id=$owner_id" >&2
  echo "pending_updated_at known_updated_at completed_rows_in_window=$visibility" >&2
  exit 1
fi
if [ "$response_completed_at" -ne "$stored_completed_at" ]; then
  echo "responseのcompleted_atとDBのupdated_atが一致しません" >&2
  echo "response_completed_at=$response_completed_at stored_completed_at=$stored_completed_at" >&2
  exit 1
fi
if [ "$after_total" -ne "$baseline_total" ]; then
  echo "known rideのuntil境界へpending rideの売上が混入しました" >&2
  echo "initial_total=$initial_total baseline_total=$baseline_total after_total=$after_total owner_id=$owner_id" >&2
  echo "pending_updated_at known_updated_at completed_rows_in_window=$visibility" >&2
  exit 1
fi

echo "OK: pending evaluation timestamp is after the known completion"
echo "OK: delayed payment held no pending ride row lock"
echo "OK: payment request acceptance and evaluation response completed_at were verified"
echo "OK: pending_updated_at known_updated_at completed_rows_in_window=$visibility"
echo "OK: owner sales stayed at $baseline_total for the known ride's until boundary"

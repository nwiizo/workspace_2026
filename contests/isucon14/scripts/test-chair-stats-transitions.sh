#!/bin/sh

set -eu

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が必要です" >&2
  exit 1
fi

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
container_suffix=$$
ok_container="isucon14-chair-stats-payment-ok-$container_suffix"
fail_container="isucon14-chair-stats-payment-fail-$container_suffix"

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
  initialize "http://benchmark:12345" >/dev/null || true
  docker rm --force "$ok_container" "$fail_container" >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

"$compose" run \
  --detach \
  --rm \
  --no-deps \
  --name "$ok_container" \
  --entrypoint sh \
  matcher \
  -c "while true; do printf 'HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n' | nc -l -p 18080; done" \
  >/dev/null
"$compose" run \
  --detach \
  --rm \
  --no-deps \
  --name "$fail_container" \
  --entrypoint sh \
  matcher \
  -c "while true; do printf 'HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n' | nc -l -p 18080; done" \
  >/dev/null

ok_payment_url="http://$ok_container:18080"
fail_payment_url="http://$fail_container:18080"
response=$(initialize "$ok_payment_url")
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac

fixture=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT users.id,
       users.access_token,
       chairs.id,
       (
         SELECT other_users.id
         FROM users AS other_users
         WHERE other_users.id <> users.id
         ORDER BY other_users.id
         LIMIT 1
       ),
       (
         SELECT other_users.access_token
         FROM users AS other_users
         WHERE other_users.id <> users.id
         ORDER BY other_users.id
         LIMIT 1
       )
FROM users
CROSS JOIN chairs
ORDER BY users.id, chairs.id
LIMIT 1
"
)
user_id=$(printf '%s\n' "$fixture" | cut -f 1)
user_token=$(printf '%s\n' "$fixture" | cut -f 2)
chair_id=$(printf '%s\n' "$fixture" | cut -f 3)
other_user_id=$(printf '%s\n' "$fixture" | cut -f 4)
other_user_token=$(printf '%s\n' "$fixture" | cut -f 5)

read_stats() {
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT COALESCE(total_rides_count, 0),
       COALESCE(total_evaluation_sum, 0)
FROM (SELECT 1) AS seed
LEFT JOIN chair_stats ON chair_stats.chair_id = '$chair_id'
"
}

post_evaluation() {
  ride_id=$1
  evaluation=$2
  request_token=${3:-$user_token}
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=$request_token" \
    --request POST \
    --header "Content-Type: application/json" \
    --data "{\"evaluation\":$evaluation}" \
    --write-out '\n%{http_code}' \
    "$base_url/api/app/rides/$ride_id/evaluation"
}

missing_ride_id=$(printf 'M%025d' "$$")
missing_arrived_id=$(printf 'A%025d' "$$")
valid_ride_id=$(printf 'V%025d' "$$")
valid_carrying_id=$(printf 'C%025d' "$$")
valid_arrived_id=$(printf 'B%025d' "$$")
other_ride_id=$(printf 'O%025d' "$$")
other_carrying_id=$(printf 'D%025d' "$$")
other_arrived_id=$(printf 'E%025d' "$$")

"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride <<SQL
INSERT INTO payment_tokens (user_id, token)
VALUES ('$user_id', 'chair-stats-test-payment-token')
ON DUPLICATE KEY UPDATE token = VALUES(token);

INSERT INTO rides (
  id,
  user_id,
  chair_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude
) VALUES
  ('$missing_ride_id', '$user_id', '$chair_id', 0, 0, 1, 1),
  ('$valid_ride_id', '$user_id', '$chair_id', 0, 0, 1, 1);
INSERT INTO ride_statuses (id, ride_id, status, created_at)
VALUES
  ('$missing_arrived_id', '$missing_ride_id', 'ARRIVED', NOW(6)),
  ('$valid_carrying_id', '$valid_ride_id', 'CARRYING', NOW(6)),
  (
    '$valid_arrived_id',
    '$valid_ride_id',
    'ARRIVED',
    TIMESTAMPADD(MICROSECOND, 1, NOW(6))
  );
SQL

initial_stats=$(read_stats)

foreign_response=$(post_evaluation "$valid_ride_id" 4 "$other_user_token")
foreign_status=$(printf '%s\n' "$foreign_response" | tail -n 1)
if [ "$foreign_status" != "404" ]; then
  echo "foreign-user evaluation: expected HTTP 404 actual=$foreign_status" >&2
  exit 1
fi
foreign_state=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT evaluation IS NULL,
       (
         SELECT COUNT(*)
         FROM ride_statuses
         WHERE ride_id = '$valid_ride_id'
           AND status = 'COMPLETED'
       )
FROM rides
WHERE id = '$valid_ride_id'
"
)
if [ "$foreign_state" != "1	0" ]; then
  echo "foreign-user evaluation changed ride state: $foreign_state" >&2
  exit 1
fi
if [ "$(read_stats)" != "$initial_stats" ]; then
  echo "foreign-user evaluation unexpectedly changed chair_stats" >&2
  exit 1
fi

missing_response=$(post_evaluation "$missing_ride_id" 5)
missing_status=$(printf '%s\n' "$missing_response" | tail -n 1)
if [ "$missing_status" != "200" ]; then
  echo "missing-CARRYING evaluation: expected HTTP 200 actual=$missing_status" >&2
  exit 1
fi
if [ "$(read_stats)" != "$initial_stats" ]; then
  echo "missing-CARRYING evaluation unexpectedly changed chair_stats" >&2
  exit 1
fi
missing_completion=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT evaluation,
       (
         SELECT COUNT(*)
         FROM ride_statuses
         WHERE ride_id = '$missing_ride_id'
           AND status = 'COMPLETED'
       )
FROM rides
WHERE id = '$missing_ride_id'
"
)
if [ "$missing_completion" != "5	1" ]; then
  echo "missing-CARRYING evaluation did not complete as expected: $missing_completion" >&2
  exit 1
fi

missing_retry=$(post_evaluation "$missing_ride_id" 5)
missing_retry_status=$(printf '%s\n' "$missing_retry" | tail -n 1)
if [ "$missing_retry_status" != "400" ]; then
  echo "completed evaluation retry: expected HTTP 400 actual=$missing_retry_status" >&2
  exit 1
fi
if [ "$(read_stats)" != "$initial_stats" ]; then
  echo "completed evaluation retry unexpectedly changed chair_stats" >&2
  exit 1
fi

"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride \
  -e "UPDATE settings SET value = '$fail_payment_url' WHERE name = 'payment_gateway_url'"

failed_response=$(post_evaluation "$valid_ride_id" 4)
failed_status=$(printf '%s\n' "$failed_response" | tail -n 1)
if [ "$failed_status" != "502" ]; then
  echo "failed payment: expected HTTP 502 actual=$failed_status" >&2
  exit 1
fi

rollback_state=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT evaluation IS NULL,
       (
         SELECT COUNT(*)
         FROM ride_statuses
         WHERE ride_id = '$valid_ride_id'
           AND status = 'COMPLETED'
       )
FROM rides
WHERE id = '$valid_ride_id'
"
)
if [ "$rollback_state" != "1	0" ]; then
  echo "failed payment did not roll back evaluation/COMPLETED: $rollback_state" >&2
  exit 1
fi
if [ "$(read_stats)" != "$initial_stats" ]; then
  echo "failed payment did not roll back chair_stats" >&2
  exit 1
fi

"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride \
  -e "UPDATE settings SET value = '$ok_payment_url' WHERE name = 'payment_gateway_url'"

valid_response=$(post_evaluation "$valid_ride_id" 4)
valid_status=$(printf '%s\n' "$valid_response" | tail -n 1)
if [ "$valid_status" != "200" ]; then
  echo "valid evaluation: expected HTTP 200 actual=$valid_status" >&2
  exit 1
fi

initial_count=$(printf '%s\n' "$initial_stats" | cut -f 1)
initial_sum=$(printf '%s\n' "$initial_stats" | cut -f 2)
expected_stats=$(printf '%s\t%s' "$((initial_count + 1))" "$((initial_sum + 4))")
if [ "$(read_stats)" != "$expected_stats" ]; then
  echo "valid evaluation did not add count=1 and sum=4" >&2
  exit 1
fi

valid_retry=$(post_evaluation "$valid_ride_id" 4)
valid_retry_status=$(printf '%s\n' "$valid_retry" | tail -n 1)
if [ "$valid_retry_status" != "400" ]; then
  echo "valid evaluation retry: expected HTTP 400 actual=$valid_retry_status" >&2
  exit 1
fi
if [ "$(read_stats)" != "$expected_stats" ]; then
  echo "valid evaluation retry added chair_stats twice" >&2
  exit 1
fi

# Make the completed ride the deterministic latest ride and warm a stable app
# notification payload that embeds the shared chair's current statistics.
"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride <<SQL
UPDATE rides
SET created_at = TIMESTAMPADD(SECOND, 1, NOW(6))
WHERE id = '$valid_ride_id';
UPDATE ride_statuses
SET app_sent_at = NOW(6)
WHERE ride_id = '$valid_ride_id';
SQL

cached_count=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=$user_token" \
    "$base_url/api/app/notification" |
    jq -r '.data.chair.stats.total_rides_count'
)
if [ "$cached_count" != "$((initial_count + 1))" ]; then
  echo "app notification did not expose the initial shared-chair stats: $cached_count" >&2
  exit 1
fi

# A different user evaluates a later ride on the same chair. The first user's
# recipient revision does not change, so only the chair-stats dependency
# revision can prevent an indefinitely stale cache hit.
"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride <<SQL
INSERT INTO payment_tokens (user_id, token)
VALUES ('$other_user_id', 'chair-stats-test-other-payment-token')
ON DUPLICATE KEY UPDATE token = VALUES(token);
INSERT INTO rides (
  id,
  user_id,
  chair_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude
) VALUES ('$other_ride_id', '$other_user_id', '$chair_id', 0, 0, 1, 1);
INSERT INTO ride_statuses (id, ride_id, status, created_at)
VALUES
  ('$other_carrying_id', '$other_ride_id', 'CARRYING', NOW(6)),
  (
    '$other_arrived_id',
    '$other_ride_id',
    'ARRIVED',
    TIMESTAMPADD(MICROSECOND, 1, NOW(6))
  );
SQL

other_response=$(post_evaluation "$other_ride_id" 5 "$other_user_token")
other_status=$(printf '%s\n' "$other_response" | tail -n 1)
if [ "$other_status" != "200" ]; then
  echo "other-user evaluation: expected HTTP 200 actual=$other_status" >&2
  exit 1
fi

refreshed_count=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=$user_token" \
    "$base_url/api/app/notification" |
    jq -r '.data.chair.stats.total_rides_count'
)
if [ "$refreshed_count" != "$((initial_count + 2))" ]; then
  echo "cross-user chair stats cache stayed stale: $refreshed_count" >&2
  exit 1
fi

echo "OK: evaluation authorization, chair stats transitions, and cross-user cache invalidation hold"

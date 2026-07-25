#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
history_coordinates=${HISTORY_COORDINATES:-24}
reset_coordinates=${RESET_COORDINATES:-48}
history_base=1900000000
reset_base=1900001000
pickup_latitude=1900002001
pickup_longitude=-1900002001
destination_latitude=1900002002
destination_longitude=-1900002002

initialize()
{
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time 30 \
    --request POST \
    --header "Content-Type: application/json" \
    --data "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize"
}

cleanup()
{
  initialize >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

case "$history_coordinates:$reset_coordinates" in
  *[!0-9:]*|0:*|*:0)
    echo "HISTORY_COORDINATES and RESET_COORDINATES must be positive integers" >&2
    exit 2
    ;;
esac

# The variable must expand inside the container rather than in this shell.
# shellcheck disable=SC2016
if ! "$compose" exec -T webapp sh -c \
  'test "${ISUCON_COORDINATE_QUEUE_SHARDS:-0}" -gt 0'
then
  echo "coordinate write queue is disabled; set ISUCON_COORDINATE_QUEUE_SHARDS" >&2
  exit 1
fi

test_started_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
response=$(initialize)
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize returned an unexpected response: $response" >&2
    exit 1
    ;;
esac

chair_row=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT chairs.id, chairs.access_token, users.id
FROM chairs
CROSS JOIN users
WHERE NOT EXISTS (
  SELECT 1
  FROM rides
  WHERE rides.chair_id = chairs.id
    AND rides.evaluation IS NULL
)
ORDER BY chairs.id, users.id
LIMIT 1
"
)
# The query returns three whitespace-separated scalar columns.
# shellcheck disable=SC2086
set -- $chair_row
chair_id=$1
chair_token=$2
user_id=$3

post_coordinate()
{
  latitude=$1
  longitude=$2
  response=$(
    curl \
      --fail \
      --silent \
      --show-error \
      --connect-timeout "$curl_connect_timeout" \
      --max-time "$curl_max_time" \
      --request POST \
      --cookie "chair_session=$chair_token" \
      --header "Content-Type: application/json" \
      --data "{\"latitude\":$latitude,\"longitude\":$longitude}" \
      "$base_url/api/chair/coordinate"
  )
  case "$response" in
    *'"recorded_at":'*) ;;
    *)
      echo "coordinate response was unexpected: $response" >&2
      exit 1
      ;;
  esac
}

post_coordinates()
{
  first_latitude=$1
  coordinate_count=$2
  coordinate_index=1
  while [ "$coordinate_index" -le "$coordinate_count" ]
  do
    post_coordinate \
      "$((first_latitude + coordinate_index))" \
      "$((-first_latitude - coordinate_index))"
    coordinate_index=$((coordinate_index + 1))
  done
}

coordinate_count()
{
  first_latitude=$1
  coordinate_count=$2
  last_latitude=$((first_latitude + coordinate_count))
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT COUNT(*)
FROM chair_locations
WHERE chair_id = '$chair_id'
  AND latitude > $first_latitude
  AND latitude <= $last_latitude
"
}

assert_no_queue_errors()
{
  queue_errors=$(
    "$compose" logs --since "$test_started_at" webapp 2>&1 |
      grep -E \
        'coordinate write queue is full|coordinate write queue worker stopped|queued coordinate persistence failed' ||
      true
  )
  if [ -n "$queue_errors" ]; then
    echo "coordinate queue emitted an error during the test:" >&2
    printf '%s\n' "$queue_errors" >&2
    exit 1
  fi
}

post_coordinates "$history_base" "$history_coordinates"

deadline=$(( $(date '+%s') + 3 ))
persisted=0
while [ "$(date '+%s')" -le "$deadline" ]
do
  persisted=$(coordinate_count "$history_base" "$history_coordinates")
  if [ "$persisted" = "$history_coordinates" ]; then
    break
  fi
  sleep 0.05
done
if [ "$persisted" != "$history_coordinates" ]; then
  echo "queued history was not visible within three seconds: expected=$history_coordinates actual=$persisted" >&2
  exit 1
fi

last_latitude=$((history_base + history_coordinates))
last_longitude=$((-history_base - history_coordinates))
projection=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT latitude, longitude
FROM chair_current_locations
WHERE chair_id = '$chair_id'
"
)
if [ "$projection" != "$last_latitude	$last_longitude" ]; then
  echo "current location did not converge to the final coordinate: $projection" >&2
  exit 1
fi

ordered_coordinates=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "
SELECT GROUP_CONCAT(latitude ORDER BY created_at, id SEPARATOR ',')
FROM chair_locations
WHERE chair_id = '$chair_id'
  AND latitude > $history_base
  AND latitude <= $last_latitude
"
)
expected_coordinates=
coordinate_index=1
while [ "$coordinate_index" -le "$history_coordinates" ]
do
  latitude=$((history_base + coordinate_index))
  if [ -z "$expected_coordinates" ]; then
    expected_coordinates=$latitude
  else
    expected_coordinates="$expected_coordinates,$latitude"
  fi
  coordinate_index=$((coordinate_index + 1))
done
if [ "$ordered_coordinates" != "$expected_coordinates" ]; then
  echo "same-chair coordinate order was not preserved" >&2
  echo "expected=$expected_coordinates" >&2
  echo "actual=$ordered_coordinates" >&2
  exit 1
fi
echo "OK: all $history_coordinates coordinates persist in order and current location converges"

"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride \
  -e "
INSERT INTO rides (
    id,
    user_id,
    chair_id,
    pickup_latitude,
    pickup_longitude,
    destination_latitude,
    destination_longitude
) VALUES (
    'queue-transition-ride',
    '$user_id',
    '$chair_id',
    $pickup_latitude,
    $pickup_longitude,
    $destination_latitude,
    $destination_longitude
);
INSERT INTO ride_statuses (id, ride_id, status) VALUES
    ('queue-transition-matching', 'queue-transition-ride', 'MATCHING'),
    ('queue-transition-enroute', 'queue-transition-ride', 'ENROUTE');
"

post_coordinate "$pickup_latitude" "$pickup_longitude"
post_coordinate "$pickup_latitude" "$pickup_longitude"
deadline=$(( $(date '+%s') + 3 ))
pickup_count=0
pickup_history_count=0
while [ "$(date '+%s')" -le "$deadline" ]
do
  pickup_result=$(
    "$compose" exec -T db mysql \
      --batch \
      --skip-column-names \
      -uisucon \
      -pisucon \
      isuride \
      -e "
SELECT
  (SELECT COUNT(*)
   FROM ride_statuses
   WHERE ride_id = 'queue-transition-ride'
     AND status = 'PICKUP'),
  (SELECT COUNT(*)
   FROM chair_locations
   WHERE chair_id = '$chair_id'
     AND latitude = $pickup_latitude
     AND longitude = $pickup_longitude)
"
  )
  pickup_count=$(printf '%s\n' "$pickup_result" | cut -f 1)
  pickup_history_count=$(printf '%s\n' "$pickup_result" | cut -f 2)
  if [ "$pickup_count" = "1" ] && [ "$pickup_history_count" = "2" ]; then
    break
  fi
  sleep 0.05
done
if [ "$pickup_count" != "1" ] || [ "$pickup_history_count" != "2" ]; then
  echo "duplicate pickup coordinates did not produce exactly one PICKUP: status=$pickup_count history=$pickup_history_count" >&2
  exit 1
fi

curl \
  --fail \
  --silent \
  --show-error \
  --connect-timeout "$curl_connect_timeout" \
  --max-time "$curl_max_time" \
  --request POST \
  --cookie "chair_session=$chair_token" \
  --header "Content-Type: application/json" \
  --data '{"status":"CARRYING"}' \
  "$base_url/api/chair/rides/queue-transition-ride/status" \
  >/dev/null

post_coordinate "$destination_latitude" "$destination_longitude"
post_coordinate "$destination_latitude" "$destination_longitude"
deadline=$(( $(date '+%s') + 3 ))
arrived_count=0
destination_history_count=0
while [ "$(date '+%s')" -le "$deadline" ]
do
  arrived_result=$(
    "$compose" exec -T db mysql \
      --batch \
      --skip-column-names \
      -uisucon \
      -pisucon \
      isuride \
      -e "
SELECT
  (SELECT COUNT(*)
   FROM ride_statuses
   WHERE ride_id = 'queue-transition-ride'
     AND status = 'ARRIVED'),
  (SELECT COUNT(*)
   FROM chair_locations
   WHERE chair_id = '$chair_id'
     AND latitude = $destination_latitude
     AND longitude = $destination_longitude)
"
  )
  arrived_count=$(printf '%s\n' "$arrived_result" | cut -f 1)
  destination_history_count=$(printf '%s\n' "$arrived_result" | cut -f 2)
  if [ "$arrived_count" = "1" ] && [ "$destination_history_count" = "2" ]; then
    break
  fi
  sleep 0.05
done
if [ "$arrived_count" != "1" ] || [ "$destination_history_count" != "2" ]; then
  echo "duplicate destination coordinates did not produce exactly one ARRIVED: status=$arrived_count history=$destination_history_count" >&2
  exit 1
fi
echo "OK: duplicate coordinates produce PICKUP and ARRIVED exactly once"

# A burst followed immediately by initialize exercises the queue-generation
# boundary. Requests acknowledged before initialize may either commit before the
# write lock or become stale after the generation changes, but none may reappear
# in the newly initialized database.
post_coordinates "$reset_base" "$reset_coordinates"
response=$(initialize)
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize returned an unexpected response after the burst: $response" >&2
    exit 1
    ;;
esac
sleep 0.5
stale_rows=$(coordinate_count "$reset_base" "$reset_coordinates")
if [ "$stale_rows" != "0" ]; then
  echo "coordinates from the previous generation reappeared after initialize: $stale_rows" >&2
  exit 1
fi
echo "OK: initialize discards queued work from the previous database generation"

assert_no_queue_errors
echo "OK: queue full, stopped worker, and post-acknowledgement persistence errors are zero"

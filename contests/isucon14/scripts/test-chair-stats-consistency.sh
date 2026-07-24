#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}

initialize() {
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
  initialize >/dev/null || true
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

response=$(initialize)
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac

compare_history_and_projection() {
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride <<'SQL'
WITH legacy_stats AS (
  SELECT chair_id,
         COUNT(*)        AS total_rides_count,
         SUM(evaluation) AS total_evaluation_sum
  FROM (
    SELECT rides.id,
           rides.chair_id,
           rides.evaluation
    FROM rides
    INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
    WHERE rides.chair_id IS NOT NULL
      AND rides.evaluation IS NOT NULL
    GROUP BY rides.id, rides.chair_id, rides.evaluation
    HAVING SUM(ride_statuses.status = 'ARRIVED') > 0
       AND SUM(ride_statuses.status = 'CARRYING') > 0
       AND SUM(ride_statuses.status = 'COMPLETED') > 0
  ) AS completed_rides
  GROUP BY chair_id
)
SELECT (SELECT COUNT(*) FROM chairs) AS chair_count,
       COUNT(*)                      AS mismatch_count
FROM chairs
LEFT JOIN legacy_stats ON legacy_stats.chair_id = chairs.id
LEFT JOIN chair_stats ON chair_stats.chair_id = chairs.id
WHERE COALESCE(legacy_stats.total_rides_count, 0)
        <> COALESCE(chair_stats.total_rides_count, 0)
   OR COALESCE(legacy_stats.total_evaluation_sum, 0)
        <> COALESCE(chair_stats.total_evaluation_sum, 0);
SQL
}

assert_consistent() {
  label=$1
  comparison=$(compare_history_and_projection)
  chair_count=$(printf '%s\n' "$comparison" | cut -f 1)
  mismatch_count=$(printf '%s\n' "$comparison" | cut -f 2)

  if [ "$chair_count" != "500" ]; then
    echo "chair stats ($label): expected initial chair count=500 actual=$chair_count" >&2
    exit 1
  fi

  if [ "$mismatch_count" != "0" ]; then
    echo "chair stats ($label): legacy aggregate mismatch count=$mismatch_count" >&2
    exit 1
  fi
}

wait_for_webapp() {
  attempt=0
  until "$compose" exec -T webapp \
    bash -c 'exec 3<>/dev/tcp/127.0.0.1/8080' >/dev/null 2>&1
  do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 60 ]; then
      echo "webapp did not become ready after restart" >&2
      exit 1
    fi
    sleep 0.2
  done
}

assert_consistent "initialize"

malformed_ride_id=$(printf 'R%025d' "$$")
malformed_arrived_id=$(printf 'A%025d' "$$")
malformed_completed_id=$(printf 'D%025d' "$$")
orphan_chair_id=$(printf 'O%025d' "$$")

"$compose" exec -T db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  -pisucon \
  isuride <<SQL
SET @fixture_chair_id = (SELECT id FROM chairs ORDER BY id LIMIT 1);
SET @fixture_user_id = (SELECT id FROM users ORDER BY id LIMIT 1);
SET @missing_chair_id = (
  SELECT chair_id
  FROM chair_stats
  WHERE chair_id <> @fixture_chair_id
  ORDER BY chair_id
  LIMIT 1
);

INSERT INTO rides (
  id,
  user_id,
  chair_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude,
  evaluation
) VALUES (
  '$malformed_ride_id',
  @fixture_user_id,
  @fixture_chair_id,
  0,
  0,
  1,
  1,
  5
);
INSERT INTO ride_statuses (id, ride_id, status)
VALUES
  ('$malformed_arrived_id', '$malformed_ride_id', 'ARRIVED'),
  ('$malformed_completed_id', '$malformed_ride_id', 'COMPLETED');

DELETE FROM chair_stats
WHERE chair_id = @missing_chair_id;
INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
) VALUES (
  @fixture_chair_id,
  999999,
  999999
) AS injected
ON DUPLICATE KEY UPDATE
  total_rides_count = injected.total_rides_count,
  total_evaluation_sum = injected.total_evaluation_sum;
INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
) VALUES (
  '$orphan_chair_id',
  1,
  5
);
SQL

"$compose" restart webapp >/dev/null
wait_for_webapp

assert_consistent "restart repair"

orphan_count=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "SELECT COUNT(*) FROM chair_stats WHERE chair_id = '$orphan_chair_id'"
)
if [ "$orphan_count" != "0" ]; then
  echo "chair stats (restart repair): stale orphan row remains" >&2
  exit 1
fi

echo "OK: initialize and restart repair match the legacy history aggregate"

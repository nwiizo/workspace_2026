#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-20}

initialize()
{
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --request POST \
    --header "Content-Type: application/json" \
    --data "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize" >/dev/null
}

# shellcheck disable=SC2329
cleanup()
{
  if ! initialize >/dev/null 2>&1
  then
    echo "warning: failed to initialize after owner distance fixture" >&2
  fi
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

command -v jq >/dev/null 2>&1 || {
  echo "jq is required" >&2
  exit 1
}
command -v perl >/dev/null 2>&1 || {
  echo "Perl with Time::HiRes is required" >&2
  exit 1
}

monotonic_now()
{
  perl -MTime::HiRes=clock_gettime,CLOCK_MONOTONIC \
    -e 'printf "%.6f\n", clock_gettime(CLOCK_MONOTONIC)'
}

initialize

fixture=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT owners.access_token,
       chairs.id,
       latest.latitude,
       latest.longitude
FROM owners
INNER JOIN chairs
        ON chairs.owner_id = owners.id
INNER JOIN LATERAL (
  SELECT latitude, longitude
  FROM chair_locations
  WHERE chair_id = chairs.id
  ORDER BY created_at DESC, id DESC
  LIMIT 1
) AS latest ON TRUE
ORDER BY owners.id, chairs.id
LIMIT 1
"
)

# The query returns four whitespace-separated scalar columns.
# shellcheck disable=SC2086
set -- $fixture
owner_token=$1
chair_id=$2
previous_latitude=$3
previous_longitude=$4
next_latitude=$((previous_latitude + 1))
location_id=$(printf 'W%025d' "$$")

owner_chair()
{
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "owner_session=$owner_token" \
    "$base_url/api/owner/chairs" |
    jq \
      --arg chair_id "$chair_id" \
      --raw-output \
      '.chairs[] |
       select(.id == $chair_id) |
       [
         .total_distance,
         has("total_distance_updated_at"),
         (.total_distance_updated_at // "null")
       ] |
       @tsv'
}

baseline=$(owner_chair)
baseline_total=$(printf '%s\n' "$baseline" | cut -f 1)
baseline_updated_at=$(printf '%s\n' "$baseline" | cut -f 3)

commit_deadline_started_at=$(monotonic_now)
fixture_updated_at=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride \
  -e "
START TRANSACTION;
SET @recorded_at = NOW(6);

INSERT INTO chair_locations (
  id,
  chair_id,
  latitude,
  longitude,
  created_at
) VALUES (
  '$location_id',
  '$chair_id',
  $next_latitude,
  $previous_longitude,
  @recorded_at
);

INSERT INTO chair_current_locations (
  chair_id,
  location_id,
  latitude,
  longitude,
  created_at
) VALUES (
  '$chair_id',
  '$location_id',
  $next_latitude,
  $previous_longitude,
  @recorded_at
)
ON DUPLICATE KEY UPDATE
  location_id = VALUES(location_id),
  latitude = VALUES(latitude),
  longitude = VALUES(longitude),
  created_at = VALUES(created_at);

COMMIT;

SELECT CAST(FLOOR(UNIX_TIMESTAMP(@recorded_at) * 1000) AS UNSIGNED);
"
)

immediate=$(owner_chair)
immediate_total=$(printf '%s\n' "$immediate" | cut -f 1)
immediate_has_updated_at=$(printf '%s\n' "$immediate" | cut -f 2)

if [ "$immediate_total" != "$baseline_total" ] ||
  [ "$immediate_has_updated_at" != "false" ]
then
  echo "owner response exposed the unacknowledged coordinate immediately" >&2
  echo "baseline=$baseline immediate=$immediate" >&2
  exit 1
fi

deadline=$(awk -v started_at="$commit_deadline_started_at" \
  'BEGIN { print started_at + 3 }')
eventual=$immediate
while current_time=$(monotonic_now) &&
  awk -v current_time="$current_time" -v deadline="$deadline" \
    'BEGIN { exit !(current_time < deadline) }'
do
  eventual=$(owner_chair)
  eventual_total=$(printf '%s\n' "$eventual" | cut -f 1)
  eventual_has_updated_at=$(printf '%s\n' "$eventual" | cut -f 2)
  eventual_updated_at=$(printf '%s\n' "$eventual" | cut -f 3)
  if [ "$eventual_total" -eq $((baseline_total + 1)) ] &&
    [ "$eventual_has_updated_at" = "true" ] &&
    [ "$eventual_updated_at" != "null" ] &&
    [ "$eventual_updated_at" != "$baseline_updated_at" ] &&
    [ "$eventual_updated_at" -ge "$fixture_updated_at" ]
  then
    finished_at=$(monotonic_now)
    elapsed=$(awk -v started_at="$commit_deadline_started_at" \
      -v finished_at="$finished_at" \
      'BEGIN { printf "%.3f", finished_at - started_at }')
    printf 'baseline: %s\n' "$baseline"
    printf 'immediate: %s\n' "$immediate"
    printf 'eventual: %s after %ss\n' "$eventual" "$elapsed"
    exit 0
  fi
  sleep 0.1
done

echo "owner response did not include the coordinate before the deadline" >&2
echo "baseline=$baseline immediate=$immediate eventual=$eventual" >&2
exit 1

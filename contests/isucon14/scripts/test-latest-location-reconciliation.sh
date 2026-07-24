#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${APP_BASE_URL:-http://localhost:8080}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}

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

initialize()
{
  curl --fail --silent --show-error \
    --max-time 30 \
    --header "Content-Type: application/json" \
    --data "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize" >/dev/null
}

cleanup()
{
  initialize || true
}

trap cleanup EXIT HUP INT TERM
initialize

user_token=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "SELECT access_token FROM users ORDER BY id LIMIT 1"
)
chair_row=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "
SELECT chairs.id, latest_location.latitude, latest_location.longitude
FROM chairs
INNER JOIN LATERAL (
  SELECT latitude, longitude
  FROM chair_locations
  WHERE chair_id = chairs.id
  ORDER BY created_at DESC, id DESC
  LIMIT 1
) AS latest_location ON TRUE
WHERE NOT EXISTS (
  SELECT 1
  FROM rides
  WHERE rides.chair_id = chairs.id
    AND rides.evaluation IS NULL
)
ORDER BY chairs.id
LIMIT 1
"
)

# The query returns four whitespace-separated scalar columns.
# shellcheck disable=SC2086
set -- $chair_row
chair_id=$1
initial_latitude=$2
initial_longitude=$3

"$compose" exec -T db mysql -uroot -pisucon isuride \
  -e "UPDATE chairs SET is_active = TRUE WHERE id = '$chair_id'"

coordinate_for()
{
  latitude=$1
  longitude=$2
  curl --fail --silent --show-error \
    --connect-timeout 0.25 \
    --max-time 0.5 \
    --cookie "app_session=$user_token" \
    "$base_url/api/app/nearby-chairs?latitude=$latitude&longitude=$longitude&distance=0" |
    jq --arg chair_id "$chair_id" --raw-output \
      '.chairs[] | select(.id == $chair_id) |
       [.current_coordinate.latitude, .current_coordinate.longitude] | @tsv'
}

wait_for_coordinate()
{
  expected_latitude=$1
  expected_longitude=$2
  started_at=$(monotonic_now)
  deadline=$(awk -v started_at="$started_at" 'BEGIN { print started_at + 3 }')

  while current_time=$(monotonic_now) &&
    awk -v current_time="$current_time" -v deadline="$deadline" \
      'BEGIN { exit !(current_time < deadline) }'
  do
    actual_coordinate=$(coordinate_for "$expected_latitude" "$expected_longitude")
    if [ "$actual_coordinate" = "$expected_latitude	$expected_longitude" ]; then
      finished_at=$(monotonic_now)
      elapsed=$(awk -v started_at="$started_at" -v finished_at="$finished_at" \
        'BEGIN { printf "%.3f", finished_at - started_at }')
      awk -v elapsed="$elapsed" 'BEGIN { exit !(elapsed <= 3) }'
      printf '%s\n' "$elapsed"
      return 0
    fi
    sleep 0.2
  done

  return 1
}

initial_coordinate=$(coordinate_for "$initial_latitude" "$initial_longitude")
test "$initial_coordinate" = "$initial_latitude	$initial_longitude"

stale_chair_id=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "
SELECT chair_id
FROM chair_current_locations
WHERE chair_id <> '$chair_id'
ORDER BY chair_id
LIMIT 1
"
)
stale_update_count=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
  -e "
DELETE FROM chair_current_locations
WHERE chair_id = '$chair_id';
UPDATE chair_current_locations
SET location_id = '00000000000000000000000000',
    latitude = -1,
    longitude = -1,
    created_at = '1970-01-01 00:00:00.000000'
WHERE chair_id = '$stale_chair_id';
SELECT ROW_COUNT();
"
)
test "$stale_update_count" = "1"
"$compose" restart webapp >/dev/null

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

current_location_mismatches=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "
SELECT COUNT(*)
FROM (
  SELECT chair_id,
         id,
         latitude,
         longitude,
         created_at,
         ROW_NUMBER() OVER (
           PARTITION BY chair_id
           ORDER BY created_at DESC, id DESC
         ) AS row_rank
  FROM chair_locations
) AS latest
LEFT JOIN chair_current_locations AS current
  ON current.chair_id = latest.chair_id
WHERE latest.row_rank = 1
  AND (
    current.chair_id IS NULL
    OR current.location_id <> latest.id
    OR current.latitude <> latest.latitude
    OR current.longitude <> latest.longitude
    OR current.created_at <> latest.created_at
  )
"
)
test "$current_location_mismatches" = "0"

history_and_current_counts=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "
SELECT
  (SELECT COUNT(DISTINCT chair_id) FROM chair_locations),
  (SELECT COUNT(*) FROM chair_current_locations)
"
)
# The query returns two whitespace-separated scalar columns.
# shellcheck disable=SC2086
set -- $history_and_current_counts
test "$1" = "$2"
echo "OK: startup repaired missing and stale current-location rows"

location_id=$(printf 'R%015d%010d' "$(date +%s)" "$$")
direct_latitude=777
direct_longitude=888
"$compose" exec -T db mysql -uroot -pisucon isuride \
  -e "
START TRANSACTION;
SET @direct_at = NOW(6);
INSERT INTO chair_locations (id, chair_id, latitude, longitude, created_at)
VALUES ('$location_id', '$chair_id', $direct_latitude, $direct_longitude, @direct_at);
INSERT INTO chair_current_locations (
  chair_id,
  location_id,
  latitude,
  longitude,
  created_at
)
VALUES (
  '$chair_id',
  '$location_id',
  $direct_latitude,
  $direct_longitude,
  @direct_at
) AS new
ON DUPLICATE KEY UPDATE
  latitude = new.latitude,
  longitude = new.longitude,
  location_id = new.location_id,
  created_at = new.created_at;
COMMIT;
"

reconciliation_elapsed=$(wait_for_coordinate "$direct_latitude" "$direct_longitude")
echo "OK: direct DB update converged through reconciliation in ${reconciliation_elapsed}s (limit: 3.000s)"

tie_id_base=$(printf 'T%014d%010d' "$(date +%s)" "$$")
tie_id_low="${tie_id_base}A"
tie_id_high="${tie_id_base}B"
tie_latitude=999
tie_longitude=111
"$compose" exec -T db mysql -uroot -pisucon isuride \
  -e "
START TRANSACTION;
SET @tie_at = NOW(6) + INTERVAL 1 SECOND;
INSERT INTO chair_locations (id, chair_id, latitude, longitude, created_at)
VALUES
  ('$tie_id_low', '$chair_id', 123, 456, @tie_at),
  ('$tie_id_high', '$chair_id', $tie_latitude, $tie_longitude, @tie_at);
INSERT INTO chair_current_locations (chair_id, location_id, latitude, longitude, created_at)
VALUES ('$chair_id', '$tie_id_high', $tie_latitude, $tie_longitude, @tie_at) AS new
ON DUPLICATE KEY UPDATE
  latitude = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.latitude,
    chair_current_locations.latitude
  ),
  longitude = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.longitude,
    chair_current_locations.longitude
  ),
  location_id = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.location_id,
    chair_current_locations.location_id
  ),
  created_at = GREATEST(new.created_at, chair_current_locations.created_at);
INSERT INTO chair_current_locations (chair_id, location_id, latitude, longitude, created_at)
VALUES ('$chair_id', '$tie_id_low', 123, 456, @tie_at) AS new
ON DUPLICATE KEY UPDATE
  latitude = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.latitude,
    chair_current_locations.latitude
  ),
  longitude = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.longitude,
    chair_current_locations.longitude
  ),
  location_id = IF(
    new.created_at > chair_current_locations.created_at
      OR (
        new.created_at = chair_current_locations.created_at
        AND new.location_id > chair_current_locations.location_id
      ),
    new.location_id,
    chair_current_locations.location_id
  ),
  created_at = GREATEST(new.created_at, chair_current_locations.created_at);
COMMIT;
"

tie_current_row=$(
  "$compose" exec -T db mysql -uroot -pisucon --batch --skip-column-names isuride \
    -e "
SELECT location_id, latitude, longitude
FROM chair_current_locations
WHERE chair_id = '$chair_id'
"
)
test "$tie_current_row" = "$tie_id_high	$tie_latitude	$tie_longitude"

tie_elapsed=$(wait_for_coordinate "$tie_latitude" "$tie_longitude")
echo "OK: equal timestamps select the lexicographically greatest location ID (${tie_elapsed}s)"

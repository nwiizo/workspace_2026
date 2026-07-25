#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${APP_BASE_URL:-http://localhost:8080}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
chair_id=b50chair000000000000000001

initialize()
{
  curl \
    --fail \
    --silent \
    --show-error \
    --max-time 30 \
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
    echo "warning: failed to initialize after current-location trigger fixture" >&2
  fi
}

mysql_query()
{
  "$compose" exec -T \
    --env MYSQL_PWD=isucon \
    db \
    mysql \
    --batch \
    --raw \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "$1"
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

initialize

trigger_count=$(
  mysql_query "
SELECT COUNT(*)
FROM information_schema.TRIGGERS
WHERE TRIGGER_SCHEMA = DATABASE()
  AND TRIGGER_NAME = 'chair_locations_after_insert_current'
  AND EVENT_MANIPULATION = 'INSERT'
  AND ACTION_TIMING = 'AFTER'
"
)
test "$trigger_count" = "1"

# Insert order intentionally differs from the canonical (created_at, id) order:
# an older timestamp and a lower tie-break ID must not rewind the projection.
mysql_query "
INSERT INTO chair_locations VALUES
  ('b50loc00000000000000000002', '$chair_id', 20, 20, '2026-01-01 00:00:02.000000');
INSERT INTO chair_locations VALUES
  ('b50loc00000000000000000004', '$chair_id', 10, 10, '2026-01-01 00:00:01.000000');
INSERT INTO chair_locations VALUES
  ('b50loc00000000000000000003', '$chair_id', 30, 30, '2026-01-01 00:00:02.000000');
INSERT INTO chair_locations VALUES
  ('b50loc00000000000000000001', '$chair_id', 40, 40, '2026-01-01 00:00:02.000000');
"

current_row=$(
  mysql_query "
SELECT CONCAT(
  location_id,
  '|',
  latitude,
  '|',
  longitude,
  '|',
  DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s.%f')
)
FROM chair_current_locations
WHERE chair_id = '$chair_id'
"
)
expected_row='b50loc00000000000000000003|30|30|2026-01-01 00:00:02.000000'
test "$current_row" = "$expected_row"

# The trigger runs in the INSERT transaction. Rolling the history row back must
# also roll back its current-state update.
mysql_query "
START TRANSACTION;
INSERT INTO chair_locations VALUES
  ('b50loc00000000000000000005', '$chair_id', 50, 50, '2026-01-01 00:00:03.000000');
ROLLBACK;
"

rollback_state=$(
  mysql_query "
SELECT CONCAT(
  (SELECT COUNT(*)
   FROM chair_locations
   WHERE id = 'b50loc00000000000000000005'),
  '|',
  (SELECT location_id
   FROM chair_current_locations
   WHERE chair_id = '$chair_id')
)
"
)
test "$rollback_state" = '0|b50loc00000000000000000003'

echo "OK: history trigger preserves latest ordering and transaction rollback"

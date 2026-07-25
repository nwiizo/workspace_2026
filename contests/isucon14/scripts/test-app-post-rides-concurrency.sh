#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
parallel_count=${PARALLEL_COUNT:-8}

case "$parallel_count" in
  '' | *[!0-9]*)
    echo "PARALLEL_COUNT must be an integer of at least 2" >&2
    exit 2
    ;;
esac
if [ "$parallel_count" -lt 2 ]; then
  echo "PARALLEL_COUNT must be an integer of at least 2" >&2
  exit 2
fi

command -v jq >/dev/null 2>&1 || {
  echo "jq is required" >&2
  exit 1
}

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/isucon14-app-post-rides.XXXXXX")

initialize() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --request POST \
    --header 'content-type: application/json' \
    --data "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize" >/dev/null
}

cleanup() {
  initialize >/dev/null 2>&1 || true
  rm -r -- "$temp_dir"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

post_ride() {
  label=$1
  access_token=$2
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --output "$temp_dir/$label.json" \
    --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --cookie "app_session=$access_token" \
    --data '{
      "pickup_coordinate":{"latitude":0,"longitude":0},
      "destination_coordinate":{"latitude":20,"longitude":0}
    }' \
    "$base_url/api/app/rides"
}

post_registration() {
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --output "$temp_dir/reward-registration.json" \
    --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --data '{
      "username":"post-rides-reward-guest",
      "firstname":"Ride",
      "lastname":"Reward",
      "date_of_birth":"2000-01-01",
      "invitation_code":"post-rides-reward-invite"
    }' \
    "$base_url/api/app/users"
}

assert_status() {
  label=$1
  actual=$2
  expected=$3
  if [ "$actual" != "$expected" ]; then
    echo "$label: expected HTTP $expected, got $actual" >&2
    jq -c . "$temp_dir/$label.json" >&2 || true
    exit 1
  fi
}

initialize

integrity_violations=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  (
    SELECT COUNT(*)
    FROM rides
    WHERE evaluation IS NULL
      AND EXISTS (
        SELECT 1
        FROM ride_statuses
        WHERE ride_statuses.ride_id = rides.id
          AND ride_statuses.status = 'COMPLETED'
      )
  ) + (
    SELECT COUNT(*)
    FROM rides
    WHERE evaluation IS NOT NULL
      AND NOT EXISTS (
        SELECT 1
        FROM ride_statuses
        WHERE ride_statuses.ride_id = rides.id
          AND ride_statuses.status = 'COMPLETED'
      )
  )
"
)
if [ "$integrity_violations" -ne 0 ]; then
  echo "evaluation / COMPLETED invariant violations: $integrity_violations" >&2
  exit 1
fi

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride <<'SQL'
INSERT INTO users (
  id,
  username,
  firstname,
  lastname,
  date_of_birth,
  access_token,
  invitation_code
) VALUES
  (
    'post-rides-first-user',
    'post-rides-first',
    'Ride',
    'First',
    '2000-01-01',
    'post-rides-first-token',
    'post-rides-first-invite'
  ),
  (
    'post-rides-next-user',
    'post-rides-next',
    'Ride',
    'Next',
    '2000-01-01',
    'post-rides-next-token',
    'post-rides-next-invite'
  ),
  (
    'post-rides-active-user',
    'post-rides-active',
    'Ride',
    'Active',
    '2000-01-01',
    'post-rides-active-token',
    'post-rides-active-invite'
  ),
  (
    'post-rides-par-user',
    'post-rides-parallel',
    'Ride',
    'Parallel',
    '2000-01-01',
    'post-rides-par-token',
    'post-rides-par-invite'
  ),
  (
    'post-rides-reward-user',
    'post-rides-reward',
    'Ride',
    'Reward',
    '2000-01-01',
    'post-rides-reward-token',
    'post-rides-reward-invite'
  );

INSERT INTO rides (
  id,
  user_id,
  pickup_latitude,
  pickup_longitude,
  destination_latitude,
  destination_longitude,
  evaluation,
  created_at,
  updated_at
) VALUES
  (
    'post-rides-completed',
    'post-rides-next-user',
    0,
    0,
    10,
    0,
    5,
    '2025-01-01 00:00:00.000000',
    '2025-01-01 00:01:00.000000'
  ),
  (
    'post-rides-active',
    'post-rides-active-user',
    0,
    0,
    10,
    0,
    NULL,
    '2025-01-01 00:00:00.000000',
    '2025-01-01 00:00:00.000000'
  );

INSERT INTO ride_statuses (id, ride_id, status, created_at) VALUES
  (
    'post-rides-complete-st',
    'post-rides-completed',
    'COMPLETED',
    '2025-01-01 00:01:00.000000'
  ),
  (
    'post-rides-active-status',
    'post-rides-active',
    'MATCHING',
    '2025-01-01 00:00:00.000000'
  );

INSERT INTO coupons (user_id, code, discount, created_at, used_by) VALUES
  (
    'post-rides-first-user',
    'CP_OLDER',
    600,
    '2025-01-01 00:00:00.000000',
    NULL
  ),
  (
    'post-rides-first-user',
    'CP_NEW2024',
    3000,
    '2025-01-02 00:00:00.000000',
    NULL
  ),
  (
    'post-rides-next-user',
    'CP_NEW2024',
    3000,
    '2025-01-01 00:00:00.000000',
    'post-rides-completed'
  ),
  (
    'post-rides-next-user',
    'CP_OLDEST_UNUSED',
    600,
    '2025-01-02 00:00:00.000000',
    NULL
  ),
  (
    'post-rides-next-user',
    'CP_NEWER_UNUSED',
    900,
    '2025-01-03 00:00:00.000000',
    NULL
  );
SQL

first_status=$(post_ride first post-rides-first-token)
assert_status first "$first_status" 202
first_ride_id=$(jq -er '.ride_id' "$temp_dir/first.json")
first_fare=$(jq -er '.fare' "$temp_dir/first.json")
if [ "$first_fare" -ne 500 ]; then
  echo "first ride fare: expected=500 actual=$first_fare" >&2
  exit 1
fi

first_coupon_state=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  SUM(code = 'CP_NEW2024' AND used_by = '$first_ride_id'),
  SUM(code = 'CP_OLDER' AND used_by IS NULL)
FROM coupons
WHERE user_id = 'post-rides-first-user'
"
)
if [ "$first_coupon_state" != "1	1" ]; then
  echo "first ride coupon priority: expected='1 1' actual='$first_coupon_state'" >&2
  exit 1
fi

next_status=$(post_ride next post-rides-next-token)
assert_status next "$next_status" 202
next_ride_id=$(jq -er '.ride_id' "$temp_dir/next.json")
next_fare=$(jq -er '.fare' "$temp_dir/next.json")
if [ "$next_fare" -ne 1900 ]; then
  echo "next ride fare: expected=1900 actual=$next_fare" >&2
  exit 1
fi

next_coupon_state=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  SUM(code = 'CP_OLDEST_UNUSED' AND used_by = '$next_ride_id'),
  SUM(code = 'CP_NEWER_UNUSED' AND used_by IS NULL)
FROM coupons
WHERE user_id = 'post-rides-next-user'
"
)
if [ "$next_coupon_state" != "1	1" ]; then
  echo "subsequent ride coupon order: expected='1 1' actual='$next_coupon_state'" >&2
  exit 1
fi

# Force ride creation and a registration that grants the same user a reward
# coupon to wait on the same users row. Either serialization order is valid:
# the ride uses the reward if registration wins, or leaves it unused if the
# ride wins.
"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  --unbuffered \
  -uisucon \
  isuride \
  -e "
START TRANSACTION;
SELECT id
FROM users
WHERE id = 'post-rides-reward-user'
FOR UPDATE;
SELECT 'reward-lock-ready';
DO SLEEP(3);
COMMIT;
" >"$temp_dir/reward-lock.log" &
reward_lock_pid=$!

reward_ready_attempt=0
until grep -qx 'reward-lock-ready' "$temp_dir/reward-lock.log"; do
  if ! kill -0 "$reward_lock_pid" 2>/dev/null; then
    wait "$reward_lock_pid" || true
    echo "reward lock session ended before the ready marker" >&2
    exit 1
  fi
  reward_ready_attempt=$((reward_ready_attempt + 1))
  if [ "$reward_ready_attempt" -ge 100 ]; then
    wait "$reward_lock_pid" || true
    echo "timed out waiting for reward lock ready marker" >&2
    exit 1
  fi
  sleep 0.05
done

post_registration >"$temp_dir/reward-registration.code" &
reward_registration_pid=$!
post_ride reward-ride post-rides-reward-token \
  >"$temp_dir/reward-ride.code" &
reward_ride_pid=$!

wait "$reward_lock_pid"
wait "$reward_registration_pid"
wait "$reward_ride_pid"

reward_registration_status=$(cat "$temp_dir/reward-registration.code")
assert_status reward-registration "$reward_registration_status" 201
reward_ride_status=$(cat "$temp_dir/reward-ride.code")
assert_status reward-ride "$reward_ride_status" 202

reward_guest_id=$(jq -er '.id' "$temp_dir/reward-registration.json")
reward_ride_id=$(jq -er '.ride_id' "$temp_dir/reward-ride.json")
reward_ride_fare=$(jq -er '.fare' "$temp_dir/reward-ride.json")
reward_used_by=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT COALESCE(used_by, 'unused')
FROM coupons
WHERE user_id = 'post-rides-reward-user'
  AND code = CONCAT(
    'RWD_post-rides-reward-invite_',
    '$reward_guest_id'
  )
  AND discount = 1000
"
)
case "$reward_used_by:$reward_ride_fare" in
  "unused:2500" | "$reward_ride_id:1500") ;;
  *)
    echo "reward race result is not a valid serialization: used_by=$reward_used_by fare=$reward_ride_fare" >&2
    exit 1
    ;;
esac

active_status=$(post_ride active post-rides-active-token)
assert_status active "$active_status" 409
jq -e '.message == "ride already exists"' "$temp_dir/active.json" >/dev/null

# Hold the empty user_id range so every request finishes its active-ride check
# before any INSERT becomes visible. The optimized implementation instead
# serializes on the users row, so only the first request reaches this gap lock.
"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --batch \
  --skip-column-names \
  --unbuffered \
  -uisucon \
  isuride \
  -e "
START TRANSACTION;
SELECT id
FROM rides FORCE INDEX (idx_rides_user_created_at)
WHERE user_id = 'post-rides-par-user'
FOR UPDATE;
SELECT 'gap-lock-ready';
DO SLEEP(5);
COMMIT;
" >"$temp_dir/gap-lock.log" &
gap_lock_pid=$!

gap_ready_attempt=0
until grep -qx 'gap-lock-ready' "$temp_dir/gap-lock.log"; do
  if ! kill -0 "$gap_lock_pid" 2>/dev/null; then
    wait "$gap_lock_pid" || true
    echo "gap lock session ended before the ready marker" >&2
    exit 1
  fi
  gap_ready_attempt=$((gap_ready_attempt + 1))
  if [ "$gap_ready_attempt" -ge 100 ]; then
    wait "$gap_lock_pid" || true
    echo "timed out waiting for gap lock ready marker" >&2
    exit 1
  fi
  sleep 0.05
done

i=1
while [ "$i" -le "$parallel_count" ]; do
  (
    post_ride "parallel-$i" post-rides-par-token \
      >"$temp_dir/parallel-$i.code"
  ) &
  i=$((i + 1))
done

wait "$gap_lock_pid"
wait

accepted=0
conflict=0
i=1
while [ "$i" -le "$parallel_count" ]; do
  status=$(cat "$temp_dir/parallel-$i.code")
  case "$status" in
    202) accepted=$((accepted + 1)) ;;
    409) conflict=$((conflict + 1)) ;;
    *)
      echo "parallel-$i: unexpected HTTP $status" >&2
      jq -c . "$temp_dir/parallel-$i.json" >&2 || true
      exit 1
      ;;
  esac
  i=$((i + 1))
done

parallel_db_state=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  COUNT(*),
  SUM(evaluation IS NULL),
  (
    SELECT COUNT(*)
    FROM ride_statuses
    WHERE ride_id IN (
      SELECT id
      FROM rides
      WHERE user_id = 'post-rides-par-user'
    )
      AND status = 'MATCHING'
  )
FROM rides
WHERE user_id = 'post-rides-par-user'
"
)

echo "parallel requests: accepted=$accepted conflict=$conflict db=$parallel_db_state"

expected_conflict=$((parallel_count - 1))
if [ "$accepted" -ne 1 ] || [ "$conflict" -ne "$expected_conflict" ]; then
  echo "parallel ride creation was not serialized per user" >&2
  exit 1
fi
if [ "$parallel_db_state" != "1	1	1" ]; then
  echo "parallel ride DB state: expected='1 1 1' actual='$parallel_db_state'" >&2
  exit 1
fi

echo "app ride creation regression: PASS"

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

response=$(
  initialize
)

case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac

"$compose" exec -T db mysql -uisucon -pisucon isuride <<'SQL'
INSERT INTO users (
    id,
    username,
    firstname,
    lastname,
    date_of_birth,
    access_token,
    invitation_code
) VALUES (
    'test-status-user',
    'test-status-user',
    'Status',
    'Test',
    '2000-01-01',
    'test-status-app-token',
    'test-status-invite'
);

INSERT INTO owners (
    id,
    name,
    access_token,
    chair_register_token
) VALUES (
    'test-status-owner',
    'test-status-owner',
    'test-status-owner-token',
    'test-status-register-token'
);

INSERT INTO chairs (
    id,
    owner_id,
    name,
    model,
    is_active,
    access_token
) VALUES (
    'test-status-chair',
    'test-status-owner',
    'test-status-chair',
    'スピードスター',
    1,
    'test-status-chair-token'
);

INSERT INTO rides (
    id,
    user_id,
    chair_id,
    pickup_latitude,
    pickup_longitude,
    destination_latitude,
    destination_longitude
) VALUES (
    'test-status-ride',
    'test-status-user',
    'test-status-chair',
    0,
    0,
    1,
    1
);

-- PICKUPのcreated_atをCARRYINGより後へずらし、実際に失格した順序を再現する。
INSERT INTO ride_statuses (id, ride_id, status, created_at) VALUES
    ('test-status-matching', 'test-status-ride', 'MATCHING', '2026-01-01 00:00:00.100000'),
    ('test-status-enroute',  'test-status-ride', 'ENROUTE',  '2026-01-01 00:00:00.200000'),
    ('test-status-pickup',   'test-status-ride', 'PICKUP',   '2026-01-01 00:00:00.400000'),
    ('test-status-carrying', 'test-status-ride', 'CARRYING', '2026-01-01 00:00:00.300000');
SQL

extract_status() {
  printf '%s' "$1" | sed -n 's/.*"status":"\([^"]*\)".*/\1/p'
}

assert_notification_order() {
  client_name=$1
  cookie=$2
  endpoint=$3
  actual=

  for expected in MATCHING ENROUTE PICKUP CARRYING; do
    response=$(
      curl \
        --fail \
        --silent \
        --show-error \
        --connect-timeout "$curl_connect_timeout" \
        --max-time "$curl_max_time" \
        --cookie "$cookie" \
        "$base_url$endpoint"
    )
    status=$(extract_status "$response")
    if [ "$status" != "$expected" ]; then
      echo "$client_name notification: expected=$expected actual=$status response=$response" >&2
      exit 1
    fi
    actual="${actual}${actual:+ -> }${status}"
  done

  echo "OK: $client_name notification: $actual"
}

assert_notification_order \
  app \
  "app_session=test-status-app-token" \
  "/api/app/notification"
assert_notification_order \
  chair \
  "chair_session=test-status-chair-token" \
  "/api/chair/notification"

assert_notification_status() {
  client_name=$1
  cookie=$2
  endpoint=$3
  expected=$4
  response=$(
    curl \
      --fail \
      --silent \
      --show-error \
      --connect-timeout "$curl_connect_timeout" \
      --max-time "$curl_max_time" \
      --cookie "$cookie" \
      "$base_url$endpoint"
  )
  status=$(extract_status "$response")
  if [ "$status" != "$expected" ]; then
    echo "$client_name latest notification: expected=$expected actual=$status response=$response" >&2
    exit 1
  fi
  echo "OK: $client_name latest notification fallback: $status"
}

assert_notification_status \
  app \
  "app_session=test-status-app-token" \
  "/api/app/notification" \
  CARRYING
assert_notification_status \
  chair \
  "chair_session=test-status-chair-token" \
  "/api/chair/notification" \
  CARRYING

latest_status=$(
  "$compose" exec -T db \
    mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "SELECT status FROM ride_statuses WHERE ride_id = 'test-status-ride' ORDER BY status DESC LIMIT 1"
)

if [ "$latest_status" != "CARRYING" ]; then
  echo "latest status: expected=CARRYING actual=$latest_status" >&2
  exit 1
fi

echo "OK: latest status: $latest_status"

curl \
  --fail \
  --silent \
  --show-error \
  --connect-timeout "$curl_connect_timeout" \
  --max-time "$curl_max_time" \
  --request POST \
  --header "Content-Type: application/json" \
  --cookie "chair_session=test-status-chair-token" \
  --data '{"latitude":1,"longitude":1}' \
  "$base_url/api/chair/coordinate" \
  >/dev/null

arrived_count=$(
  "$compose" exec -T db \
    mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "SELECT COUNT(*) FROM ride_statuses WHERE ride_id = 'test-status-ride' AND status = 'ARRIVED'"
)

if [ "$arrived_count" != "1" ]; then
  echo "coordinate transition: expected ARRIVED count=1 actual=$arrived_count" >&2
  exit 1
fi

echo "OK: coordinate locking read: CARRYING -> ARRIVED"

# The preceding fallback requests populate the steady-state notification
# cache. The coordinate transition must invalidate both app and chair entries.
assert_notification_status \
  app \
  "app_session=test-status-app-token" \
  "/api/app/notification" \
  ARRIVED
assert_notification_status \
  chair \
  "chair_session=test-status-chair-token" \
  "/api/chair/notification" \
  ARRIVED

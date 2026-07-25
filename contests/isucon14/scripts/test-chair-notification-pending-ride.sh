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

"$compose" exec -T --env MYSQL_PWD=isucon db \
  mysql -uisucon isuride <<'SQL'
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
        't-pend-user-old',
        't-pend-user-old',
        'Stale',
        'Ride',
        '2000-01-01',
        'test-chair-pending-stale-app-token',
        't-pend-invite-old'
    ),
    (
        't-pend-user-new',
        't-pend-user-new',
        'Current',
        'Ride',
        '2000-01-01',
        'test-chair-pending-current-app-token',
        't-pend-invite-new'
    );

INSERT INTO owners (
    id,
    name,
    access_token,
    chair_register_token
) VALUES (
    't-pend-owner',
    'test-chair-pending-owner',
    'test-chair-pending-owner-token',
    'test-chair-pending-register-token'
);

INSERT INTO chairs (
    id,
    owner_id,
    name,
    model,
    is_active,
    access_token
) VALUES (
    't-pend-chair',
    't-pend-owner',
    'test-chair-pending-chair',
    'スピードスター',
    1,
    'test-chair-pending-chair-token'
);

-- stale rideはupdated_atが新しいものの、全statusを送信済みにする。
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
    't-pend-ride-old',
    't-pend-user-old',
    't-pend-chair',
    0,
    0,
    1,
    1,
    5,
    '2026-01-01 00:00:00.000000',
    '2026-01-03 00:00:00.000000'
);

INSERT INTO ride_statuses (
    id,
    ride_id,
    status,
    created_at,
    app_sent_at,
    chair_sent_at
) VALUES
    (
        't-pend-old-match',
        't-pend-ride-old',
        'MATCHING',
        '2026-01-01 00:00:00.100000',
        '2026-01-01 00:00:00.110000',
        '2026-01-01 00:00:00.120000'
    ),
    (
        't-pend-old-enroute',
        't-pend-ride-old',
        'ENROUTE',
        '2026-01-01 00:00:00.200000',
        '2026-01-01 00:00:00.210000',
        '2026-01-01 00:00:00.220000'
    ),
    (
        't-pend-old-pickup',
        't-pend-ride-old',
        'PICKUP',
        '2026-01-01 00:00:00.300000',
        '2026-01-01 00:00:00.310000',
        '2026-01-01 00:00:00.320000'
    ),
    (
        't-pend-old-carry',
        't-pend-ride-old',
        'CARRYING',
        '2026-01-01 00:00:00.400000',
        '2026-01-01 00:00:00.410000',
        '2026-01-01 00:00:00.420000'
    ),
    (
        't-pend-old-arrive',
        't-pend-ride-old',
        'ARRIVED',
        '2026-01-01 00:00:00.500000',
        '2026-01-01 00:00:00.510000',
        '2026-01-01 00:00:00.520000'
    ),
    (
        't-pend-old-complete',
        't-pend-ride-old',
        'COMPLETED',
        '2026-01-01 00:00:00.600000',
        '2026-01-01 00:00:00.610000',
        '2026-01-01 00:00:00.620000'
    );

-- current rideはupdated_atが古い一方、MATCHINGをまだ椅子へ送っていない。
INSERT INTO rides (
    id,
    user_id,
    chair_id,
    pickup_latitude,
    pickup_longitude,
    destination_latitude,
    destination_longitude,
    created_at,
    updated_at
) VALUES (
    't-pend-ride-new',
    't-pend-user-new',
    't-pend-chair',
    2,
    2,
    3,
    3,
    '2026-01-02 00:00:00.000000',
    '2026-01-02 00:00:00.000000'
);

INSERT INTO ride_statuses (
    id,
    ride_id,
    status,
    created_at
) VALUES (
    't-pend-new-match',
    't-pend-ride-new',
    'MATCHING',
    '2026-01-02 00:00:00.100000'
);

-- 別rideのCOMPLETEDだけが先に送信済みという失敗runの反例も固定する。
-- current rideが配信途中なら、このrideの未送信MATCHINGへ切り替えてはならない。
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
    't-pend-ride-anomaly',
    't-pend-user-old',
    't-pend-chair',
    4,
    4,
    5,
    5,
    3,
    '2026-01-01 12:00:00.000000',
    '2026-01-01 12:00:00.000000'
);

INSERT INTO ride_statuses (
    id,
    ride_id,
    status,
    created_at,
    app_sent_at,
    chair_sent_at
) VALUES
    (
        't-pend-anomaly-match',
        't-pend-ride-anomaly',
        'MATCHING',
        '2026-01-01 12:00:00.100000',
        '2026-01-01 12:00:00.110000',
        NULL
    ),
    (
        't-pend-anomaly-complete',
        't-pend-ride-anomaly',
        'COMPLETED',
        '2026-01-01 12:00:00.200000',
        '2026-01-01 12:00:00.210000',
        '2026-01-01 12:00:00.220000'
    );
SQL

response=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "chair_session=test-chair-pending-chair-token" \
    "$base_url/api/chair/notification"
)

ride_id=$(printf '%s' "$response" | jq -r '.data.ride_id // empty')
user_id=$(printf '%s' "$response" | jq -r '.data.user.id // empty')
status=$(printf '%s' "$response" | jq -r '.data.status // empty')

if [ "$ride_id" != "t-pend-ride-new" ]; then
  echo "pending ride selection: expected ride=t-pend-ride-new actual=$ride_id response=$response" >&2
  exit 1
fi
if [ "$user_id" != "t-pend-user-new" ]; then
  echo "pending ride selection: expected user=t-pend-user-new actual=$user_id response=$response" >&2
  exit 1
fi
if [ "$status" != "MATCHING" ]; then
  echo "pending ride selection: expected status=MATCHING actual=$status response=$response" >&2
  exit 1
fi

cursor=$(
  "$compose" exec -T --env MYSQL_PWD=isucon db \
    mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT chair_sent_at IS NOT NULL
FROM ride_statuses
WHERE id = 't-pend-new-match'
"
)

if [ "$cursor" != "1" ]; then
  echo "pending ride cursor: expected=1 actual=$cursor" >&2
  exit 1
fi

echo "OK: chair notification prioritized pending ride: ride=$ride_id user=$user_id status=$status"

# MATCHINGのcursorを進めた直後、次のstatus追加まで空白があっても、以前の完了rideへ
# 戻ってはならない。直近に椅子へMATCHINGを送ったrideを定常fallbackとして返す。
response=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "chair_session=test-chair-pending-chair-token" \
    "$base_url/api/chair/notification"
)

ride_id=$(printf '%s' "$response" | jq -r '.data.ride_id // empty')
user_id=$(printf '%s' "$response" | jq -r '.data.user.id // empty')
status=$(printf '%s' "$response" | jq -r '.data.status // empty')

if [ "$ride_id" != "t-pend-ride-new" ]; then
  echo "current ride fallback: expected ride=t-pend-ride-new actual=$ride_id response=$response" >&2
  exit 1
fi
if [ "$user_id" != "t-pend-user-new" ]; then
  echo "current ride fallback: expected user=t-pend-user-new actual=$user_id response=$response" >&2
  exit 1
fi
if [ "$status" != "MATCHING" ]; then
  echo "current ride fallback: expected status=MATCHING actual=$status response=$response" >&2
  exit 1
fi

echo "OK: chair notification kept current ride fallback: ride=$ride_id user=$user_id status=$status"

curl \
  --fail \
  --silent \
  --show-error \
  --connect-timeout "$curl_connect_timeout" \
  --max-time "$curl_max_time" \
  --request POST \
  --header "Content-Type: application/json" \
  --cookie "chair_session=test-chair-pending-chair-token" \
  --data '{"status":"ENROUTE"}' \
  "$base_url/api/chair/rides/t-pend-ride-new/status" \
  >/dev/null

for phase in pending fallback; do
  response=$(
    curl \
      --fail \
      --silent \
      --show-error \
      --connect-timeout "$curl_connect_timeout" \
      --max-time "$curl_max_time" \
      --cookie "chair_session=test-chair-pending-chair-token" \
      "$base_url/api/chair/notification"
  )

  ride_id=$(printf '%s' "$response" | jq -r '.data.ride_id // empty')
  user_id=$(printf '%s' "$response" | jq -r '.data.user.id // empty')
  status=$(printf '%s' "$response" | jq -r '.data.status // empty')

  if [ "$ride_id" != "t-pend-ride-new" ]; then
    echo "active ride $phase: expected ride=t-pend-ride-new actual=$ride_id response=$response" >&2
    exit 1
  fi
  if [ "$user_id" != "t-pend-user-new" ]; then
    echo "active ride $phase: expected user=t-pend-user-new actual=$user_id response=$response" >&2
    exit 1
  fi
  if [ "$status" != "ENROUTE" ]; then
    echo "active ride $phase: expected status=ENROUTE actual=$status response=$response" >&2
    exit 1
  fi
  if [ "$phase" = "pending" ]; then
    cursor=$(
      "$compose" exec -T --env MYSQL_PWD=isucon db \
        mysql \
        --batch \
        --skip-column-names \
        -uisucon \
        isuride \
        -e "
SELECT chair_sent_at IS NOT NULL
FROM ride_statuses
WHERE ride_id = 't-pend-ride-new'
  AND status = 'ENROUTE'
"
    )
    if [ "$cursor" != "1" ]; then
      echo "ENROUTE cursor: expected=1 actual=$cursor" >&2
      exit 1
    fi
  fi
done

echo "OK: chair notification kept active ride across pending and fallback ENROUTE"

# current rideのCOMPLETEDを配送し終えた後も、COMPLETED送信済みの異常rideに残る
# 古いMATCHINGを「新しい割当」と誤認してはならない。
"$compose" exec -T --env MYSQL_PWD=isucon db \
  mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride \
  -e "
INSERT INTO ride_statuses (
    id,
    ride_id,
    status,
    created_at
) VALUES (
    't-pend-new-pickup',
    't-pend-ride-new',
    'PICKUP',
    CURRENT_TIMESTAMP(6)
)
"

# 正規のstatus APIを通してCARRYINGを追加し、直前のpayload cacheも無効化する。
curl \
  --fail \
  --silent \
  --show-error \
  --connect-timeout "$curl_connect_timeout" \
  --max-time "$curl_max_time" \
  --request POST \
  --header "Content-Type: application/json" \
  --cookie "chair_session=test-chair-pending-chair-token" \
  --data '{"status":"CARRYING"}' \
  "$base_url/api/chair/rides/t-pend-ride-new/status" \
  >/dev/null

"$compose" exec -T --env MYSQL_PWD=isucon db \
  mysql \
  --batch \
  --skip-column-names \
  -uisucon \
  isuride \
  -e "
INSERT INTO ride_statuses (
    id,
    ride_id,
    status,
    created_at,
    app_sent_at,
    chair_sent_at
) VALUES
    (
        't-pend-new-arrived',
        't-pend-ride-new',
        'ARRIVED',
        CURRENT_TIMESTAMP(6),
        CURRENT_TIMESTAMP(6),
        CURRENT_TIMESTAMP(6)
    ),
    (
        't-pend-new-complete',
        't-pend-ride-new',
        'COMPLETED',
        CURRENT_TIMESTAMP(6),
        CURRENT_TIMESTAMP(6),
        CURRENT_TIMESTAMP(6)
    );

UPDATE ride_statuses
SET app_sent_at = CURRENT_TIMESTAMP(6),
    chair_sent_at = CURRENT_TIMESTAMP(6)
WHERE ride_id = 't-pend-ride-new'
  AND status IN ('PICKUP', 'CARRYING');

-- matching送信時刻とupdated_atが同値でも、作成順とIDでfallbackを決定できるようにする。
UPDATE ride_statuses
SET chair_sent_at = '2026-01-04 00:00:00.000000'
WHERE id IN ('t-pend-old-match', 't-pend-new-match');

UPDATE rides
SET updated_at = '2026-01-04 00:00:00.000000'
WHERE id IN ('t-pend-ride-old', 't-pend-ride-new')
"

response=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "chair_session=test-chair-pending-chair-token" \
    "$base_url/api/chair/notification"
)

ride_id=$(printf '%s' "$response" | jq -r '.data.ride_id // empty')
user_id=$(printf '%s' "$response" | jq -r '.data.user.id // empty')
status=$(printf '%s' "$response" | jq -r '.data.status // empty')

if [ "$ride_id" != "t-pend-ride-new" ]; then
  echo "completed fallback: expected ride=t-pend-ride-new actual=$ride_id response=$response" >&2
  exit 1
fi
if [ "$user_id" != "t-pend-user-new" ]; then
  echo "completed fallback: expected user=t-pend-user-new actual=$user_id response=$response" >&2
  exit 1
fi
if [ "$status" != "COMPLETED" ]; then
  echo "completed fallback: expected status=COMPLETED actual=$status response=$response" >&2
  exit 1
fi

echo "OK: chair notification ignored stale pending MATCHING after current COMPLETED"

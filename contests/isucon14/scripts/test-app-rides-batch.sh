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
  initialize >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

command -v jq >/dev/null 2>&1 || {
  echo "jq is required" >&2
  exit 1
}

response=$(initialize)
case "$response" in
  *'"language":"rust"'*) ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac

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
  ) + (
    SELECT COUNT(*)
    FROM (
      SELECT used_by
      FROM coupons
      WHERE used_by IS NOT NULL
      GROUP BY used_by
      HAVING COUNT(*) > 1
    ) AS duplicate_coupon_rides
  )
"
)
if [ "$integrity_violations" -ne 0 ]; then
  echo "初期データのcompletion/coupon不変条件に違反があります: $integrity_violations" >&2
  exit 1
fi

"$compose" exec -T --env MYSQL_PWD=isucon db mysql \
  --default-character-set=utf8mb4 \
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
) VALUES (
  'test-app-rides-user',
  'test-app-rides-user',
  'Rides',
  'Test',
  '2000-01-01',
  'test-app-rides-token',
  'test-app-rides-invite'
);

INSERT INTO owners (
  id,
  name,
  access_token,
  chair_register_token
) VALUES (
  'test-app-rides-owner',
  '履歴テストオーナー',
  'test-app-rides-owner-token',
  'test-app-rides-register-token'
);

INSERT INTO chairs (
  id,
  owner_id,
  name,
  model,
  is_active,
  access_token
) VALUES (
  'test-app-rides-chair',
  'test-app-rides-owner',
  '履歴テスト椅子',
  'スピードスター',
  1,
  'test-app-rides-chair-token'
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
) VALUES
  (
    'test-app-rides-completed-1',
    'test-app-rides-user',
    'test-app-rides-chair',
    0,
    0,
    2,
    3,
    4,
    '2026-01-01 00:00:01.123456',
    '2026-01-01 00:00:09.987654'
  ),
  (
    'test-app-rides-completed-2',
    'test-app-rides-user',
    'test-app-rides-chair',
    10,
    10,
    11,
    11,
    5,
    '2026-01-01 00:00:02.123456',
    '2026-01-01 00:00:10.987654'
  ),
  (
    'test-app-rides-active',
    'test-app-rides-user',
    'test-app-rides-chair',
    20,
    20,
    21,
    21,
    NULL,
    '2026-01-01 00:00:03.123456',
    '2026-01-01 00:00:03.123456'
  ),
  (
    'test-app-rides-completed-3',
    'test-app-rides-user',
    'test-app-rides-chair',
    20,
    20,
    21,
    22,
    3,
    '2026-01-01 00:00:04.123456',
    '2026-01-01 00:00:11.987654'
  );

INSERT INTO ride_statuses (id, ride_id, status, created_at) VALUES
  (
    'test-app-rides-status-1',
    'test-app-rides-completed-1',
    'COMPLETED',
    '2026-01-01 00:00:09.987654'
  ),
  (
    'test-app-rides-status-2',
    'test-app-rides-completed-2',
    'COMPLETED',
    '2026-01-01 00:00:10.987654'
  ),
  (
    'test-app-rides-active-st',
    'test-app-rides-active',
    'MATCHING',
    '2026-01-01 00:00:03.123456'
  ),
  (
    'test-app-rides-status-3',
    'test-app-rides-completed-3',
    'COMPLETED',
    '2026-01-01 00:00:11.987654'
  );

INSERT INTO coupons (user_id, code, discount, used_by) VALUES
  (
    'test-app-rides-user',
    'TEST_DISCOUNT',
    200,
    'test-app-rides-completed-1'
  ),
  (
    'test-app-rides-user',
    'TEST_OVER_DISCOUNT',
    5000,
    'test-app-rides-completed-2'
  );
SQL

response=$(
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "app_session=test-app-rides-token" \
    "$base_url/api/app/rides"
)

printf '%s\n' "$response" | jq -e '
  . == {
    rides: [
      {
        id: "test-app-rides-completed-3",
        pickup_coordinate: {latitude: 20, longitude: 20},
        destination_coordinate: {latitude: 21, longitude: 22},
        chair: {
          id: "test-app-rides-chair",
          owner: "履歴テストオーナー",
          name: "履歴テスト椅子",
          model: "スピードスター"
        },
        fare: 800,
        evaluation: 3,
        requested_at: 1767225604123,
        completed_at: 1767225611987
      },
      {
        id: "test-app-rides-completed-2",
        pickup_coordinate: {latitude: 10, longitude: 10},
        destination_coordinate: {latitude: 11, longitude: 11},
        chair: {
          id: "test-app-rides-chair",
          owner: "履歴テストオーナー",
          name: "履歴テスト椅子",
          model: "スピードスター"
        },
        fare: 500,
        evaluation: 5,
        requested_at: 1767225602123,
        completed_at: 1767225610987
      },
      {
        id: "test-app-rides-completed-1",
        pickup_coordinate: {latitude: 0, longitude: 0},
        destination_coordinate: {latitude: 2, longitude: 3},
        chair: {
          id: "test-app-rides-chair",
          owner: "履歴テストオーナー",
          name: "履歴テスト椅子",
          model: "スピードスター"
        },
        fare: 800,
        evaluation: 4,
        requested_at: 1767225601123,
        completed_at: 1767225609987
      }
    ]
  }
' >/dev/null

printf '%s\n' "app rides batch regression: PASS"

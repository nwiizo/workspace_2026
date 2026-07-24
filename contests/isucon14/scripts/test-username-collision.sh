#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
test_username=duplicate-regression
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/isucon14-username-collision.XXXXXX")

initialize() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    -X POST \
    -H 'content-type: application/json' \
    -d "{\"payment_server\":\"$payment_server\"}" \
    "$base_url/api/initialize" >/dev/null
}

cleanup() {
  initialize >/dev/null 2>&1 || true
  rm -r "$temp_dir"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

post_user() {
  label=$1
  firstname=$2
  date_of_birth=$3
  invitation_code=${4:-}
  invitation_field=
  if [ -n "$invitation_code" ]; then
    invitation_field=",\"invitation_code\":\"$invitation_code\""
  fi
  status=$(
    curl -sS \
      --connect-timeout "$curl_connect_timeout" \
      --max-time "$curl_max_time" \
      -o "$temp_dir/$label.json" \
      -w '%{http_code}' \
      -c "$temp_dir/$label.cookies" \
      -X POST \
      -H 'content-type: application/json' \
      -d "{
        \"username\":\"$test_username\",
        \"firstname\":\"$firstname\",
        \"lastname\":\"User\",
        \"date_of_birth\":\"$date_of_birth\"
        $invitation_field
      }" \
      "$base_url/api/app/users"
  )
  if [ "$status" != 201 ]; then
    echo "$label user registration failed: status=$status" >&2
    jq -c . "$temp_dir/$label.json" >&2 || true
    exit 1
  fi
}

post_payment_method() {
  label=$1
  token=$2
  status=$(
    curl -sS \
      --connect-timeout "$curl_connect_timeout" \
      --max-time "$curl_max_time" \
      -o "$temp_dir/$label-payment.json" \
      -w '%{http_code}' \
      -b "$temp_dir/$label.cookies" \
      -X POST \
      -H 'content-type: application/json' \
      -d "{\"token\":\"$token\"}" \
      "$base_url/api/app/payment-methods"
  )
  if [ "$status" != 204 ]; then
    echo "$label authentication check failed: status=$status" >&2
    jq -c . "$temp_dir/$label-payment.json" >&2 || true
    exit 1
  fi
}

"$script_dir/up.sh" >/dev/null
initialize

post_user first First 2000-01-01
first_invitation_code=$(jq -er '.invitation_code' "$temp_dir/first.json")
case "$first_invitation_code" in
  *[!0-9a-f]*)
    echo "registration returned an unexpected invitation code format." >&2
    exit 1
    ;;
esac
if [ "${#first_invitation_code}" -ne 30 ]; then
  echo "registration returned an unexpected invitation code length." >&2
  exit 1
fi
post_user second Second 2000-01-02 "$first_invitation_code"

first_id=$(jq -er '.id' "$temp_dir/first.json")
second_id=$(jq -er '.id' "$temp_dir/second.json")
if [ "$first_id" = "$second_id" ]; then
  echo "duplicate registrations returned the same user ID: $first_id" >&2
  exit 1
fi
case "$first_id:$second_id" in
  *[!0-9A-Z:]*)
    echo "registration returned an unexpected user ID format." >&2
    exit 1
    ;;
esac
if [ "${#first_id}" -ne 26 ] || [ "${#second_id}" -ne 26 ]; then
  echo "registration returned an unexpected user ID length." >&2
  exit 1
fi

registration_counts=$(
  "$compose" exec -T \
    -e MYSQL_PWD=isucon \
    db \
    mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  COUNT(*),
  COUNT(DISTINCT username),
  SUM(username = '$test_username'),
  SUM(username = CONCAT('~', id))
FROM users
WHERE id IN ('$first_id', '$second_id')
"
)
if [ "$registration_counts" != "2	2	1	1" ]; then
  echo "unexpected stored username counts: $registration_counts" >&2
  exit 1
fi

coupon_counts=$(
  "$compose" exec -T \
    -e MYSQL_PWD=isucon \
    db \
    mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    isuride \
    -e "
SELECT
  SUM(user_id = '$second_id' AND code = 'CP_NEW2024'),
  SUM(user_id = '$second_id' AND code = 'INV_$first_invitation_code'),
  SUM(user_id = '$first_id' AND code LIKE 'RWD_${first_invitation_code}_%')
FROM coupons
WHERE user_id IN ('$first_id', '$second_id')
"
)
if [ "$coupon_counts" != "1	1	1" ]; then
  echo "unexpected coupon counts after the retry: $coupon_counts" >&2
  exit 1
fi

post_payment_method first payment-token-first
post_payment_method second payment-token-second

printf 'username collision regression passed: first=%s second=%s\n' "$first_id" "$second_id"

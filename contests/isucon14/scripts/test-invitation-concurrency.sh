#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
payment_server=${PAYMENT_SERVER_URL:-http://benchmark:12345}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
parallel_inviter_count=${PARALLEL_INVITER_COUNT:-24}

case "$parallel_inviter_count" in
  '' | *[!0-9]*)
    echo "PARALLEL_INVITER_COUNT must be an integer of at least 2" >&2
    exit 2
    ;;
esac
if [ "$parallel_inviter_count" -lt 2 ]; then
  echo "PARALLEL_INVITER_COUNT must be an integer of at least 2" >&2
  exit 2
fi

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/isucon14-invitation-concurrency.XXXXXX")

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
  rm -r "$temp_dir"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

registration_payload() {
  username=$1
  invitation_code=${2:-}
  if [ -n "$invitation_code" ]; then
    jq -cn \
      --arg username "$username" \
      --arg invitation_code "$invitation_code" \
      '{
        username: $username,
        firstname: "Invitation",
        lastname: "Concurrency",
        date_of_birth: "2000-01-01",
        invitation_code: $invitation_code
      }'
  else
    jq -cn \
      --arg username "$username" \
      '{
        username: $username,
        firstname: "Invitation",
        lastname: "Concurrency",
        date_of_birth: "2000-01-01"
      }'
  fi
}

post_registration() {
  label=$1
  username=$2
  invitation_code=${3:-}
  payload=$(registration_payload "$username" "$invitation_code")
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --output "$temp_dir/$label.json" \
    --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --data "$payload" \
    "$base_url/api/app/users"
}

register_inviter() {
  number=$1
  status=$(post_registration "inviter-$number" "ic-inviter-$number-$$")
  if [ "$status" != 201 ]; then
    echo "inviter registration failed: number=$number status=$status" >&2
    jq -c . "$temp_dir/inviter-$number.json" >&2 || true
    exit 1
  fi
  jq -er '.invitation_code' "$temp_dir/inviter-$number.json" \
    >"$temp_dir/inviter-$number.code"
}

mysql_error_count() {
  error_number=$1
  "$compose" exec -T \
    -e MYSQL_PWD=isucon \
    db \
    mysql \
    --batch \
    --skip-column-names \
    -uroot \
    performance_schema \
    -e "
SELECT SUM_ERROR_RAISED
FROM events_errors_summary_global_by_error
WHERE ERROR_NUMBER = $error_number
"
}

wait_for_ready_workers() {
  prefix=$1
  expected=$2
  deadline=$(( $(date +%s) + 10 ))
  while :; do
    ready_count=$(find "$temp_dir" -type f -name "$prefix-ready-*" | wc -l | tr -d ' ')
    if [ "$ready_count" -eq "$expected" ]; then
      return
    fi
    if [ "$(date +%s)" -ge "$deadline" ]; then
      echo "workers did not reach the start barrier: prefix=$prefix ready=$ready_count expected=$expected" >&2
      exit 1
    fi
    sleep 0.01
  done
}

launch_registration() {
  prefix=$1
  number=$2
  username=$3
  invitation_code=$4
  (
    : >"$temp_dir/$prefix-ready-$number"
    while [ ! -f "$temp_dir/$prefix-start" ]; do
      sleep 0.01
    done
    status=$(post_registration "$prefix-$number" "$username" "$invitation_code" || printf 'curl-error')
    printf '%s\n' "$status" >"$temp_dir/$prefix-$number.status"
  ) &
}

assert_statuses() {
  prefix=$1
  expected_count=$2
  expected_status=$3
  number=1
  while [ "$number" -le "$expected_count" ]; do
    status=$(sed -n '1p' "$temp_dir/$prefix-$number.status")
    if [ "$status" != "$expected_status" ]; then
      echo "concurrent registration failed: prefix=$prefix number=$number status=$status" >&2
      jq -c . "$temp_dir/$prefix-$number.json" >&2 || true
      exit 1
    fi
    number=$((number + 1))
  done
}

"$script_dir/up.sh" >/dev/null
initialize

number=1
while [ "$number" -le "$parallel_inviter_count" ]; do
  register_inviter "$number"
  number=$((number + 1))
done

duplicate_errors_before=$(mysql_error_count 1062)
deadlock_errors_before=$(mysql_error_count 1213)

number=1
while [ "$number" -le "$parallel_inviter_count" ]; do
  invitation_code=$(sed -n '1p' "$temp_dir/inviter-$number.code")
  launch_registration \
    distinct \
    "$number" \
    "ic-distinct-$number-$$" \
    "$invitation_code"
  number=$((number + 1))
done
wait_for_ready_workers distinct "$parallel_inviter_count"
: >"$temp_dir/distinct-start"
wait
assert_statuses distinct "$parallel_inviter_count" 201

register_inviter shared
shared_invitation_code=$(sed -n '1p' "$temp_dir/inviter-shared.code")
number=1
while [ "$number" -le 4 ]; do
  launch_registration \
    shared \
    "$number" \
    "ic-shared-$number-$$" \
    "$shared_invitation_code"
  number=$((number + 1))
done
wait_for_ready_workers shared 4
: >"$temp_dir/shared-start"
wait

shared_created=0
shared_rejected=0
number=1
while [ "$number" -le 4 ]; do
  status=$(sed -n '1p' "$temp_dir/shared-$number.status")
  case "$status" in
    201) shared_created=$((shared_created + 1)) ;;
    400) shared_rejected=$((shared_rejected + 1)) ;;
    *)
      echo "unexpected shared invitation status: number=$number status=$status" >&2
      jq -c . "$temp_dir/shared-$number.json" >&2 || true
      exit 1
      ;;
  esac
  number=$((number + 1))
done
if [ "$shared_created" -ne 3 ] || [ "$shared_rejected" -ne 1 ]; then
  echo "shared invitation limit mismatch: created=$shared_created rejected=$shared_rejected" >&2
  exit 1
fi

shared_coupon_counts=$(
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
  SUM(code = 'INV_$shared_invitation_code'),
  SUM(user_id = (
        SELECT id FROM users WHERE invitation_code = '$shared_invitation_code'
      ) AND code LIKE 'RWD_${shared_invitation_code}_%')
FROM coupons
WHERE code = 'INV_$shared_invitation_code'
   OR user_id = (
        SELECT id FROM users WHERE invitation_code = '$shared_invitation_code'
      )
"
)
if [ "$shared_coupon_counts" != "3	3" ]; then
  echo "shared invitation coupon counts mismatch: $shared_coupon_counts" >&2
  exit 1
fi

duplicate_errors_after=$(mysql_error_count 1062)
deadlock_errors_after=$(mysql_error_count 1213)
duplicate_error_delta=$((duplicate_errors_after - duplicate_errors_before))
deadlock_error_delta=$((deadlock_errors_after - deadlock_errors_before))
if [ "$duplicate_error_delta" -ne 0 ] || [ "$deadlock_error_delta" -ne 0 ]; then
  echo "MySQL errors increased: duplicate_delta=$duplicate_error_delta deadlock_delta=$deadlock_error_delta" >&2
  exit 1
fi

printf 'invitation concurrency regression passed: distinct=%s shared_created=%s shared_rejected=%s duplicate_delta=%s deadlock_delta=%s\n' \
  "$parallel_inviter_count" \
  "$shared_created" \
  "$shared_rejected" \
  "$duplicate_error_delta" \
  "$deadlock_error_delta"

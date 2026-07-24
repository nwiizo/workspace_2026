#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
base_url=${BASE_URL:-http://127.0.0.1:${APP_PORT:-8080}}
curl_connect_timeout=${CURL_CONNECT_TIMEOUT:-2}
curl_max_time=${CURL_MAX_TIME:-10}
temp_dir=$(mktemp -d)
cookie_jar="$temp_dir/app-cookies.txt"

cleanup() {
  rm -rf "$temp_dir"
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

initialize() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --request POST \
    --header "Content-Type: application/json" \
    --data '{"payment_server":"http://benchmark:12345"}' \
    "$base_url/api/initialize" \
    >/dev/null
}

auth_query_count() {
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uroot \
    -pisucon \
    performance_schema \
    -e "
SELECT COALESCE(SUM(COUNT_EXECUTE), 0)
FROM prepared_statements_instances
WHERE SQL_TEXT = 'SELECT * FROM users WHERE access_token = ?'
"
}

get_notification() {
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie "$1" \
    --output /dev/null \
    --write-out '%{http_code}' \
    "$base_url/api/app/notification"
}

initialize

initial_token=$(
  "$compose" exec -T db mysql \
    --batch \
    --skip-column-names \
    -uisucon \
    -pisucon \
    isuride \
    -e "SELECT access_token FROM users ORDER BY id LIMIT 1"
)
before_initial=$(auth_query_count)
initial_status=$(get_notification "app_session=$initial_token")
after_initial=$(auth_query_count)
if [ "$initial_status" != "200" ] || [ "$after_initial" != "$before_initial" ]; then
  echo "起動時cache hitがDB認証を省略しませんでした: status=$initial_status count=$before_initial->$after_initial" >&2
  exit 1
fi
echo "OK: initial user authentication is served from cache"

registration_status=$(
  curl \
    --silent \
    --show-error \
    --connect-timeout "$curl_connect_timeout" \
    --max-time "$curl_max_time" \
    --cookie-jar "$cookie_jar" \
    --output /dev/null \
    --write-out '%{http_code}' \
    --request POST \
    --header "Content-Type: application/json" \
    --data "{\"username\":\"auth-cache-$$\",\"firstname\":\"Auth\",\"lastname\":\"Cache\",\"date_of_birth\":\"2000-01-01\"}" \
    "$base_url/api/app/users"
)
if [ "$registration_status" != "201" ]; then
  echo "動的user登録に失敗しました: status=$registration_status" >&2
  exit 1
fi

before_dynamic=$(auth_query_count)
first_dynamic_status=$(get_notification "$cookie_jar")
after_first_dynamic=$(auth_query_count)
second_dynamic_status=$(get_notification "$cookie_jar")
after_second_dynamic=$(auth_query_count)
if [ "$first_dynamic_status" != "200" ] || [ "$second_dynamic_status" != "200" ]; then
  echo "動的userの認証に失敗しました: first=$first_dynamic_status second=$second_dynamic_status" >&2
  exit 1
fi
if [ "$after_first_dynamic" -ne $((before_dynamic + 1)) ]; then
  echo "動的userの最初のcache missが想定外です: count=$before_dynamic->$after_first_dynamic" >&2
  exit 1
fi
if [ "$after_second_dynamic" != "$after_first_dynamic" ]; then
  echo "動的userの2回目がcache hitしませんでした: count=$after_first_dynamic->$after_second_dynamic" >&2
  exit 1
fi
echo "OK: dynamic user uses one DB fallback and is cached"

initialize
stale_status=$(get_notification "$cookie_jar")
if [ "$stale_status" != "401" ]; then
  echo "initialize後も削除済みuserがcacheに残りました: status=$stale_status" >&2
  exit 1
fi

before_reloaded=$(auth_query_count)
reloaded_status=$(get_notification "app_session=$initial_token")
after_reloaded=$(auth_query_count)
if [ "$reloaded_status" != "200" ] || [ "$after_reloaded" != "$before_reloaded" ]; then
  echo "initialize後のcache再構築が失敗しました: status=$reloaded_status count=$before_reloaded->$after_reloaded" >&2
  exit 1
fi
echo "OK: initialize replaces stale entries and reloads initial users"

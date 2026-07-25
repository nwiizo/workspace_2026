#!/bin/sh

set -eu

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi

fixture='
{"id":"periodic","periodic_sample":true}
{"id":"ride-trace","periodic_sample":false}
{"id":"legacy"}
'
filter='select(if has("periodic_sample") then .periodic_sample == true else true end)'
actual=$(printf '%s\n' "$fixture" | jq --compact-output "$filter")
expected='{"id":"periodic","periodic_sample":true}
{"id":"legacy"}'

if [ "$actual" != "$expected" ]; then
  echo "periodic sample filterが想定外の行を通しました。" >&2
  printf 'expected:\n%s\nactual:\n%s\n' "$expected" "$actual" >&2
  exit 1
fi

drive_fixture='
{"id":"pickup","success":true,"world_tick":10,"picked_up_tick":10,"arrived_tick":13}
{"id":"driving-1","success":true,"world_tick":11,"picked_up_tick":10,"arrived_tick":13}
{"id":"driving-2","success":true,"world_tick":12,"picked_up_tick":10,"arrived_tick":13}
{"id":"arrived","success":true,"world_tick":13,"picked_up_tick":10,"arrived_tick":13}
{"id":"failed","success":false,"world_tick":11,"picked_up_tick":10,"arrived_tick":13}
'
drive_filter='select(
  .world_tick > .picked_up_tick and
  .world_tick < .arrived_tick
)'
drive_actual=$(printf '%s\n' "$drive_fixture" | jq --compact-output "$drive_filter")
drive_expected='{"id":"driving-1","success":true,"world_tick":11,"picked_up_tick":10,"arrived_tick":13}
{"id":"driving-2","success":true,"world_tick":12,"picked_up_tick":10,"arrived_tick":13}
{"id":"failed","success":false,"world_tick":11,"picked_up_tick":10,"arrived_tick":13}'

if [ "$drive_actual" != "$drive_expected" ]; then
  echo "drive coordinate filterが採点区間内のattemptを正しく選べませんでした。" >&2
  printf 'expected:\n%s\nactual:\n%s\n' "$drive_expected" "$drive_actual" >&2
  exit 1
fi

commit_fixture='
{"id":"committed-after-cancel","transition_status":"ARRIVED","committed_at_unix_us":200,"outcome":"error_or_cancelled"}
{"id":"not-committed","transition_status":"ARRIVED","committed_at_unix_us":null,"outcome":"success"}
'
commit_actual=$(
  printf '%s\n' "$commit_fixture" |
    jq --compact-output \
      'select(.transition_status == "ARRIVED" and .committed_at_unix_us != null)'
)
commit_expected='{"id":"committed-after-cancel","transition_status":"ARRIVED","committed_at_unix_us":200,"outcome":"error_or_cancelled"}'

if [ "$commit_actual" != "$commit_expected" ]; then
  echo "commit milestone filterがhandler outcomeとDB commitを混同しました。" >&2
  printf 'expected:\n%s\nactual:\n%s\n' "$commit_expected" "$commit_actual" >&2
  exit 1
fi

server_fixture='
{"id":"carrying-boundary","ride_id":"ride-1","committed_at_unix_us":100,"outcome":"success"}
{"id":"inside","ride_id":"ride-1","committed_at_unix_us":150,"outcome":"success"}
{"id":"cancelled","ride_id":"ride-1","committed_at_unix_us":160,"outcome":"error_or_cancelled"}
{"id":"arrived-boundary","ride_id":"ride-1","committed_at_unix_us":200,"outcome":"success"}
{"id":"other-ride","ride_id":"ride-2","committed_at_unix_us":150,"outcome":"success"}
'
server_actual=$(
  printf '%s\n' "$server_fixture" |
    jq --compact-output 'select(
      .ride_id == "ride-1" and
      .outcome == "success" and
      .committed_at_unix_us > 100 and
      .committed_at_unix_us < 200
    )'
)
server_expected='{"id":"inside","ride_id":"ride-1","committed_at_unix_us":150,"outcome":"success"}'

if [ "$server_actual" != "$server_expected" ]; then
  echo "server drive-window filterがride・outcome・commit境界を正しく選べませんでした。" >&2
  printf 'expected:\n%s\nactual:\n%s\n' "$server_expected" "$server_actual" >&2
  exit 1
fi

printf 'diagnostic periodic, drive-window, and commit filters: ok\n'

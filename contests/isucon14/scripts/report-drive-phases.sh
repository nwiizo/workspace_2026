#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
diagnostic_since=${1:-${DIAGNOSTIC_SINCE:-}}
benchmark_log=${2:-${BENCHMARK_OUTPUT_FILE:-}}

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi

if [ -z "$diagnostic_since" ]; then
  echo "診断run開始時刻を第1引数または DIAGNOSTIC_SINCE で指定してください。" >&2
  exit 2
fi
if [ -z "$benchmark_log" ] || [ ! -f "$benchmark_log" ]; then
  echo "診断benchmark出力を第2引数または BENCHMARK_OUTPUT_FILE で指定してください。" >&2
  exit 2
fi

"$script_dir/flush-diagnostics.sh"

raw_web_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-web.XXXXXX")
benchmark_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-benchmark.XXXXXX")
client_coordinate_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-client-coordinate.XXXXXX")
coordinate_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-coordinate.XXXXXX")
notification_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-notification.XXXXXX")
ride_status_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-status.XXXXXX")
correlated_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-correlated.XXXXXX")
drive_coordinate_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-drive-coordinate-window.XXXXXX")

cleanup() {
  rm -f \
    "$raw_web_log" \
    "$benchmark_json" \
    "$client_coordinate_json" \
    "$coordinate_json" \
    "$notification_json" \
    "$ride_status_json" \
    "$correlated_json" \
    "$drive_coordinate_json"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

ISUCON_DIAGNOSTIC=1 "$compose" logs \
  --no-color \
  --timestamps \
  --since "$diagnostic_since" \
  webapp >"$raw_web_log"

sed -n 's/^.*DRIVE_BENCHMARK_DIAGNOSTIC //p' "$benchmark_log" >"$benchmark_json"
sed -n 's/^.*COORDINATE_CLIENT_DIAGNOSTIC //p' "$benchmark_log" >"$client_coordinate_json"
sed -n 's/^.*COORDINATE_DIAGNOSTIC //p' "$raw_web_log" |
  jq --compact-output 'select(.trace_ride == true)' >"$coordinate_json"
sed -n 's/^.*NOTIFICATION_DIAGNOSTIC //p' "$raw_web_log" |
  jq --compact-output 'select(.trace_ride == true)' >"$notification_json"
sed -n 's/^.*RIDE_STATUS_DIAGNOSTIC //p' "$raw_web_log" >"$ride_status_json"

if [ ! -s "$benchmark_json" ]; then
  echo "ベンチマーカーのdrive診断sampleがありません。ISUCON_DIAGNOSTIC=1で実行してください。" >&2
  exit 1
fi
if [ ! -s "$client_coordinate_json" ]; then
  echo "ベンチマーカーのcoordinate診断sampleがありません。" >&2
  exit 1
fi
if [ ! -s "$coordinate_json" ] || [ ! -s "$notification_json" ] || [ ! -s "$ride_status_json" ]; then
  echo "webappのride追跡sampleが揃っていません。" >&2
  echo "coordinate=$(wc -l <"$coordinate_json" | tr -d ' ') notification=$(wc -l <"$notification_json" | tr -d ' ') status=$(wc -l <"$ride_status_json" | tr -d ' ')" >&2
  exit 1
fi

jq -n --compact-output \
  --slurpfile benchmark "$benchmark_json" \
  --slurpfile client "$client_coordinate_json" \
  --slurpfile coordinate "$coordinate_json" \
  --slurpfile notification "$notification_json" \
  --slurpfile ride_status "$ride_status_json" '
  def transition_commit($values; $ride_id; $status):
    [
      $values[] | select(
        .ride_id == $ride_id and
        .transition_status == $status and
        .committed_at_unix_us != null
      )
    ] | sort_by(.committed_at_unix_us) | first;
  def notification_response($values; $ride_id; $endpoint; $status):
    [
      $values[] | select(
        .ride_id == $ride_id and
        .endpoint == $endpoint and
        .ride_status == $status and
        .outcome == "success" and
        .response_built_at_unix_us != null
      )
    ] | sort_by(.response_built_at_unix_us) | first;
  def carrying_commit($values; $ride_id):
    [
      $values[] | select(
        .ride_id == $ride_id and
        .status == "CARRYING" and
        .committed_at_unix_us != null
      )
    ] | sort_by(.committed_at_unix_us) | first;
  def gap_ms($from; $to):
    if $from == null or $to == null then null
    else (($to - $from) / 1000 | floor)
    end;
  $benchmark[] as $bench |
  (transition_commit($coordinate; $bench.ride_id; "PICKUP")) as $pickup_coordinate |
  (notification_response($notification; $bench.ride_id; "app"; "PICKUP")) as $app_pickup |
  (notification_response($notification; $bench.ride_id; "chair"; "PICKUP")) as $chair_pickup |
  (carrying_commit($ride_status; $bench.ride_id)) as $carrying_status |
  (notification_response($notification; $bench.ride_id; "app"; "CARRYING")) as $app_carrying |
  (notification_response($notification; $bench.ride_id; "chair"; "CARRYING")) as $chair_carrying |
  (transition_commit($coordinate; $bench.ride_id; "ARRIVED")) as $arrived_coordinate |
  (notification_response($notification; $bench.ride_id; "app"; "ARRIVED")) as $app_arrived |
  (notification_response($notification; $bench.ride_id; "chair"; "ARRIVED")) as $chair_arrived |
  [
    $client[] |
    select(
      .ride_id == $bench.ride_id and
      .world_tick > $bench.picked_up_tick and
      .world_tick < $bench.arrived_tick
    )
  ] as $drive_coordinates |
  select($carrying_status != null and $arrived_coordinate != null and ($drive_coordinates | length) > 0) |
  {
    ride_id: $bench.ride_id,
    drive_pass: $bench.drive_pass,
    ideal_drive_ticks: $bench.ideal_drive_ticks,
    actual_drive_ticks: $bench.actual_drive_ticks,
    excess_drive_ticks: $bench.excess_drive_ticks,
    pickup_wait_ticks: $bench.pickup_wait_ticks,
    coordinate_requests: ($drive_coordinates | length),
    coordinate_failed: (
      [$drive_coordinates[] | select(.success == false)] | length
    ),
    coordinate_over_30ms: (
      [$drive_coordinates[] | select(.duration_us >= 30000)] | length
    ),
    coordinate_client_durations_us: [$drive_coordinates[].duration_us],
    coordinate_client_avg_us: (
      [$drive_coordinates[].duration_us] | add / length | floor
    ),
    coordinate_client_max_us: (
      [$drive_coordinates[].duration_us] | max
    ),
    estimated_blocked_ticks: (
      [
        $drive_coordinates[].duration_us |
        ([0, (((. + 29999) / 30000 | floor) - 1)] | max)
      ] | add
    ),
    pickup_coordinate_to_app_ms: gap_ms(
      $pickup_coordinate.committed_at_unix_us;
      $app_pickup.response_built_at_unix_us
    ),
    pickup_coordinate_to_chair_ms: gap_ms(
      $pickup_coordinate.committed_at_unix_us;
      $chair_pickup.response_built_at_unix_us
    ),
    app_pickup_to_carrying_commit_ms: gap_ms(
      $app_pickup.response_built_at_unix_us;
      $carrying_status.committed_at_unix_us
    ),
    carrying_commit_to_app_ms: gap_ms(
      $carrying_status.committed_at_unix_us;
      $app_carrying.response_built_at_unix_us
    ),
    carrying_commit_to_chair_ms: gap_ms(
      $carrying_status.committed_at_unix_us;
      $chair_carrying.response_built_at_unix_us
    ),
    carrying_commit_to_arrived_coordinate_ms: gap_ms(
      $carrying_status.committed_at_unix_us;
      $arrived_coordinate.committed_at_unix_us
    ),
    arrived_coordinate_to_app_ms: gap_ms(
      $arrived_coordinate.committed_at_unix_us;
      $app_arrived.response_built_at_unix_us
    ),
    arrived_coordinate_to_chair_ms: gap_ms(
      $arrived_coordinate.committed_at_unix_us;
      $chair_arrived.response_built_at_unix_us
    )
  }
' >"$correlated_json"

if [ ! -s "$correlated_json" ]; then
  echo "ベンチマーカーとwebappを同じride IDで結合できませんでした。" >&2
  exit 1
fi

jq -n --compact-output \
  --slurpfile coordinate "$coordinate_json" \
  --slurpfile ride_status "$ride_status_json" \
  --slurpfile correlated "$correlated_json" '
  ($correlated | map(.ride_id)) as $correlated_ride_ids |
  $coordinate[] as $sample |
  (
    [$ride_status[] | select(
      .ride_id == $sample.ride_id and
      .status == "CARRYING" and
      .outcome == "success" and
      .committed_at_unix_us != null
    )] |
    sort_by(.committed_at_unix_us) |
    first
  ) as $carrying |
  (
    [
      $coordinate[] |
      select(
        .ride_id == $sample.ride_id and
        .transition_status == "ARRIVED" and
        .outcome == "success" and
        .committed_at_unix_us != null
      )
    ] |
    sort_by(.committed_at_unix_us) |
    first
  ) as $arrived |
  select(
    ($correlated_ride_ids | index($sample.ride_id)) != null and
    $sample.outcome == "success" and
    $sample.committed_at_unix_us != null and
    $carrying != null and
    $arrived != null and
    $sample.committed_at_unix_us > $carrying.committed_at_unix_us and
    $sample.committed_at_unix_us < $arrived.committed_at_unix_us
  ) |
  $sample
' >"$drive_coordinate_json"

printf 'benchmark drive evaluation\n\n'
printf 'The benchmarker tick values are authoritative. A ride fails drive evaluation when excess is 5 ticks or more.\n\n'
jq --slurp --raw-output '
  def percentile($values; $ratio):
    ($values | sort) as $sorted |
    $sorted[(($sorted | length) - 1) * $ratio | floor];
  def row($label; $values):
    "| \($label) | \($values | length) | " +
    "\(($values | add) / ($values | length) | floor) | " +
    "\(percentile($values; 0.50)) | " +
    "\(percentile($values; 0.95)) | " +
    "\(percentile($values; 0.99)) | " +
    "\($values | max) |";
  "| metric | samples | avg_tick | p50 | p95 | p99 | max |\n" +
  "|---|---:|---:|---:|---:|---:|---:|\n" +
  row("ideal_drive_ticks"; map(.ideal_drive_ticks)) + "\n" +
  row("actual_drive_ticks"; map(.actual_drive_ticks)) + "\n" +
  row("excess_drive_ticks"; map(.excess_drive_ticks))
' "$benchmark_json"

printf '\n| completed rides | drive pass | drive fail | dissatisfaction |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  (length) as $total |
  (map(select(.drive_pass)) | length) as $passed |
  "| \($total) | \($passed) | \($total - $passed) | " +
  "\((($total - $passed) * 1000 / $total | floor) / 10)% |"
' "$benchmark_json"

printf '\ncorrelated ride sample\n\n'
printf 'Coordinate POSTs use picked_up_tick < world_tick < arrived_tick. The final POST after ArrivedAt is excluded because its latency cannot increase drive ticks.\n\n'
printf '| rides | drive fail | coordinate POSTs | failed POSTs | POST >=30ms | excess ticks | estimated blocked ticks |\n'
printf '|---:|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  "| \(length) | \(map(select(.drive_pass == false)) | length) | " +
  "\(map(.coordinate_requests) | add) | \(map(.coordinate_failed) | add) | " +
  "\(map(.coordinate_over_30ms) | add) | " +
  "\(map(.excess_drive_ticks) | add) | \(map(.estimated_blocked_ticks) | add) |"
' "$correlated_json"

printf '\nclient-observed coordinate POST latency during drive\n\n'
jq --slurp --raw-output '
  def percentile($values; $ratio):
    ($values | sort) as $sorted |
    $sorted[(($sorted | length) - 1) * $ratio | floor];
  def row($label; $values):
    "| \($label) | \($values | length) | " +
    "\(($values | add) / ($values | length) | floor) | " +
    "\(percentile($values; 0.50)) | \(percentile($values; 0.95)) | " +
    "\(percentile($values; 0.99)) | \($values | max) |";
  [.[].coordinate_client_durations_us[]] as $requests |
  "| metric | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n" +
  "|---|---:|---:|---:|---:|---:|---:|\n" +
  row("request"; $requests) + "\n" +
  row("per-ride coordinate average"; map(.coordinate_client_avg_us)) + "\n" +
  row("per-ride coordinate maximum"; map(.coordinate_client_max_us))
' "$correlated_json"

printf '\nserver coordinate phases inside the CARRYING window\n\n'
jq --slurp --raw-output '
  def percentile($values; $ratio):
    ($values | sort) as $sorted |
    $sorted[(($sorted | length) - 1) * $ratio | floor];
  def row($label; $values):
    "| \($label) | \($values | length) | " +
    "\(($values | add) / ($values | length) | floor) | " +
    "\(percentile($values; 0.50)) | \(percentile($values; 0.95)) | " +
    "\(percentile($values; 0.99)) | \($values | max) |";
  "| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n" +
  "|---|---:|---:|---:|---:|---:|---:|\n" +
  row("pool_acquire_us"; map(.pool_acquire_us)) + "\n" +
  row("transaction_begin_us"; map(.transaction_begin_us)) + "\n" +
  row("history_insert_us"; map(.history_insert_us)) + "\n" +
  row("current_write_us"; map(.current_write_us)) + "\n" +
  row("ride_lookup_us"; map(.ride_lookup_us)) + "\n" +
  row("commit_us"; map(.commit_us)) + "\n" +
  row("total_us"; map(.total_us))
' "$drive_coordinate_json"

printf '\nserver CARRYING status phases\n\n'
jq --slurp --raw-output '
  def percentile($values; $ratio):
    ($values | sort) as $sorted |
    $sorted[(($sorted | length) - 1) * $ratio | floor];
  def row($label; $values):
    "| \($label) | \($values | length) | " +
    "\(($values | add) / ($values | length) | floor) | " +
    "\(percentile($values; 0.50)) | \(percentile($values; 0.95)) | " +
    "\(percentile($values; 0.99)) | \($values | max) |";
  map(select(.outcome == "success")) as $successful |
  "| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n" +
  "|---|---:|---:|---:|---:|---:|---:|\n" +
  row("pool_acquire_us"; $successful | map(.pool_acquire_us)) + "\n" +
  row("transaction_begin_us"; $successful | map(.transaction_begin_us)) + "\n" +
  row("ride_lock_us"; $successful | map(.ride_lock_us)) + "\n" +
  row("status_write_us"; $successful | map(.status_write_us)) + "\n" +
  row("commit_us"; $successful | map(.commit_us)) + "\n" +
  row("total_us"; $successful | map(.total_us))
' "$ride_status_json"

printf '\nserver milestone gaps for the same ride\n\n'
printf 'Coordinate and CARRYING boundaries are successful DB commits. Notification boundaries are when the server handler finished building a successful poll response, not when the client received it.\n\n'
jq --slurp --raw-output '
  def percentile($values; $ratio):
    ($values | map(select(. != null)) | sort) as $sorted |
    if ($sorted | length) == 0 then "n/a"
    else $sorted[(($sorted | length) - 1) * $ratio | floor]
    end;
  def average($values):
    ($values | map(select(. != null))) as $present |
    if ($present | length) == 0 then "n/a"
    else ($present | add) / ($present | length) | floor
    end;
  def row($label; $values):
    ($values | map(select(. != null))) as $present |
    "| \($label) | \($present | length) | \(average($values)) | " +
    "\(percentile($values; 0.50)) | \(percentile($values; 0.95)) | " +
    "\(percentile($values; 0.99)) | " +
    "\(if ($present | length) == 0 then "n/a" else ($present | max) end) |";
  "| gap | rides | avg_ms | p50_ms | p95_ms | p99_ms | max_ms |\n" +
  "|---|---:|---:|---:|---:|---:|---:|\n" +
  row("PICKUP commit -> app response built"; map(.pickup_coordinate_to_app_ms)) + "\n" +
  row("PICKUP commit -> chair response built"; map(.pickup_coordinate_to_chair_ms)) + "\n" +
  row("app response built -> CARRYING commit"; map(.app_pickup_to_carrying_commit_ms)) + "\n" +
  row("CARRYING commit -> app response built"; map(.carrying_commit_to_app_ms)) + "\n" +
  row("CARRYING commit -> chair response built"; map(.carrying_commit_to_chair_ms)) + "\n" +
  row("CARRYING commit -> ARRIVED coordinate"; map(.carrying_commit_to_arrived_coordinate_ms)) + "\n" +
  row("ARRIVED commit -> app response built"; map(.arrived_coordinate_to_app_ms)) + "\n" +
  row("ARRIVED commit -> chair response built"; map(.arrived_coordinate_to_chair_ms))
' "$correlated_json"

printf '\ntraced diagnostic failures and cancellations\n\n'
printf '| source | terminal phase | samples |\n'
printf '|---|---|---:|\n'
jq -n --raw-output \
  --slurpfile client "$client_coordinate_json" \
  --slurpfile coordinate "$coordinate_json" \
  --slurpfile notification "$notification_json" \
  --slurpfile ride_status "$ride_status_json" '
  [
    ($client[] | select(.success == false) |
      {source: "client coordinate", terminal_phase: "HTTP attempt"}),
    ($coordinate[] | select(.outcome != "success") |
      {source: "coordinate", terminal_phase}),
    ($notification[] | select(.outcome != "success") |
      {source: ("notification/" + .endpoint), terminal_phase}),
    ($ride_status[] | select(.outcome != "success") |
      {source: "CARRYING status", terminal_phase})
  ] |
  group_by([.source, .terminal_phase]) |
  if length == 0 then
    "| all | none | 0 |"
  else
    .[] |
    "| \(.[0].source) | \(.[0].terminal_phase) | \(length) |"
  end
'

printf '\ndrive failures with the largest excess\n\n'
printf '| ride | ideal | actual | excess | POSTs | >=30ms | blocked tick estimate | max POST us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  map(select(.drive_pass == false)) |
  sort_by(-.excess_drive_ticks, -.coordinate_client_max_us) |
  .[:10][] |
  "| \(.ride_id) | \(.ideal_drive_ticks) | \(.actual_drive_ticks) | " +
  "\(.excess_drive_ticks) | \(.coordinate_requests) | " +
  "\(.coordinate_over_30ms) | \(.estimated_blocked_ticks) | " +
  "\(.coordinate_client_max_us) |"
' "$correlated_json"

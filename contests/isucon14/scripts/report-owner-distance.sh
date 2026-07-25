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

raw_web_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-web.XXXXXX")
owner_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-app.XXXXXX")
owner_chair_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-chair.XXXXXX")
coordinate_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-coordinate.XXXXXX")
benchmark_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-benchmark.XXXXXX")
correlated_json=$(mktemp "${TMPDIR:-/tmp}/isucon14-owner-distance-correlated.XXXXXX")

cleanup() {
  rm -f \
    "$raw_web_log" \
    "$owner_json" \
    "$owner_chair_json" \
    "$coordinate_json" \
    "$benchmark_json" \
    "$correlated_json"
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

sed -n 's/^.*OWNER_DISTANCE_DIAGNOSTIC //p' "$raw_web_log" >"$owner_json"
sed -n 's/^.*OWNER_DISTANCE_CHAIR_DIAGNOSTIC //p' "$raw_web_log" >"$owner_chair_json"
sed -n 's/^.*COORDINATE_DIAGNOSTIC //p' "$raw_web_log" >"$coordinate_json"
sed -n 's/^.*OWNER_DISTANCE_BENCHMARK_DIAGNOSTIC //p' "$benchmark_log" >"$benchmark_json"

if [ ! -s "$owner_json" ]; then
  echo "owner距離のwebapp診断sampleがありません。ISUCON_DIAGNOSTIC=1で実行してください。" >&2
  exit 1
fi
if [ ! -s "$owner_chair_json" ]; then
  echo "owner距離のchair別診断sampleがありません。" >&2
  exit 1
fi

printf 'owner distance request latency\n\n'
printf '| metric | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
for metric in query_us response_build_us total_us
do
  jq --slurp --raw-output --arg metric "$metric" '
    [.[].[$metric]] | sort as $values |
    ($values | length) as $length |
    [
      $metric,
      $length,
      (($values | add) / $length | floor),
      $values[(($length - 1) * 0.50 | floor)],
      $values[(($length - 1) * 0.95 | floor)],
      $values[(($length - 1) * 0.99 | floor)],
      $values[$length - 1]
    ] |
    "| \(.[0]) | \(.[1]) | \(.[2]) | \(.[3]) | \(.[4]) | \(.[5]) | \(.[6]) |"
  ' "$owner_json"
done

printf '\nwatermark suppression\n\n'
jq --slurp --raw-output '
  "requests=\(length) " +
  "chairs=\(map(.chair_count) | add) " +
  "suppressed=\(map(.timestamp_suppressed_count) | add) " +
  "requests_with_suppression=\(map(select(.timestamp_suppressed_count > 0)) | length)"
' "$owner_json"

printf '\ncoordinate recorded_at to commit window\n\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.recorded_to_commit_us != null) |
    .recorded_to_commit_us
  ] | sort as $values |
  ($values | length) as $length |
  if $length == 0 then
    "samples=0"
  else
    "samples=\($length) " +
    "p95_us=\($values[(($length - 1) * 0.95 | floor)]) " +
    "p99_us=\($values[(($length - 1) * 0.99 | floor)]) " +
    "max_us=\($values[$length - 1]) " +
    "over_1000ms=\([$values[] | select(. >= 1000000)] | length)"
  end
' "$coordinate_json"

printf '\nbenchmark mismatch correlation\n\n'
if [ ! -s "$benchmark_json" ]; then
  printf 'mismatches=0\n'
  exit 0
fi

jq -n --raw-output \
  --slurpfile owner_chair "$owner_chair_json" \
  --slurpfile coordinate "$coordinate_json" \
  --slurpfile benchmark "$benchmark_json" '
  def response_watermark_matches($request; $bench):
    (
      $request.chair.stable_updated_at_unix_us != null and
      (
        (($request.chair.stable_updated_at_unix_us / 1000) | floor) * 1000
        == $bench.response_watermark_unix_us
      )
    );
  $benchmark[] as $bench |
  (
    [
      $owner_chair[] |
      (.request_started_at_unix_us - $bench.request_started_at_unix_us) as $delta |
      select(
        .chair.chair_id == $bench.chair_id and
        $delta >= 0 and
        $delta <= 1000000 and
        .chair.total_distance == $bench.response_total_distance and
        response_watermark_matches(.; $bench)
      )
    ]
  ) as $requests |
  if ($requests | length) != 1 then
    error(
      "chair=\($bench.chair_id) の対応requestは1件である必要があります: " +
      "matches=\($requests | length)"
    )
  elif (
    $bench.initial_expected_distance != $bench.location.distance_at_watermark or
    $bench.initial_current_distance != $bench.location.current_distance
  ) then
    error("chair=\($bench.chair_id) のbenchmark診断snapshotが不整合です")
  else
  ($requests[0]) as $request |
  ($request.chair) as $server_chair |
  (
    [
      $coordinate[] |
      select(.location_id == $server_chair.latest_location_id)
    ] |
    sort_by(.committed_at_unix_us) |
    last
  ) as $latest_coordinate |
  {
    chair_id: $bench.chair_id,
    request_delta_us: (
      $request.request_started_at_unix_us - $bench.request_started_at_unix_us
    ),
    response_watermark_unix_us: $bench.response_watermark_unix_us,
    response_total_distance: $bench.response_total_distance,
    initial_expected_distance: $bench.initial_expected_distance,
    initial_current_distance: $bench.initial_current_distance,
    diagnostic_expected_distance: $bench.location.distance_at_watermark,
    diagnostic_current_distance: $bench.location.current_distance,
    unknown_server_times: $bench.location.unknown_server_times,
    server_snapshot_at_unix_us: $request.distance_snapshot_at_unix_us,
    server_stable_updated_at_unix_us: $server_chair.stable_updated_at_unix_us,
    server_latest_location_id: $server_chair.latest_location_id,
    server_latest_created_at_unix_us: $server_chair.latest_location_created_at_unix_us,
    server_stable_location_count: $server_chair.stable_location_count,
    timestamp_suppressed: $server_chair.timestamp_suppressed,
    latest_coordinate_recorded_to_commit_us: $latest_coordinate.recorded_to_commit_us,
    latest_coordinate_commit_unix_us: $latest_coordinate.committed_at_unix_us
  } |
  @json
  end
' >"$correlated_json"

cat "$correlated_json"
benchmark_count=$(jq --slurp 'length' "$benchmark_json")
correlated_count=$(jq --slurp 'length' "$correlated_json")
if [ "$benchmark_count" -ne "$correlated_count" ]; then
  echo "benchmarkと相関結果の件数が一致しません: benchmark=$benchmark_count correlated=$correlated_count" >&2
  exit 1
fi
printf 'mismatches=%s\n' "$correlated_count"

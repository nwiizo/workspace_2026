#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_PID=""
NODE1_PID=""
NODE2_PID=""
NODE3_PID=""

PEERS="1=http://127.0.0.1:9080,2=http://127.0.0.1:9081,3=http://127.0.0.1:9082"

cleanup() {
    local pid

    for pid in "$NODE1_PID" "$NODE2_PID" "$NODE3_PID" "$UPSTREAM_PID"; do
        if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
            kill "${pid}" 2>/dev/null || true
        fi
    done

    for pid in "$NODE1_PID" "$NODE2_PID" "$NODE3_PID" "$UPSTREAM_PID"; do
        if [[ -n "${pid}" ]]; then
            wait "${pid}" 2>/dev/null || true
        fi
    done

    echo "Logs:"
    echo "  /tmp/raft-proxy-1.log"
    echo "  /tmp/raft-proxy-2.log"
    echo "  /tmp/raft-proxy-3.log"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

banner() {
    echo "== $* =="
}

node_pid() {
    case "$1" in
        1) printf '%s\n' "$NODE1_PID" ;;
        2) printf '%s\n' "$NODE2_PID" ;;
        3) printf '%s\n' "$NODE3_PID" ;;
        *) fail "unknown node id $1" ;;
    esac
}

admin_port() {
    case "$1" in
        1) printf '%s\n' "9080" ;;
        2) printf '%s\n' "9081" ;;
        3) printf '%s\n' "9082" ;;
        *) fail "unknown node id $1" ;;
    esac
}

proxy_port() {
    case "$1" in
        1) printf '%s\n' "8080" ;;
        2) printf '%s\n' "8081" ;;
        3) printf '%s\n' "8082" ;;
        *) fail "unknown node id $1" ;;
    esac
}

base_url() {
    printf 'http://127.0.0.1:%s\n' "$(admin_port "$1")"
}

metrics_json() {
    curl -fsS "$(base_url "$1")/cluster/metrics"
}

json_current_leader() {
    python3 -c 'import json,sys; value=json.load(sys.stdin).get("current_leader"); print("" if value is None else value)'
}

wait_for_admin() {
    local node="$1"
    local deadline=$(( $(date +%s) + 5 ))
    local pid

    pid="$(node_pid "$node")"
    while [[ "$(date +%s)" -le "$deadline" ]]; do
        if ! kill -0 "$pid" 2>/dev/null; then
            fail "node $node exited before admin listened; see /tmp/raft-proxy-$node.log"
        fi

        if curl -fsS "$(base_url "$node")/cluster/metrics" >/dev/null 2>&1; then
            echo "node $node admin listening on $(base_url "$node")"
            return
        fi

        sleep 0.1
    done

    fail "timed out waiting for node $node admin on $(base_url "$node")"
}

wait_for_leader_from_nodes() {
    local max_wait="$1"
    shift
    local deadline=$(( $(date +%s) + max_wait ))
    local node
    local metrics
    local leader

    while [[ "$(date +%s)" -le "$deadline" ]]; do
        for node in "$@"; do
            metrics="$(metrics_json "$node" 2>/dev/null || true)"
            if [[ -z "$metrics" ]]; then
                continue
            fi

            leader="$(printf '%s' "$metrics" | json_current_leader)"
            if [[ -n "$leader" ]]; then
                printf '%s\n' "$leader"
                return
            fi
        done

        sleep 0.1
    done

    fail "timed out waiting for leader from nodes: $*"
}

wait_for_new_leader() {
    local old_leader="$1"
    local max_wait="$2"
    shift 2
    local deadline=$(( $(date +%s) + max_wait ))
    local node
    local metrics
    local leader
    local survivor

    while [[ "$(date +%s)" -le "$deadline" ]]; do
        for node in "$@"; do
            metrics="$(metrics_json "$node" 2>/dev/null || true)"
            if [[ -z "$metrics" ]]; then
                continue
            fi

            leader="$(printf '%s' "$metrics" | json_current_leader)"
            if [[ -z "$leader" || "$leader" == "$old_leader" ]]; then
                continue
            fi

            for survivor in "$@"; do
                if [[ "$leader" == "$survivor" ]]; then
                    printf '%s\n' "$leader"
                    return
                fi
            done
        done

        sleep 0.1
    done

    fail "timed out waiting for new leader after killing node $old_leader"
}

put_route() {
    local node="$1"
    local host="$2"
    local response

    response="$(
        curl -fsS -X PUT "$(base_url "$node")/admin/routes" \
            -H 'Content-Type: application/json' \
            --data "{\"host\":\"$host\",\"upstreams\":[{\"addr\":\"127.0.0.1:19001\",\"weight\":1}]}"
    )"

    if ! [[ "$response" == *'"status":"ok"'* || "$response" == *'"status": "ok"'* ]]; then
        fail "PUT route for $host via node $node returned unexpected response: $response"
    fi
}

expect_proxy_body() {
    local host="$1"
    local port="$2"
    local body

    body="$(curl -fsS -H "Host: $host" "http://127.0.0.1:$port/")"
    if ! [[ "$body" == "hello from upstream" ]]; then
        fail "proxy $port for host $host returned unexpected body: $body"
    fi

    echo "verified host=$host via proxy port $port"
}

banner "STEP 1: build release binary"
cargo build --release -p raft-proxy

banner "STEP 2: start upstream"
python3 - <<'PY' &
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        body = b"hello from upstream\n"
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *a, **kw): pass
HTTPServer(("127.0.0.1", 19001), H).serve_forever()
PY
UPSTREAM_PID=$!

for _ in 1 2 3 4 5 6 7 8 9 10; do
    if curl -fsS "http://127.0.0.1:19001/" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done
if ! curl -fsS "http://127.0.0.1:19001/" >/dev/null 2>&1; then
    fail "upstream did not start on 127.0.0.1:19001"
fi
echo "upstream listening on http://127.0.0.1:19001"

banner "STEP 3: start raft-proxy nodes"
rm -f /tmp/raft-proxy-1.log /tmp/raft-proxy-2.log /tmp/raft-proxy-3.log

./target/release/raft-proxy --id 1 --proxy-addr 127.0.0.1:8080 --admin-addr 127.0.0.1:9080 --peers "$PEERS" >/tmp/raft-proxy-1.log 2>&1 &
NODE1_PID=$!
wait_for_admin 1

./target/release/raft-proxy --id 2 --proxy-addr 127.0.0.1:8081 --admin-addr 127.0.0.1:9081 --peers "$PEERS" >/tmp/raft-proxy-2.log 2>&1 &
NODE2_PID=$!
wait_for_admin 2

./target/release/raft-proxy --id 3 --proxy-addr 127.0.0.1:8082 --admin-addr 127.0.0.1:9082 --peers "$PEERS" >/tmp/raft-proxy-3.log 2>&1 &
NODE3_PID=$!
wait_for_admin 3

banner "STEP 4: bootstrap cluster"
curl -fsS -X POST "$(base_url 1)/cluster/init" \
    -H 'Content-Type: application/json' \
    --data '{"members":[{"id":1,"rpc_addr":"http://127.0.0.1:9080"},{"id":2,"rpc_addr":"http://127.0.0.1:9081"},{"id":3,"rpc_addr":"http://127.0.0.1:9082"}]}'
echo

banner "STEP 5: wait for leader"
LEADER="$(wait_for_leader_from_nodes 10 1 2 3)"
echo "leader is node $LEADER"

banner "STEP 6: add example.test route"
put_route "$LEADER" "example.test"
sleep 0.5

banner "STEP 7: verify example.test on all proxies"
expect_proxy_body "example.test" 8080
expect_proxy_body "example.test" 8081
expect_proxy_body "example.test" 8082

banner "STEP 8: kill leader"
LEADER_PID="$(node_pid "$LEADER")"
kill "$LEADER_PID"
wait "$LEADER_PID" 2>/dev/null || true
echo "killed leader node $LEADER"

SURVIVORS=()
for node in 1 2 3; do
    if [[ "$node" != "$LEADER" ]]; then
        SURVIVORS+=("$node")
    fi
done

NEW_LEADER="$(wait_for_new_leader "$LEADER" 5 "${SURVIVORS[@]}")"
echo "new leader is node $NEW_LEADER"

banner "STEP 9: add beta.test route after failover"
put_route "$NEW_LEADER" "beta.test"
sleep 0.5

banner "STEP 10: verify both routes on surviving proxies"
for node in "${SURVIVORS[@]}"; do
    port="$(proxy_port "$node")"
    expect_proxy_body "example.test" "$port"
    expect_proxy_body "beta.test" "$port"
done

banner "STEP 11: PASS summary"
echo "PASS: raft-proxy e2e completed; leader node $LEADER killed, new leader node $NEW_LEADER served replicated routes on surviving proxies"

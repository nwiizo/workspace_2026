# raft-proxy

`raft-proxy` is a Rust + openraft + pingora distributed HTTP proxy sample that will replicate routing table changes through a Raft control plane and later hot-swap those routes into the proxy data plane. Future steps will add storage, networking, control APIs, pingora proxying, and three-node verification; the implementation plan is tracked at `/Users/nwiizo/.claude/plans/rust-raft-snoopy-lovelace.md`.

Status: working PoC - all tests + clippy + e2e green

## How to run

Start three nodes locally, one process per node:

```sh
# terminal A
cargo run -p raft-proxy -- --id 1 --proxy-addr 127.0.0.1:8080 --admin-addr 127.0.0.1:9080 \
  --peers 1=http://127.0.0.1:9080,2=http://127.0.0.1:9081,3=http://127.0.0.1:9082

# terminal B
cargo run -p raft-proxy -- --id 2 --proxy-addr 127.0.0.1:8081 --admin-addr 127.0.0.1:9081 \
  --peers 1=http://127.0.0.1:9080,2=http://127.0.0.1:9081,3=http://127.0.0.1:9082

# terminal C
cargo run -p raft-proxy -- --id 3 --proxy-addr 127.0.0.1:8082 --admin-addr 127.0.0.1:9082 \
  --peers 1=http://127.0.0.1:9080,2=http://127.0.0.1:9081,3=http://127.0.0.1:9082
```

Bootstrap the cluster:

```sh
curl -X POST http://127.0.0.1:9080/cluster/init \
  -H 'Content-Type: application/json' \
  -d '{"members":[
        {"id":1,"rpc_addr":"http://127.0.0.1:9080"},
        {"id":2,"rpc_addr":"http://127.0.0.1:9081"},
        {"id":3,"rpc_addr":"http://127.0.0.1:9082"}
      ]}'
```

Put a route:

```sh
curl -X PUT http://127.0.0.1:9080/admin/routes \
  -H 'Content-Type: application/json' \
  -d '{"host":"example.test","upstreams":[{"addr":"127.0.0.1:19001","weight":1}]}'
```

Writes sent to a follower's admin API return `307 Temporary Redirect` with
`Location: <leader base url>/admin/routes`. Clients must follow cross-method
redirects with the body preserved, for example `curl -L --post301 --post302
--post303 ...` or `reqwest::redirect::Policy::limited(N)`.

`GET /admin/routes` reads the contacted node's local `ArcSwap` route table: the
committed state visible on that node. It is not a linearizable read, which is
acceptable for this learning sample but important when comparing nodes.

Proxy through any node:

```sh
curl -H 'Host: example.test' http://127.0.0.1:8082/
```

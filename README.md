# Node Health

Sidecar that monitors an Ethereum execution + consensus node pair and exposes Kubernetes-style health probes. Designed to run as a third container alongside an execution node and a consensus (beacon) node in the same pod.

## Health checks

Five checks run every 4 seconds. The node is **ready** only when all pass.

| # | Layer | Check | Pass condition |
|---|-------|-------|----------------|
| 1 | Execution | `eth_syncing` | Returns `false` (not syncing) |
| 2 | Execution | `eth_getBlockByNumber("latest")` | Block age < 48 s |
| 3 | Execution | `net_peerCount` | Peers >= 5 (mainnet) / 2 (testnet) |
| 4 | Consensus | `GET /eth/v1/node/health` | Returns `200 OK` |
| 5 | Consensus | `GET /eth/v1/node/peer_count` | Connected peers >= 10 (mainnet) / 5 (testnet) |

## Endpoints

| Path | Purpose | Response |
|------|---------|----------|
| `GET /livez` | Liveness probe | Always `200 OK` |
| `GET /readyz` | Readiness probe | `200 OK` when healthy, `503` otherwise |

## Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `EXECUTION_NODE_URL` | yes | — | JSON-RPC endpoint (e.g. `http://localhost:8545`) |
| `BEACON_URL` | yes | — | Beacon API endpoint (e.g. `http://localhost:5052`) |
| `NETWORK` | no | `mainnet` | `mainnet`, `holesky`, or `hoodi` — controls min peer thresholds |
| `PORT` | no | `3004` | HTTP server port |
| `BIND_PUBLIC_INTERFACE` | no | `true` | Bind `0.0.0.0` (`true`) or `127.0.0.1` (`false`) |

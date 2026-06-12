# Development Guide

## Performance Testing

### Prerequisites

Install hey (HTTP load generator) and samply (profiler):

```bash
brew install hey
cargo install samply
```

### Benchmark

```bash
# 1. Build release binary
make build

# 2. Start server
./target/dx/yggdrasil/release/web/server

# 3. Run load test (in another terminal)
hey -c 100 -n 100000 http://localhost:8080/
```

### Flame Graph

```bash
# 1. Build with debug symbols (required for readable flame graphs)
CARGO_PROFILE_RELEASE_DEBUG=1 make build

# Terminal 1: Start profiling
samply record -- ./target/dx/yggdrasil/release/web/server

# Terminal 2: Wait for server to start, then send load
hey -c 100 -n 100000 http://localhost:8080/

# Terminal 1: Ctrl+C after hey finishes — samply opens flame graph in browser
```

### Key Metrics to Watch

| Metric | Description |
|--------|-------------|
| Requests/sec | Throughput |
| Average latency | Mean response time |
| P99 latency | Tail latency |
| Status codes | Error rate (should be 0) |
| Latency distribution | Consistency (tight = stable) |

### Flame Graph Hotspots

| Expected Hotspot | Code Location | Cause |
|------------------|---------------|-------|
| SSR rendering | Dioxus framework | Virtual DOM diff + render per request |
| `deadpool` connection acquisition | `src/db/mod.rs` | Connection pool contention under concurrency |
| `moka` cache lookup | `src/cache.rs` | Cache hit/miss overhead |
| `tokio` scheduling | tokio runtime | Async task dispatch |
| `serde` serialization | Models | Post/User serialization |

### Tuning

- `DB_POOL_SIZE` — increase if `deadpool` / `Semaphore` shows high in flame graph
- `SSR_CACHE_SECS` — increase to cache SSR output longer
- `TOKIO_WORKER_THREADS` — explicitly set worker thread count

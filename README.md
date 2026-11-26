# fry

Embedded database engine for **nanovm agents**: durable key/value memory plus an agent inbox, single-writer, WAL-backed.

## Design

- **Library first** — `fry::FryDb::open(path)` from Rust (or other hosts via FFI later).
- **Append-only WAL** — `wal.ndjson` (one JSON object per line); state rebuilds on open.
- **Exclusive lock** — `.fry.lock` so only one process writes at a time (fine for nanovm cron agents).
- **CLI** — `fry` binary for shell/Python agents without linking Rust.

Default guest path (suggested): `/var/lib/nanovm/fry`

## Build

```bash
cd /Users/sunny/fry
make build          # macOS dev binary: target/release/fry
make test
```

### musl build (Alpine nanovm guest)

```bash
make build-alpine-docker   # recommended on macOS: rust:alpine → target/release/fry (static musl)
# or, with a cross linker installed:
# CARGO_TARGET_..._LINKER=aarch64-linux-musl-gcc make build-musl
make deploy-nanovm         # or: cd ../nanovm && make deploy-fry
```

## CLI

```bash
export FRY_DATA_DIR=/var/lib/nanovm/fry

fry put global deploy-notes "last deploy ok"
fry get global deploy-notes

fry send news principal-dev "headlines ready"
fry inbox principal-dev
fry ack principal-dev <message-uuid>
```

Namespaces are arbitrary strings (`global`, `agent:news`, `project:tradev`, …).

## Rust API

```rust
use fry::FryDb;

let mut db = FryDb::open("/var/lib/nanovm/fry")?;
db.put("global", "tz", b"America/Chicago")?;
let id = db.inbox_send("news", "*", b"digest")?;
```

## nanovm

`make deploy-nanovm` from this repo, or `make deploy-fry` from `../nanovm` (VM must be up). Agents: `source /opt/nanovm/agents/common/fry.sh`.

## Not yet

- Snapshots / WAL compaction
- TTL expiry
- Full-text search
- Concurrent readers while writer is open (lock is process-wide today)
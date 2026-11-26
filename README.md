# fry

**fry** is a small embedded store for **multi-agent systems**: durable key/value memory plus a message inbox. Agents identify themselves with plain string IDs (`researcher`, `ops-bot`, …) and coordinate through a shared on-disk directory.

Repository: **https://github.com/somayaj/fry**

## Why fry

| Need | fry feature |
|------|-------------|
| Shared facts between jobs | Namespaced **KV** (`put` / `get` / `scan`) |
| “Hey, other agent” messages | **Inbox** (`send` / `inbox` / `ack`) |
| Crash-safe, no server | Append-only **WAL** (`wal.ndjson`) |
| Simple deployment | One binary + one data folder |

## Install

```bash
git clone https://github.com/somayaj/fry.git
cd fry
cargo build --release
# binary: target/release/fry
```

Linux **musl** (e.g. Alpine) from macOS:

```bash
make build-alpine-docker
```

## Quick start

Default data directory (when `FRY_DATA_DIR` is unset):

- `$XDG_DATA_HOME/fry`, or
- `~/.local/share/fry`

Override:

```bash
export FRY_DATA_DIR=/var/lib/my-fleet/fry
```

### Shared memory (KV)

```bash
fry put shared deploy-status "staging verified"
fry get shared deploy-status
fry scan agent researcher-
```

Namespaces are free-form (`shared`, `agent:writer`, `session:abc`, …).

### Agent-to-agent (inbox)

```bash
fry send researcher writer "three citations ready"
fry send ops-monitor '*' "disk 90% on host-3"
fry inbox writer
fry ack writer <message-uuid>
```

- **`from` / `to`** — agent IDs you choose; must match what each process uses in `inbox` / `ack`.
- **`*`** — broadcast; every agent’s inbox poll also sees messages addressed to `*`.

### Inbox output

Tab-separated: `id`, `new|read`, `from`, `to`, `body`.

## Rust library

```rust
use fry::FryDb;

let mut db = FryDb::open("/path/to/fry-data")?;
db.put("shared", "checkpoint", b"epoch-12")?;
let id = db.inbox_send("collector", "trainer", b"batch ready")?;
```

## Agent integration patterns

1. **Bulletin board** — writer `put`s facts; readers `get` on their schedule (no receiver required online).
2. **Ping** — sender `send`s; receiver `inbox` + `ack` at start of each run.
3. **Fleet layout** — one `FRY_DATA_DIR` on a host (or NFS volume); all agents use the same path.

## Limitations

- **Single writer at a time** (`.fry.lock`). Stagger jobs or retry on `database locked`.
- **Not a network database** — agents must share the filesystem (or sync the directory yourself).
- **No encryption** — do not store secrets; use your platform’s secret store.

## License

MIT — see [LICENSE](LICENSE).

## Development

```bash
cargo test
fry --help
```
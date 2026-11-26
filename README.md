# fry

**fry** is a small embedded store for **multi-agent systems**: durable key/value memory plus a message inbox. Agents use string IDs and a **shared data directory** on disk — no database server.

**https://github.com/somayaj/fry**

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | **Start, stop, terminate** — lifecycle, install, reset, locking |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | WAL, replay, components, diagrams |
| [docs/SPEC.md](docs/SPEC.md) | On-disk format, API/CLI spec, **footprint** |
| [docs/RUST.md](docs/RUST.md) | Step-by-step Rust integration |

## TL;DR

```bash
cargo build --release
export FRY_DATA_DIR=~/.local/share/fry
fry put shared hello world
fry get shared hello
fry send researcher writer "done"
fry inbox writer
```

There is nothing to `systemctl start` — each `fry` command or `FryDb::open` **starts** a session; exiting **stops** it and releases the lock.

## Why fry

| Need | Feature |
|------|---------|
| Shared facts | Namespaced KV |
| Agent messages | Inbox + broadcast `*` |
| No server | Embedded library + CLI |
| Crash-safe writes | WAL + `fsync` per mutation |

## Limitations

Single writer per data directory; shared filesystem required; WAL does not compact in v0.1. See [docs/SPEC.md](docs/SPEC.md).

## License

MIT — [LICENSE](LICENSE)
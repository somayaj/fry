# fry specification (v0.1)

## On-disk layout

Data directory `D` (your `FRY_DATA_DIR` or Rust `open` path):

| File | Purpose |
|------|---------|
| `D/wal.ndjson` | Append-only log; UTF-8; one JSON object per line |
| `D/.fry.lock` | Empty or arbitrary content; used only for `flock(LOCK_EX \| LOCK_NB)` |

No other files are required. Missing `wal.ndjson` ⇒ empty database on first write.

## WAL record schema

Each line is a JSON object with field `"op"` (snake_case variant tag). Timestamps are `ts_millis` (Unix ms, u64).

### `put`

```json
{"op":"put","ns":"shared","key":"status","value":"<base64>","ts_millis":1730000000000}
```

### `delete`

```json
{"op":"delete","ns":"shared","key":"status","ts_millis":1730000000000}
```

### `inbox_put`

```json
{"op":"inbox_put","id":"550e8400-e29b-41d4-a716-446655440000","from":"researcher","to":"writer","body":"<base64>","ts_millis":1730000000000}
```

### `inbox_ack`

```json
{"op":"inbox_ack","id":"550e8400-e29b-41d4-a716-446655440000","agent":"writer","ts_millis":1730000000000}
```

Binary fields (`value`, `body`) use standard base64 in JSON.

### Replay rules

- Records applied in file order.
- `put` upserts `(ns, key)`.
- `delete` removes `(ns, key)` if present.
- `inbox_put` adds message; duplicate `id` on replay: later puts overwrite index state for that id.
- `inbox_ack` marks message read; acked messages stay in the log.

## API surface (Rust)

| Method | Mutates | Notes |
|--------|---------|-------|
| `FryDb::open(path)` | — | Creates dir, lock, replay |
| `data_dir()` | — | Path reference |
| `put(ns, key, value)` | yes | Value bytes arbitrary |
| `get(ns, key)` | no | `Option<&[u8]>` borrows from db |
| `delete(ns, key)` | yes | Returns `false` if absent |
| `scan_prefix(ns, prefix)` | no | Iterator over keys with prefix in `ns` |
| `inbox_send(from, to, body)` | yes | Returns new `Uuid` |
| `inbox_list(agent, unread_only)` | no | Sorted by `ts_millis` |
| `inbox_ack(agent, id)` | yes | `FryError::InboxNotFound` if unknown id |

Strings (`ns`, `key`, agent ids) are UTF-8 in practice; stored as Rust `String`.

## CLI specification

| Command | Args | Exit codes |
|---------|------|------------|
| `put` | `<ns> <key> [value]` | 0 ok, 1 io/lock, 2 usage |
| `get` | `<ns> <key>` | 0 ok, **3** missing key |
| `del` | `<ns> <key>` | 0 ok |
| `scan` | `<ns> [prefix]` | 0 ok; tab-separated `key\tvalue` |
| `send` | `<from> <to> [body]` | 0 + prints message uuid |
| `inbox` | `<agent> [--all]` | 0; TSV lines |
| `ack` | `<agent> <uuid>` | 0 ok |

Global: `--data <dir>`, `--help`.

## Error types

| `FryError` | When |
|------------|------|
| `Locked` | Another process holds `.fry.lock` |
| `CorruptWal(line)` | Invalid JSON on replay |
| `InboxNotFound` | Ack unknown id |
| `Io` | Filesystem errors |
| `Json` | Serialize/parse (rare on write path) |

## Limits (v0.1)

| Topic | Limit |
|-------|-------|
| Max value size | Practical limit = available RAM + disk; no hard cap in code |
| Writers | 1 per data directory |
| Readers in other processes | Blocked while writer holds lock |
| WAL growth | Unbounded (no compaction) |
| Network | None |
| Encryption | None |

## Footprint

Measured on **aarch64 Linux musl** (Alpine `rust:alpine` build, Nov 2025):

| Artifact | Approximate size |
|----------|------------------|
| `fry` CLI (release, static musl) | **~0.9 MB** |
| `libfry.rlib` (release) | **~0.8 MB** |

Runtime (order of magnitude):

| Resource | Behavior |
|----------|----------|
| **RAM** | All KV values + all inbox bodies loaded at open; grows with total stored bytes and key count |
| **Disk** | Sum of WAL line lengths (monotonic append) |
| **CPU open** | O(number of WAL lines) replay |
| **CPU write** | O(1) append + fsync per operation |

Dependencies (Rust crate, compiled in): `serde`, `serde_json`, `uuid`, `base64`, `fs2`, `thiserror`.

No runtime threads inside fry; no external services.
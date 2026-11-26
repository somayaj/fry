# Operations: start, stop, terminate

fry does **not** run as a system service. Lifecycle is **per process** — usually one short-lived agent job or one CLI invocation.

## Start

### CLI

Any command opens the database for the duration of that command:

```bash
export FRY_DATA_DIR=/var/lib/my-fleet/fry   # optional
fry put shared status ok
```

Default data directory if `FRY_DATA_DIR` unset:

| Priority | Path |
|----------|------|
| 1 | `--data <dir>` on the command line |
| 2 | `FRY_DATA_DIR` |
| 3 | `$XDG_DATA_HOME/fry` |
| 4 | `~/.local/share/fry` |
| 5 | `./.fry` (no `HOME`) |

On success the process holds `.fry.lock` until it exits.

### Rust

```rust
use fry::FryDb;

let mut db = FryDb::open("/path/to/data")?;  // start + lock + replay
// ... use db ...
// drop(db) or end of scope → stop (unlock)
```

### Prerequisites

- Data directory on a **shared local filesystem** visible to all agents (local disk, bind mount, NFS — same path on every host that participates).
- Writable by the agent user.
- Only **one** process should have `FryDb` open (or run `fry` CLI) at a time on that directory.

### Build / install (host)

```bash
git clone https://github.com/somayaj/fry.git
cd fry
cargo build --release
sudo install -m 0755 target/release/fry /usr/local/bin/fry
```

Linux static binary from macOS:

```bash
make build-alpine-docker
sudo install -m 0755 target/release/fry /usr/local/bin/fry
```

## Stop

**Normal stop** = process exits or `FryDb` is dropped.

- Exclusive lock on `.fry.lock` is released.
- WAL file remains; all data is already durable from completed writes.
- Next agent run **replays** the WAL on open (no separate shutdown step).

```rust
{
    let mut db = FryDb::open(path)?;
    db.put("shared", "heartbeat", b"ok")?;
} // stop here — lock released
```

CLI: when the `fry` command finishes, stop is implicit.

### If stop fails (kill -9)

- In-flight write may be lost if killed mid-append (partial last line).
- On next open, replay uses complete lines only; a truncated final line may cause `CorruptWal` — trim or repair that line manually (see terminate/reset).

## Terminate

“Terminate” means **retire the store or reset data**, not kill a server.

| Goal | Action |
|------|--------|
| **Retire fleet store** | Stop all agents; archive `wal.ndjson`; remove directory |
| **Hard reset (empty DB)** | No agents running; `rm -rf "$FRY_DATA_DIR"` then recreate dir on next start |
| **Remove lock after crash** | Ensure no `fry` process alive (`pgrep -x fry`); delete stale `.fry.lock` only if sure no holder |
| **Rotate / archive** | `mv wal.ndjson wal.ndjson.bak.$(date +%Y%m%d)` — **not supported** by engine yet (no snapshot); full reset or restore from backup only |

**Do not** delete `wal.ndjson` while another process holds the lock.

### Health checks

```bash
# Can we open? (will take the lock briefly)
fry get shared probe 2>/dev/null || echo "missing key or locked"

# WAL size
wc -c "$FRY_DATA_DIR/wal.ndjson"

# Who holds the lock (Linux)
fuser -v "$FRY_DATA_DIR/.fry.lock" 2>/dev/null || true
```

## Concurrency playbook for agents

1. **Cron / scheduled jobs** — offset schedules so two agents rarely overlap.
2. **Retry on lock** — exit code `1` and stderr `database locked`; sleep 1–5s and retry (bash loop).
3. **Long-lived worker** — keep one process holding `FryDb`; others use CLI against a different design (not supported in v0.1 — use one writer only).

Example retry (shell):

```bash
for i in 1 2 3 4 5; do
  fry put shared last-run "ok" && break
  sleep 2
done
```
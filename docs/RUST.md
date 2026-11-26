# Using fry from Rust

## 1. Add the dependency

**Path / git** (until crates.io publish):

```toml
[dependencies]
fry = { path = "../fry/fry" }
# or: fry = { git = "https://github.com/somayaj/fry" }
```

## 2. Open, use, close (start / stop)

```rust
use fry::{FryDb, FryError};

fn main() -> Result<(), FryError> {
    let data = std::env::var("FRY_DATA_DIR")
        .unwrap_or_else(|_| "/var/lib/my-fleet/fry".into());

  // START: lock + replay WAL
    let mut db = FryDb::open(&data)?;

    db.put("shared", "epoch", b"42")?;

    if let Some(v) = db.get("shared", "epoch") {
        println!("epoch = {}", String::from_utf8_lossy(v));
    }

    for (key, val) in db.scan_prefix("agent", "researcher-") {
        println!("{key}: {}", String::from_utf8_lossy(val));
    }

  // STOP: implicit on drop
    Ok(())
}
```

Use `?` on `FryError`; match `FryError::Locked` for retry logic in multi-agent hosts.

## 3. Inbox workflow

```rust
use fry::{FryDb, InboxMessage};
use uuid::Uuid;

fn run_writer(db: &mut FryDb) -> Result<(), fry::FryError> {
    let id = db.inbox_send("collector", "trainer", b"batch 7 ready")?;
    db.put("shared", "last_batch", b"7")?;
    println!("sent {id}");
    Ok(())
}

fn run_reader(db: &mut FryDb, agent: &str) -> Result<(), fry::FryError> {
    let msgs: Vec<InboxMessage> = db.inbox_list(agent, true);
    for m in msgs {
        println!("from {}: {}", m.from, String::from_utf8_lossy(&m.body));
        db.inbox_ack(agent, m.id)?;
    }
    Ok(())
}
```

## 4. Handling `Locked` (another agent is writing)

```rust
use fry::{FryDb, FryError};
use std::{thread, time::Duration};

fn open_with_retry(path: &str, attempts: u32) -> Result<FryDb, FryError> {
    for i in 0..attempts {
        match FryDb::open(path) {
            Ok(db) => return Ok(db),
            Err(FryError::Locked) if i + 1 < attempts => {
                thread::sleep(Duration::from_secs(2));
            }
            Err(e) => return Err(e),
        }
    }
    Err(FryError::Locked)
}
```

## 5. Agent id convention (your code)

fry does not assign agent identities. Pick stable strings and use them consistently:

```rust
const AGENT: &str = "researcher";

db.inbox_send(AGENT, "writer", b"...")?;
let pending = db.inbox_list(AGENT, true);
```

## 6. Embed in a long-running binary

v0.1 expects **one** `FryDb` per data directory per process. Pattern:

- Open once at startup.
- Serve your agent loop.
- Drop on shutdown.

Do not open two `FryDb` on the same path in one process (second will fail with `Locked` — same lock file).

## 7. Tests

Use a temp directory per test:

```rust
let dir = tempfile::tempdir().unwrap();
let mut db = FryDb::open(dir.path())?;
```

See `fry/src/lib.rs` `mod tests` for examples.

## 8. Public types

| Type | Use |
|------|-----|
| `FryDb` | Main handle |
| `FryError` / `Result` | Errors |
| `InboxMessage` | `id`, `from`, `to`, `body`, `ts_millis`, `read` |
| `WalRecord` | Advanced / debugging WAL (optional) |
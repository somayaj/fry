//! **fry** — embedded, log-structured KV + agent inbox.
//!
//! Single-writer, append-only WAL (`wal.ndjson`), in-memory indexes rebuilt on open.
//! Multiple autonomous agents (CLI jobs, bots, workers) can share one data directory.

mod error;
mod inbox;
mod record;
mod wal;

pub use error::{FryError, Result};
pub use inbox::InboxMessage;
pub use record::WalRecord;

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use uuid::Uuid;

use crate::inbox::InboxIndex;
use crate::wal::Wal;

/// `(namespace, key) -> value`
type KvStore = BTreeMap<(String, String), Vec<u8>>;

pub struct FryDb {
    dir: PathBuf,
    _lock: std::fs::File,
    kv: KvStore,
    inbox: InboxIndex,
    wal: Wal,
}

impl FryDb {
    /// Open or create a database directory. Holds an exclusive lock for the process lifetime.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let lock_path = dir.join(".fry.lock");
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        lock.try_lock_exclusive().map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                FryError::Locked
            } else {
                FryError::Io(e)
            }
        })?;

        let mut kv = KvStore::new();
        let mut inbox = InboxIndex::default();
        for rec in Wal::replay(&dir)? {
            apply_record(&mut kv, &mut inbox, &rec);
        }

        let wal = Wal::open(&dir)?;

        Ok(Self {
            dir,
            _lock: lock,
            kv,
            inbox,
            wal,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    pub fn put(&mut self, ns: &str, key: &str, value: &[u8]) -> Result<()> {
        let rec = WalRecord::Put {
            ns: ns.to_string(),
            key: key.to_string(),
            value: value.to_vec(),
            ts_millis: now_millis(),
        };
        self.wal.append(&rec)?;
        apply_record(&mut self.kv, &mut self.inbox, &rec);
        Ok(())
    }

    pub fn get(&self, ns: &str, key: &str) -> Option<&[u8]> {
        self.kv.get(&(ns.to_string(), key.to_string())).map(|v| v.as_slice())
    }

    /// Delete a key if present. Returns whether a key was removed.
    pub fn delete(&mut self, ns: &str, key: &str) -> Result<bool> {
        let present = self.kv.contains_key(&(ns.to_string(), key.to_string()));
        if !present {
            return Ok(false);
        }
        let rec = WalRecord::Delete {
            ns: ns.to_string(),
            key: key.to_string(),
            ts_millis: now_millis(),
        };
        self.wal.append(&rec)?;
        apply_record(&mut self.kv, &mut self.inbox, &rec);
        Ok(true)
    }

    /// Keys in `ns` whose key string starts with `prefix`.
    pub fn scan_prefix<'a>(
        &'a self,
        ns: &'a str,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a [u8])> + 'a {
        self.kv
            .iter()
            .filter(move |((n, k), _)| n == ns && k.starts_with(prefix))
            .map(|((_, k), v)| (k.as_str(), v.as_slice()))
    }

    pub fn inbox_send(&mut self, from: &str, to: &str, body: &[u8]) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let rec = WalRecord::InboxPut {
            id,
            from: from.to_string(),
            to: to.to_string(),
            body: body.to_vec(),
            ts_millis: now_millis(),
        };
        self.wal.append(&rec)?;
        apply_record(&mut self.kv, &mut self.inbox, &rec);
        Ok(id)
    }

    pub fn inbox_list(&self, agent: &str, unread_only: bool) -> Vec<InboxMessage> {
        self.inbox.list_for(agent, unread_only)
    }

    pub fn inbox_ack(&mut self, agent: &str, id: Uuid) -> Result<()> {
        if self.inbox.get(&id).is_none() {
            return Err(FryError::InboxNotFound(id.to_string()));
        }
        let rec = WalRecord::InboxAck {
            id,
            agent: agent.to_string(),
            ts_millis: now_millis(),
        };
        self.wal.append(&rec)?;
        apply_record(&mut self.kv, &mut self.inbox, &rec);
        Ok(())
    }
}

fn apply_record(kv: &mut KvStore, inbox: &mut InboxIndex, record: &WalRecord) {
    match record {
        WalRecord::Put { ns, key, value, .. } => {
            kv.insert((ns.clone(), key.clone()), value.clone());
        }
        WalRecord::Delete { ns, key, .. } => {
            kv.remove(&(ns.clone(), key.clone()));
        }
        WalRecord::InboxPut { .. } | WalRecord::InboxAck { .. } => {
            inbox.apply(record);
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn kv_survives_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        {
            let mut db = FryDb::open(&path).unwrap();
            db.put("global", "greeting", b"hi agents").unwrap();
        }
        let db = FryDb::open(&path).unwrap();
        assert_eq!(db.get("global", "greeting"), Some(b"hi agents".as_slice()));
    }

    #[test]
    fn second_open_fails_while_locked() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let _db = FryDb::open(&path).unwrap();
        assert!(matches!(FryDb::open(&path), Err(FryError::Locked)));
    }

    #[test]
    fn inbox_broadcast_and_ack() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let mut db = FryDb::open(&path).unwrap();
        let id = db.inbox_send("news", "*", b"hourly digest ready").unwrap();
        let msgs = db.inbox_list("principal-dev", true);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, id);
        db.inbox_ack("principal-dev", id).unwrap();
        assert!(db.inbox_list("principal-dev", true).is_empty());
    }
}
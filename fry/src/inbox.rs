use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::record::WalRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxMessage {
    pub id: Uuid,
    pub from: String,
    pub to: String,
    pub body: Vec<u8>,
    pub ts_millis: u64,
    pub read: bool,
}

#[derive(Debug, Default)]
pub struct InboxIndex {
    by_id: HashMap<Uuid, InboxMessage>,
    by_recipient: HashMap<String, Vec<Uuid>>,
    acked: HashSet<Uuid>,
}

impl InboxIndex {
    pub fn apply(&mut self, record: &WalRecord) {
        match record {
            WalRecord::InboxPut {
                id,
                from,
                to,
                body,
                ts_millis,
            } => {
                if self.acked.contains(id) {
                    return;
                }
                let msg = InboxMessage {
                    id: *id,
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                    ts_millis: *ts_millis,
                    read: false,
                };
                self.by_id.insert(*id, msg);
                self.by_recipient
                    .entry(to.clone())
                    .or_default()
                    .push(*id);
            }
            WalRecord::InboxAck { id, agent: _, .. } => {
                self.acked.insert(*id);
                if let Some(msg) = self.by_id.get_mut(id) {
                    msg.read = true;
                }
            }
            _ => {}
        }
    }

    pub fn list_for(&self, agent: &str, unread_only: bool) -> Vec<InboxMessage> {
        let mut out = Vec::new();
        for key in [agent, "*"] {
            let Some(ids) = self.by_recipient.get(key) else {
                continue;
            };
            for id in ids {
                let Some(msg) = self.by_id.get(id) else {
                    continue;
                };
                if unread_only && msg.read {
                    continue;
                }
                if out.iter().any(|m: &InboxMessage| m.id == *id) {
                    continue;
                }
                out.push(msg.clone());
            }
        }
        out.sort_by_key(|m| m.ts_millis);
        out
    }

    pub fn get(&self, id: &Uuid) -> Option<&InboxMessage> {
        self.by_id.get(id)
    }
}
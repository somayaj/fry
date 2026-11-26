use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One durable entry in `wal.ndjson`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WalRecord {
    Put {
        ns: String,
        key: String,
        #[serde(with = "bytes_field")]
        value: Vec<u8>,
        ts_millis: u64,
    },
    Delete {
        ns: String,
        key: String,
        ts_millis: u64,
    },
    InboxPut {
        id: Uuid,
        from: String,
        to: String,
        #[serde(with = "bytes_field")]
        body: Vec<u8>,
        ts_millis: u64,
    },
    InboxAck {
        id: Uuid,
        agent: String,
        ts_millis: u64,
    },
}

mod bytes_field {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_roundtrip() {
        let r = WalRecord::Put {
            ns: "global".into(),
            key: "k".into(),
            value: b"hello".to_vec(),
            ts_millis: 1,
        };
        let line = serde_json::to_string(&r).unwrap();
        let back: WalRecord = serde_json::from_str(&line).unwrap();
        assert_eq!(r, back);
    }
}
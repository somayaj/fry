use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FryError {
    #[error("database locked by another process")]
    Locked,
    #[error("corrupt or unknown record in WAL")]
    CorruptWal(String),
    #[error("inbox message not found: {0}")]
    InboxNotFound(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FryError>;
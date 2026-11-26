use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::{FryError, Result};
use crate::record::WalRecord;

pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Wal {
    pub fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("wal.ndjson");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            writer: BufWriter::new(file),
        })
    }

    pub fn replay(dir: &Path) -> Result<Vec<WalRecord>> {
        let path = dir.join("wal.ndjson");
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let mut records = Vec::new();
        for (line_no, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let rec: WalRecord = serde_json::from_str(trimmed).map_err(|e| {
                FryError::CorruptWal(format!("line {}: {e}", line_no + 1))
            })?;
            records.push(rec);
        }
        Ok(records)
    }

    pub fn append(&mut self, record: &WalRecord) -> Result<()> {
        let line = serde_json::to_string(record)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        self.writer.get_mut().sync_data()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
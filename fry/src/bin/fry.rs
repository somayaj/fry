use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use fry::FryDb;
use uuid::Uuid;

fn usage() -> &'static str {
    r#"fry — shared memory and inbox for autonomous agents

Store durable key/value facts and pass messages between agent IDs on one machine
(or one shared data directory). Single-writer WAL; safe for scheduled agents.

Usage:
  fry put    <ns> <key> [value]     # value from args or stdin
  fry get    <ns> <key>
  fry del    <ns> <key>
  fry scan   <ns> [prefix]
  fry send   <from-agent> <to-agent> [body]
  fry inbox  <agent-id> [--all]
  fry ack    <agent-id> <message-id>

Options:
  --data <dir>   database directory (default: see FRY_DATA_DIR / XDG below)
  -h, --help

Env:
  FRY_DATA_DIR     override default store (highest priority after --data)
  XDG_DATA_HOME    default store: $XDG_DATA_HOME/fry, else ~/.local/share/fry

Agents:
  Agent IDs are arbitrary strings (researcher, code-review, nightly-report).
  Send to "*" to broadcast; inbox lists messages for <agent-id> and "*".

Examples:
  fry put shared deployment "staging is green"
  fry get shared deployment
  fry send researcher writer "found three papers on topic X"
  fry inbox writer
  fry ack writer 550e8400-e29b-41d4-a716-446655440000
"#
}

/// Default store when neither `--data` nor `FRY_DATA_DIR` is set.
fn default_data_dir() -> PathBuf {
    if let Ok(dir) = env::var("FRY_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(xdg) = env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("fry");
        }
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".local/share/fry");
    }
    PathBuf::from(".fry")
}

fn resolve_data_dir(explicit: Option<String>) -> PathBuf {
    explicit
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_data_dir)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprint!("{}", usage());
        return ExitCode::from(2);
    }

    let (mut data_dir, mut i) = (None::<String>, 1usize);
    while i < args.len() {
        match args[i].as_str() {
            "--data" => {
                i += 1;
                data_dir = Some(args.get(i).cloned().unwrap_or_default());
                i += 1;
            }
            "--help" | "-h" => {
                eprint!("{}", usage());
                return ExitCode::SUCCESS;
            }
            _ => break,
        }
    }

    let dir = resolve_data_dir(data_dir);

    let Some(cmd) = args.get(i) else {
        eprint!("{}", usage());
        return ExitCode::from(2);
    };

    let mut db = match FryDb::open(&dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    let tail = &args[i + 1..];
    match cmd.as_str() {
        "put" => {
            let Some(ns) = tail.first() else {
                eprintln!("put needs <ns> <key>");
                return ExitCode::from(2);
            };
            let Some(key) = tail.get(1) else {
                eprintln!("put needs <ns> <key>");
                return ExitCode::from(2);
            };
            let value = if tail.len() > 2 {
                tail[2..].join(" ").into_bytes()
            } else {
                read_stdin_bytes()
            };
            if let Err(e) = db.put(ns, key, &value) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
        "get" => {
            let Some(ns) = tail.first() else {
                return ExitCode::from(2);
            };
            let Some(key) = tail.get(1) else {
                return ExitCode::from(2);
            };
            match db.get(ns, key) {
                Some(v) => {
                    let _ = io::stdout().write_all(v);
                }
                None => return ExitCode::from(3),
            }
        }
        "del" => {
            let Some(ns) = tail.first() else {
                return ExitCode::from(2);
            };
            let Some(key) = tail.get(1) else {
                return ExitCode::from(2);
            };
            if let Err(e) = db.delete(ns, key) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
        "scan" => {
            let Some(ns) = tail.first() else {
                return ExitCode::from(2);
            };
            let prefix = tail.get(1).map(|s| s.as_str()).unwrap_or("");
            for (k, v) in db.scan_prefix(ns, prefix) {
                let text = String::from_utf8_lossy(v);
                println!("{k}\t{text}");
            }
        }
        "send" => {
            let Some(from) = tail.first() else {
                return ExitCode::from(2);
            };
            let Some(to) = tail.get(1) else {
                return ExitCode::from(2);
            };
            let body = if tail.len() > 2 {
                tail[2..].join(" ").into_bytes()
            } else {
                read_stdin_bytes()
            };
            match db.inbox_send(from, to, &body) {
                Ok(id) => println!("{id}"),
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
        }
        "inbox" => {
            let Some(agent) = tail.first() else {
                return ExitCode::from(2);
            };
            let unread_only = !tail.iter().any(|a| a == "--all");
            for m in db.inbox_list(agent, unread_only) {
                let body = String::from_utf8_lossy(&m.body);
                let flag = if m.read { "read" } else { "new" };
                println!("{}\t{}\t{}\t{}\t{}", m.id, flag, m.from, m.to, body);
            }
        }
        "ack" => {
            let Some(agent) = tail.first() else {
                return ExitCode::from(2);
            };
            let Some(id_s) = tail.get(1) else {
                return ExitCode::from(2);
            };
            let Ok(id) = Uuid::from_str(id_s) else {
                eprintln!("invalid message id");
                return ExitCode::from(2);
            };
            if let Err(e) = db.inbox_ack(agent, id) {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
        _ => {
            eprintln!("unknown command: {cmd}");
            eprint!("{}", usage());
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn read_stdin_bytes() -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dir_uses_xdg_when_home_set() {
        let _guard = env_lock("HOME", Some("/tmp/fry-test-home"));
        let _xdg = env_lock("XDG_DATA_HOME", None);
        let _fdd = env_lock("FRY_DATA_DIR", None);
        let p = default_data_dir();
        assert_eq!(
            p,
            PathBuf::from("/tmp/fry-test-home/.local/share/fry")
        );
    }

    struct EnvGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.old {
                Some(v) => env::set_var(self.key, v),
                None => env::remove_var(self.key),
            }
        }
    }

    fn env_lock(key: &'static str, value: Option<&str>) -> EnvGuard {
        let old = env::var(key).ok();
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        EnvGuard { key, old }
    }
}
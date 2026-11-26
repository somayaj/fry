use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::str::FromStr;

use fry::FryDb;
use uuid::Uuid;

fn usage() -> &'static str {
    r#"fry — embedded agent memory (nanovm)

Usage:
  fry --data <dir> put    <ns> <key> [value]     # value from args or stdin
  fry --data <dir> get    <ns> <key>
  fry --data <dir> del    <ns> <key>
  fry --data <dir> scan   <ns> <prefix>
  fry --data <dir> send   <from> <to> [body]     # body from args or stdin
  fry --data <dir> inbox  <agent> [--all]
  fry --data <dir> ack    <agent> <message-id>

Env:
  FRY_DATA_DIR   default database directory (overridden by --data)
"#
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprint!("{}", usage());
        return ExitCode::from(2);
    }

    let (mut data_dir, mut i) = (env::var("FRY_DATA_DIR").ok(), 1usize);
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

    let Some(dir) = data_dir.filter(|s| !s.is_empty()) else {
        eprintln!("missing --data <dir> or FRY_DATA_DIR");
        eprint!("{}", usage());
        return ExitCode::from(2);
    };

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
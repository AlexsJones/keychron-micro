use crate::config;
use crate::status::Status;
use evdev::KeyCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

const PAGE: &str = include_str!("index.html");

/// Refuse a request body larger than this. config.toml is a few KB; anything
/// near this is a mistake or a probe, and we would rather not buffer it.
const MAX_BODY: usize = 256 * 1024;

/// What the daemon knows, shared with the HTTP thread.
pub struct Shared {
    pub binds: RwLock<HashMap<KeyCode, config::Action>>,
    pub status: Mutex<Status>,
    /// Last key pressed, bound or not -- this is what makes the page usable for
    /// identifying pictogram caps without stopping the daemon to run `learn`.
    pub last_key: Mutex<Option<String>>,
    /// (command, resolved args) handed to every script it spawns. Swapped on
    /// save, so changing agent takes effect on the next keypress.
    pub harness: Mutex<(String, Vec<String>)>,
    pub config_path: PathBuf,
    pub root: PathBuf,
    pub pad: String,
    /// Where this server is listening, for scripts that want to open it.
    /// Fixed at startup: changing the port in config.toml cannot rebind a
    /// listening socket, so that one setting needs a restart.
    pub web_url: Option<String>,
}

impl Shared {
    fn state_json(&self) -> Value {
        let binds = self.binds.read().unwrap();
        let mut rows: Vec<Value> = binds
            .iter()
            .map(|(k, a)| {
                json!({
                    "key": config::key_name(*k).unwrap_or_else(|| format!("{k:?}")),
                    "label": a.label,
                    "run": a.what.describe(),
                })
            })
            .collect();
        rows.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));

        let (cmd, args) = self.harness.lock().unwrap().clone();
        json!({
            "pad": self.pad,
            "status": self.status.lock().unwrap().label(),
            "lastKey": *self.last_key.lock().unwrap(),
            "harness": if args.is_empty() { cmd } else { format!("{cmd} {}", args.join(" ")) },
            "binds": rows,
            "config": std::fs::read_to_string(&self.config_path).unwrap_or_default(),
            "configPath": self.config_path.display().to_string(),
        })
    }

    /// Validate, then write, then swap. In that order: a config that would not
    /// load must never reach disk, or the next restart comes up dead.
    fn save(&self, text: &str) -> Result<usize, String> {
        let loaded = config::parse(text, &self.root).map_err(|e| e.to_string())?;
        let n = loaded.binds.len();

        // Write via a temp file in the same directory so the replace is atomic
        // and a full disk cannot leave a half-written config behind.
        let tmp = self.config_path.with_extension("toml.new");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.config_path).map_err(|e| e.to_string())?;

        *self.harness.lock().unwrap() = (
            loaded.harness.command.clone(),
            loaded.harness.resolved_args(&self.root),
        );
        *self.binds.write().unwrap() = loaded.binds;
        Ok(n)
    }
}

fn respond(s: &mut TcpStream, code: &str, ctype: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {code}\r\n\
         Content-Type: {ctype}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn handle(mut s: TcpStream, shared: &Arc<Shared>) {
    let mut r = BufReader::new(match s.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    });

    let mut line = String::new();
    if r.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));

    let mut len = 0usize;
    loop {
        let mut h = String::new();
        match r.read_line(&mut h) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => return,
        }
        if h.trim().is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }

    match (method, path) {
        ("GET", "/") => respond(&mut s, "200 OK", "text/html; charset=utf-8", PAGE.as_bytes()),

        ("GET", "/api/state") => {
            let body = shared.state_json().to_string();
            respond(&mut s, "200 OK", "application/json", body.as_bytes());
        }

        ("POST", "/api/config") => {
            if len > MAX_BODY {
                respond(&mut s, "413 Payload Too Large", "application/json",
                    json!({"error": "config too large"}).to_string().as_bytes());
                return;
            }
            let mut body = vec![0u8; len];
            if r.read_exact(&mut body).is_err() {
                return;
            }
            let text = String::from_utf8_lossy(&body).to_string();
            match shared.save(&text) {
                Ok(n) => {
                    println!("config saved from web ui: {n} keys bound");
                    respond(&mut s, "200 OK", "application/json",
                        json!({"ok": true, "bound": n}).to_string().as_bytes());
                }
                // 422, not 500: the config is the user's to fix, and the page
                // shows this message verbatim.
                Err(e) => respond(&mut s, "422 Unprocessable Entity", "application/json",
                    json!({"error": e}).to_string().as_bytes()),
            }
        }

        _ => respond(&mut s, "404 Not Found", "text/plain", b"not found\n"),
    }
}

/// Serve until the process exits. Localhost only, and deliberately so: this
/// endpoint rewrites config.toml, and every binding in it is a script this
/// daemon will run on a keypress. It must not be reachable from the network.
pub fn serve(shared: Arc<Shared>, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    println!("web:    http://127.0.0.1:{port}");
    for conn in listener.incoming() {
        match conn {
            Ok(s) => {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || handle(s, &shared));
            }
            Err(e) => eprintln!("web: {e}"),
        }
    }
    Ok(())
}

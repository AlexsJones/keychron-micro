use crate::config::{self, Do};
use crate::status::Status;
use crate::tap::Tapper;
use crate::via::{self, Via};
use crate::web::{self, Shared};
use evdev::EventSummary;
use std::io::{BufRead, BufReader};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// How long a finished script's colour lingers before the pad returns to idle.
const SETTLE: Duration = Duration::from_millis(900);

enum Msg {
    /// A bound key was pressed.
    Fire { label: String, script: PathBuf },
    /// Someone wrote a state to the status pipe.
    Set(Status),
}

/// The pipe scripts and Claude Code hooks write to, e.g.
/// `printf 'thinking\n' > "$XDG_RUNTIME_DIR/keychron-micro/status"`.
pub fn status_pipe() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("keychron-micro/status")
}

fn make_pipe(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // A stale pipe from a previous run is fine to reuse; anything else is not.
    match std::fs::metadata(path) {
        Ok(m) => {
            use std::os::unix::fs::FileTypeExt;
            if !m.file_type().is_fifo() {
                return Err(std::io::Error::other(format!(
                    "{} exists and is not a fifo",
                    path.display()
                )));
            }
            return Ok(());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(std::io::Error::other)?;
    if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Forward states written to the pipe. Opened read+write so that it never sees
/// EOF when the last writer closes -- otherwise this loop would spin.
fn watch_pipe(path: PathBuf, tx: Sender<Msg>) {
    let f = match std::fs::OpenOptions::new().read(true).write(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("status pipe {}: {e}", path.display());
            return;
        }
    };
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        match Status::parse(&line) {
            Some(s) => {
                if tx.send(Msg::Set(s)).is_err() {
                    return;
                }
            }
            None if line.trim().is_empty() => {}
            None => eprintln!("status pipe: unknown state {line:?}"),
        }
    }
}

/// Owns the pad's lighting. Kept on one thread so key presses and pipe writes
/// cannot interleave half-finished HID transactions, and so scripts run in the
/// order they were pressed -- which matters when the knob is cycling panes.
fn lighting(rx: mpsc::Receiver<Msg>, mut v: Via, shared: Arc<Shared>) {
    let paint = |v: &mut Via, s: Status| {
        *shared.status.lock().unwrap() = s;
        let (h, sat) = s.hue_sat();
        if let Err(e) = v.set_color(h, sat) {
            eprintln!("lights: {e}");
        }
    };

    // When to drop back to idle, or None if we are already there. Waiting for it
    // as a recv timeout rather than sleeping is the whole point: a blocking
    // sleep here would hold up every key pressed during it, and the encoder
    // fires far faster than SETTLE. Spinning the knob would queue seconds of
    // backlog and the pad would still be catching up after you stopped.
    let mut settle_at: Option<Instant> = None;

    loop {
        let wait = match settle_at {
            Some(t) => t.saturating_duration_since(Instant::now()),
            // Nothing pending: block until there is.
            None => Duration::from_secs(3600),
        };

        let msg = match rx.recv_timeout(wait) {
            Ok(m) => m,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                paint(&mut v, Status::Idle);
                settle_at = None;
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };

        match msg {
            // An explicit state, from a script or a Claude hook. It stands until
            // something else changes it, so no settle: "waiting for you" should
            // not time out into white after a second.
            Msg::Set(s) => {
                paint(&mut v, s);
                settle_at = None;
            }
            Msg::Fire { label, script } => {
                paint(&mut v, Status::Thinking);
                // Scripts read the harness from the environment rather than
                // naming an agent themselves, so switching claude->codex is one
                // edit in config.toml instead of a change to every script.
                let (cmd, args) = shared.harness.lock().unwrap().clone();
                let mut c = Command::new(&script);
                c.env("KEYCHRON_HARNESS", cmd)
                    .env("KEYCHRON_HARNESS_ARGS", config::quote_args(&args))
                    .stdin(Stdio::null());
                if let Some(url) = &shared.web_url {
                    c.env("KEYCHRON_WEB_URL", url);
                }
                let outcome = c.spawn().and_then(|mut c| c.wait());

                let s = match outcome {
                    Ok(st) if st.success() => Status::Complete,
                    Ok(st) => {
                        eprintln!("{label}: {} exited {st}", script.display());
                        Status::Error
                    }
                    Err(e) => {
                        eprintln!("{label}: {} failed to start: {e}", script.display());
                        Status::Error
                    }
                };
                paint(&mut v, s);
                // Push the deadline out rather than sleep to it. Held down or
                // spun, the pad stays lit and settles once, SETTLE after the
                // last one, instead of strobing through a queue.
                settle_at = Some(Instant::now() + SETTLE);
            }
        }
    }
}

pub fn run(config_path: &Path, root: &Path) -> std::io::Result<()> {
    let loaded = config::load(config_path, root)?;

    let (path, mut dev) = crate::pad::find()?;
    dev.grab().map_err(|e| {
        std::io::Error::other(format!(
            "EVIOCGRAB failed ({e}) -- another keychron-micro may already be running"
        ))
    })?;
    let pad_name = dev.name().unwrap_or("unknown").to_string();

    let mut v = Via::open()?;
    v.set_effect(via::EFFECT_SOLID)?;
    v.set_brightness(255)?;

    let shared = Arc::new(Shared {
        binds: RwLock::new(loaded.binds),
        status: Mutex::new(Status::Idle),
        last_key: Mutex::new(None),
        harness: Mutex::new((
            loaded.harness.command.clone(),
            loaded.harness.resolved_args(root),
        )),
        config_path: config_path.to_path_buf(),
        root: root.to_path_buf(),
        pad: format!("{pad_name} [{}]", path.display()),
        web_url: (loaded.web.port != 0)
            .then(|| format!("http://127.0.0.1:{}", loaded.web.port)),
    });

    let (tx, rx) = mpsc::channel();
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || lighting(rx, v, shared));
    }
    tx.send(Msg::Set(Status::Idle)).ok();

    let pipe = status_pipe();
    match make_pipe(&pipe) {
        Ok(()) => {
            let (p, t) = (pipe.clone(), tx.clone());
            std::thread::spawn(move || watch_pipe(p, t));
        }
        // Worth continuing without: the keys still work, only hooks are lost.
        Err(e) => eprintln!("status pipe unavailable ({e}) -- colours from scripts only"),
    }

    println!("pad:    {pad_name} [{}] -- grabbed", path.display());
    println!("keys:   {} bound", shared.binds.read().unwrap().len());
    println!("status: {}", pipe.display());

    if loaded.web.port != 0 {
        let shared = Arc::clone(&shared);
        let port = loaded.web.port;
        // Not fatal: a busy port should cost you the UI, not your macro keys.
        std::thread::spawn(move || {
            if let Err(e) = web::serve(shared, port) {
                eprintln!("web: port {port}: {e} -- UI disabled, keys unaffected");
            }
        });
    }

    // One virtual keyboard for the life of the daemon, declaring every key any
    // `tap` binding might send. A uinput device's capabilities are fixed at
    // creation, and a fresh device per tap would need ~200ms for the compositor
    // to notice it -- useless for a key you press to answer a prompt.
    let mut tapper = match tap_keys(&shared) {
        keys if keys.is_empty() => None,
        keys => match Tapper::new(keys.into_iter()) {
            Ok(t) => Some(t),
            // Not fatal: `run` bindings are unaffected, and saying why once is
            // better than failing silently on every press.
            Err(e) => {
                eprintln!("tap: {e} -- `tap` bindings will do nothing");
                None
            }
        },
    };

    loop {
        for ev in dev.fetch_events()? {
            let EventSummary::Key(_, key, 1) = ev.destructure() else {
                continue; // key-down only; repeats and releases are noise here
            };

            // Record every press, bound or not: this is what lets the web UI
            // name a pictogram cap without stopping the daemon to run `learn`.
            if let Some(name) = config::key_name(key) {
                *shared.last_key.lock().unwrap() = Some(name);
            }

            let hit = shared
                .binds
                .read()
                .unwrap()
                .get(&key)
                .map(|a| (a.label.clone(), a.what.clone()));

            let Some((label, what)) = hit else { continue };

            match what {
                // Handled here rather than on the lighting thread: that thread
                // blocks while a script runs, and a tap queued behind a terminal
                // launch would land whole seconds after you pressed it.
                Do::Tap { name, key } => {
                    println!("{key:?} -> {label} (tap {name})");
                    match tapper.as_mut() {
                        Some(t) => {
                            if let Err(e) = t.tap(key) {
                                eprintln!("{label}: tap {name}: {e}");
                            }
                        }
                        None => eprintln!("{label}: no virtual keyboard, cannot tap {name}"),
                    }
                }
                Do::Run { script, .. } => {
                    println!("{key:?} -> {label}");
                    tx.send(Msg::Fire { label, script })
                        .map_err(|_| std::io::Error::other("lighting thread died"))?;
                }
            }
        }
    }
}

/// Every key any `tap` binding might emit, which is what the virtual keyboard
/// has to declare up front.
fn tap_keys(shared: &Shared) -> Vec<evdev::KeyCode> {
    shared
        .binds
        .read()
        .unwrap()
        .values()
        .filter_map(|a| match &a.what {
            Do::Tap { key, .. } => Some(*key),
            Do::Run { .. } => None,
        })
        .collect()
}

use evdev::KeyCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn default_port() -> u16 {
    7373
}

fn default_harness_command() -> String {
    "claude".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Web {
    /// 0 disables the server entirely.
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for Web {
    fn default() -> Self {
        Web {
            port: default_port(),
        }
    }
}

/// Which coding agent the scripts launch. Kept in config rather than hardcoded
/// in each script so swapping claude for codex is one edit here, not a rewrite
/// of every script that opens one.
#[derive(Debug, Deserialize, Clone)]
pub struct Harness {
    /// The executable, e.g. "claude" or "codex".
    #[serde(default = "default_harness_command")]
    pub command: String,
    /// Flags passed on every launch, e.g. skipping permission prompts. Named
    /// per-harness because no two agree on what that flag is called.
    #[serde(default)]
    pub args: Vec<String>,
}

impl Default for Harness {
    fn default() -> Self {
        Harness {
            command: default_harness_command(),
            args: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Bind {
    /// evdev name as printed by `keychron-micro learn`, e.g. "KEY_KP6".
    pub key: String,
    /// Documentation only: which pictogram the keycap actually shows.
    #[serde(default)]
    pub label: String,
    /// Script path, relative to the repo root so the config stays portable.
    pub run: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub web: Web,
    #[serde(default)]
    pub harness: Harness,
    #[serde(default, rename = "bind")]
    pub binds: Vec<Bind>,
}

/// Resolved binding: the key to match, and the absolute script to run.
#[derive(Clone)]
pub struct Action {
    pub label: String,
    /// As written in config.toml, for display.
    pub run: String,
    pub script: PathBuf,
}

pub struct Loaded {
    pub binds: HashMap<KeyCode, Action>,
    pub web: Web,
    pub harness: Harness,
}

/// Wrap for a shell, so an argument containing spaces survives being handed to
/// a script through the environment.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Args as one pre-quoted string, ready to interpolate into a shell command
/// line. Scripts read this from $KEYCHRON_HARNESS_ARGS. Quoted only here, at
/// the point it meets a shell -- anywhere else it is just noise to read.
pub fn quote_args(args: &[String]) -> String {
    args.iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Harness {
    /// `{repo}` expands to the repo root, so args can name files that ship with
    /// the repo (Claude's hook settings, say) without hardcoding a clone path.
    pub fn resolved_args(&self, root: &Path) -> Vec<String> {
        let root = root.display().to_string();
        self.args
            .iter()
            .map(|a| a.replace("{repo}", &root))
            .collect()
    }

}

/// evdev has no name->KeyCode lookup, but its Debug impl prints the kernel name
/// ("KEY_KP6"). Walk the whole code space once and invert that, which keeps this
/// in step with whatever evdev knows rather than a table we would have to update.
fn key_names() -> HashMap<String, KeyCode> {
    (0..0x300u16)
        .map(KeyCode::new)
        .filter_map(|k| {
            let name = format!("{k:?}");
            name.starts_with("KEY_").then_some((name, k))
        })
        .collect()
}

/// The name evdev knows a code by, or None if it has none.
pub fn key_name(k: KeyCode) -> Option<String> {
    let name = format!("{k:?}");
    name.starts_with("KEY_").then_some(name)
}

/// Parse and resolve, checking every script exists and is executable now rather
/// than discovering it on a keypress hours later. Kept separate from `load` so
/// the web UI can validate an edit before it is allowed to overwrite anything.
pub fn parse(text: &str, root: &Path) -> std::io::Result<Loaded> {
    let cfg: Config = toml::from_str(text).map_err(|e| {
        // toml's Display is already a multi-line pointer at the offending span.
        std::io::Error::other(e.message().to_string())
    })?;

    let names = key_names();
    let mut binds: HashMap<KeyCode, Action> = HashMap::new();

    for b in cfg.binds {
        let key = *names.get(b.key.as_str()).ok_or_else(|| {
            std::io::Error::other(format!(
                "unknown key {:?} -- press it with the web UI open, or run `make learn`",
                b.key
            ))
        })?;

        let script = root.join(&b.run);
        if !script.is_file() {
            return Err(std::io::Error::other(format!(
                "{}: no such script (from key {})",
                script.display(),
                b.key
            )));
        }
        if !is_executable(&script) {
            return Err(std::io::Error::other(format!(
                "{} is not executable -- chmod +x it",
                script.display()
            )));
        }

        if let Some(prev) = binds.insert(
            key,
            Action {
                label: b.label,
                run: b.run,
                script,
            },
        ) {
            return Err(std::io::Error::other(format!(
                "{} is bound twice (already bound to {})",
                b.key, prev.run
            )));
        }
    }
    Ok(Loaded {
        binds,
        web: cfg.web,
        harness: cfg.harness,
    })
}

pub fn load(path: &Path, root: &Path) -> std::io::Result<Loaded> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!(
                "{}: {e} -- copy config.example.toml to config.toml to get started",
                path.display()
            ),
        )
    })?;
    parse(&text, root).map_err(|e| std::io::Error::other(format!("{}: {e}", path.display())))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

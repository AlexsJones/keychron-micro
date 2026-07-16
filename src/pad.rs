use evdev::{Device, EventSummary, KeyCode};
use std::path::PathBuf;

pub const VENDOR: u16 = 0x3434;
pub const PRODUCT: u16 = 0x0800;

const UDEV_HINT: &str = "install the udev rule: \
     sudo cp udev/70-keychron-micro.rules /etc/udev/rules.d/ && \
     sudo udevadm control --reload-rules && sudo udevadm trigger";

fn sysfs_hex(path: &std::path::Path) -> Option<u16> {
    let raw = std::fs::read_to_string(path).ok()?;
    u16::from_str_radix(raw.trim(), 16).ok()
}

/// Every evdev node belonging to the pad, found via sysfs so that nodes we lack
/// permission to open are still discovered. `evdev::enumerate()` silently omits
/// those, which otherwise makes a permissions problem look like the wrong device.
fn nodes() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/input") else {
        return found;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }
        let id = e.path().join("device/id");
        if sysfs_hex(&id.join("vendor")) == Some(VENDOR)
            && sysfs_hex(&id.join("product")) == Some(PRODUCT)
        {
            found.push(PathBuf::from("/dev/input").join(&name));
        }
    }
    found.sort();
    found
}

/// The pad presents several evdev nodes (keyboard, mouse, consumer control).
/// Only the one carrying ordinary KEY_* events is worth grabbing.
pub fn find() -> std::io::Result<(PathBuf, Device)> {
    let all = nodes();
    if all.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no Keychron {VENDOR:04x}:{PRODUCT:04x} found -- is the pad plugged in?"),
        ));
    }

    let mut denied = Vec::new();
    let mut candidates = Vec::new();
    for path in &all {
        match Device::open(path) {
            Ok(dev) => {
                // evdev puts mouse/joystick buttons at BTN_MISC (0x100) and above;
                // anything below that is a keyboard key.
                let keys = dev
                    .supported_keys()
                    .map(|k| k.iter().filter(|key| key.code() < 0x100).count())
                    .unwrap_or(0);
                if keys > 0 {
                    candidates.push((path.clone(), dev, keys));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                denied.push(path.clone());
            }
            Err(_) => {}
        }
    }

    if !denied.is_empty() {
        let list: Vec<_> = denied.iter().map(|p| p.display().to_string()).collect();
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "cannot open {} -- Fedora withholds uaccess from keyboards, so {UDEV_HINT}",
                list.join(", ")
            ),
        ));
    }

    candidates.sort_by_key(|(_, _, keys)| std::cmp::Reverse(*keys));
    let (path, dev, _) = candidates.into_iter().next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "pad is present but exposes no keyboard node",
        )
    })?;
    Ok((path, dev))
}

/// Print each keypress as it arrives so a physical key can be matched to the
/// name the daemon will know it by. Grabs the device, so presses do not reach
/// whatever window happens to be focused.
pub fn learn() -> std::io::Result<()> {
    let (path, mut dev) = find()?;
    println!(
        "device: {}  [{}]",
        dev.name().unwrap_or("unknown"),
        path.display()
    );

    dev.grab().map_err(|e| {
        std::io::Error::other(format!(
            "EVIOCGRAB failed ({e}) -- another process may hold the device"
        ))
    })?;
    println!("grabbed -- presses will NOT reach other windows.");
    println!("press each key on the pad; ctrl-c when done.\n");

    let mut seen: Vec<KeyCode> = Vec::new();
    loop {
        for ev in dev.fetch_events()? {
            let EventSummary::Key(_, key, 1) = ev.destructure() else {
                continue; // key-down only
            };
            let first = !seen.contains(&key);
            if first {
                seen.push(key);
            }
            println!(
                "  {:<26} code={:<4} {}",
                format!("{key:?}"),
                key.code(),
                if first {
                    format!("<- new (#{})", seen.len())
                } else {
                    String::new()
                }
            );
        }
    }
}

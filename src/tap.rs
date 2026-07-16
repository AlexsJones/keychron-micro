use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

/// A keyboard the daemon can type on, so a pad key can answer a prompt in
/// whatever window has focus.
///
/// Wayland gives no way to send input to another window: that is the whole
/// point of it, and no D-Bus call or extension will do this. But the kernel
/// will happily create a keyboard, and a virtual one is indistinguishable from
/// the plastic sort by the time the compositor sees it. Nothing is being
/// bypassed here; the compositor delivers to the focused window exactly as it
/// would if you had typed the key yourself, which also means the tap goes
/// wherever focus happens to be.
pub struct Tapper {
    dev: VirtualDevice,
}

impl Tapper {
    /// Declares every key it might ever emit up front: a uinput device's
    /// capabilities are fixed when it is created, so the set has to be known
    /// now rather than at the first tap.
    pub fn new(keys: impl Iterator<Item = KeyCode>) -> std::io::Result<Self> {
        let mut set = AttributeSet::<KeyCode>::new();
        for k in keys {
            set.insert(k);
        }
        let dev = VirtualDevice::builder()
            .map_err(|e| uinput_error(e))?
            .name("keychron-micro virtual keyboard")
            .with_keys(&set)
            .map_err(|e| uinput_error(e))?
            .build()
            .map_err(|e| uinput_error(e))?;
        Ok(Tapper { dev })
    }

    pub fn tap(&mut self, key: KeyCode) -> std::io::Result<()> {
        let down = InputEvent::new(EventType::KEY.0, key.code(), 1);
        let up = InputEvent::new(EventType::KEY.0, key.code(), 0);
        // Both in one emit: the kernel syncs after the batch, so the press and
        // release arrive together rather than as a key held for a scheduler tick.
        self.dev.emit(&[down, up])
    }
}

fn uinput_error(e: std::io::Error) -> std::io::Error {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        return std::io::Error::new(
            e.kind(),
            "cannot open /dev/uinput -- `make install` adds a udev rule for it; \
             if the pad was plugged in already, replug it",
        );
    }
    e
}

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

    /// Press every key in order, then release in reverse. One key is a tap;
    /// several is a chord, with the leaders held while the last goes down --
    /// which is what makes ctrl+shift+w a shortcut rather than three letters.
    pub fn tap(&mut self, keys: &[KeyCode]) -> std::io::Result<()> {
        let mut ev = Vec::with_capacity(keys.len() * 2);
        for k in keys {
            ev.push(InputEvent::new(EventType::KEY.0, k.code(), 1));
        }
        for k in keys.iter().rev() {
            ev.push(InputEvent::new(EventType::KEY.0, k.code(), 0));
        }
        // One emit: the kernel syncs after the batch, so the whole chord lands
        // as a single atomic thing rather than keys held across scheduler ticks.
        self.dev.emit(&ev)
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

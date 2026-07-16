use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const VENDOR_USAGE_PAGE: [u8; 3] = [0x06, 0x60, 0xff];
const REPORT_LEN: usize = 32;

const CMD_GET_PROTOCOL_VERSION: u8 = 0x01;
const CMD_CUSTOM_SET_VALUE: u8 = 0x07;
const CMD_CUSTOM_GET_VALUE: u8 = 0x08;

const CHANNEL_RGB_MATRIX: u8 = 3;

const VALUE_BRIGHTNESS: u8 = 1;
const VALUE_EFFECT: u8 = 2;
const VALUE_EFFECT_SPEED: u8 = 3;
const VALUE_COLOR: u8 = 4;

/// QMK's "solid color" RGB matrix effect: the whole board shows one colour.
pub const EFFECT_SOLID: u8 = 1;

/// A snapshot of the pad's lighting, so we can put it back exactly as we found it.
#[derive(Debug, Clone, Copy)]
pub struct Lighting {
    pub brightness: u8,
    pub effect: u8,
    pub effect_speed: u8,
    pub hue: u8,
    pub sat: u8,
}

pub struct Via {
    dev: fs::File,
}

impl Via {
    /// Locate the Keychron's vendor HID interface by report descriptor rather than
    /// by hidraw number, which is reassigned on every replug.
    pub fn open() -> std::io::Result<Self> {
        let path = Self::discover()?;
        let dev = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)?;
        Ok(Self { dev })
    }

    fn discover() -> std::io::Result<PathBuf> {
        for entry in fs::read_dir("/sys/bus/hid/devices")? {
            let dir = entry?.path();
            let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !name.contains("3434:0800") {
                continue;
            }
            let Ok(desc) = fs::read(dir.join("report_descriptor")) else {
                continue;
            };
            if !desc.windows(3).any(|w| w == VENDOR_USAGE_PAGE) {
                continue;
            }
            let Ok(hidraw_dir) = fs::read_dir(dir.join("hidraw")) else {
                continue;
            };
            for h in hidraw_dir.flatten() {
                let node = PathBuf::from("/dev").join(h.file_name());
                if node.exists() {
                    return Ok(node);
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Keychron Q0 Max vendor HID interface (3434:0800, usage page 0xFF60) not found",
        ))
    }

    fn xact(&mut self, payload: &[u8]) -> std::io::Result<[u8; REPORT_LEN]> {
        let mut out = [0u8; REPORT_LEN + 1]; // leading byte is the hidraw report ID
        out[1..1 + payload.len()].copy_from_slice(payload);
        self.dev.write_all(&out)?;

        let deadline = Instant::now() + Duration::from_millis(500);
        let mut buf = [0u8; REPORT_LEN];
        while Instant::now() < deadline {
            match self.dev.read(&mut buf) {
                Ok(n) if n > 0 => return Ok(buf),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(e),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "no response from pad",
        ))
    }

    pub fn protocol_version(&mut self) -> std::io::Result<u16> {
        let r = self.xact(&[CMD_GET_PROTOCOL_VERSION])?;
        Ok(u16::from_be_bytes([r[1], r[2]]))
    }

    fn get_value(&mut self, value_id: u8) -> std::io::Result<[u8; 2]> {
        let r = self.xact(&[CMD_CUSTOM_GET_VALUE, CHANNEL_RGB_MATRIX, value_id])?;
        // The pad echoes the command byte on success and 0xFF when unsupported.
        if r[0] != CMD_CUSTOM_GET_VALUE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("pad rejected get of value {value_id}"),
            ));
        }
        Ok([r[3], r[4]])
    }

    fn set_value(&mut self, value_id: u8, data: &[u8]) -> std::io::Result<()> {
        let mut payload = vec![CMD_CUSTOM_SET_VALUE, CHANNEL_RGB_MATRIX, value_id];
        payload.extend_from_slice(data);
        self.xact(&payload)?;
        Ok(())
    }

    pub fn snapshot(&mut self) -> std::io::Result<Lighting> {
        let color = self.get_value(VALUE_COLOR)?;
        Ok(Lighting {
            brightness: self.get_value(VALUE_BRIGHTNESS)?[0],
            effect: self.get_value(VALUE_EFFECT)?[0],
            effect_speed: self.get_value(VALUE_EFFECT_SPEED)?[0],
            hue: color[0],
            sat: color[1],
        })
    }

    pub fn restore(&mut self, l: Lighting) -> std::io::Result<()> {
        self.set_value(VALUE_EFFECT, &[l.effect])?;
        self.set_value(VALUE_EFFECT_SPEED, &[l.effect_speed])?;
        self.set_value(VALUE_BRIGHTNESS, &[l.brightness])?;
        self.set_value(VALUE_COLOR, &[l.hue, l.sat])
    }

    pub fn set_color(&mut self, hue: u8, sat: u8) -> std::io::Result<()> {
        self.set_value(VALUE_COLOR, &[hue, sat])
    }

    pub fn set_brightness(&mut self, v: u8) -> std::io::Result<()> {
        self.set_value(VALUE_BRIGHTNESS, &[v])
    }

    pub fn set_effect(&mut self, v: u8) -> std::io::Result<()> {
        self.set_value(VALUE_EFFECT, &[v])
    }
}

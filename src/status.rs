/// The status model from Codex Micro, mapped onto Claude Code's hook lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Thinking,
    Complete,
    NeedsInput,
    Error,
}

impl Status {
    /// QMK hue/sat, both 0..=255. Saturation 0 is white regardless of hue.
    ///
    /// Hue is the 0..=360 colour wheel scaled to a byte: red 0, yellow 43,
    /// green 85, magenta 213. Chosen to be told apart at a glance across the
    /// room, which rules out neighbours -- amber (21) next to yellow (43) reads
    /// as the same colour on a diffused keycap.
    pub fn hue_sat(self) -> (u8, u8) {
        match self {
            Status::Idle => (0, 0),
            Status::Thinking => (43, 255),
            Status::Complete => (85, 255),
            Status::NeedsInput => (213, 255),
            Status::Error => (0, 255),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Idle => "idle (white)",
            Status::Thinking => "working (yellow)",
            Status::Complete => "complete (green)",
            Status::NeedsInput => "needs input (magenta)",
            Status::Error => "error (red)",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "idle" => Status::Idle,
            "thinking" => Status::Thinking,
            "complete" => Status::Complete,
            "needs_input" => Status::NeedsInput,
            "error" => Status::Error,
            _ => return None,
        })
    }
}

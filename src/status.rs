#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VolumeState {
    percent: u16,
    known: bool,
    muted: bool,
}

impl VolumeState {
    pub const UNKNOWN: Self = Self {
        percent: 0,
        known: false,
        muted: false,
    };

    pub const fn new(percent: u16, muted: bool) -> Self {
        Self {
            percent,
            known: true,
            muted,
        }
    }

    fn push_to(self, out: &mut String) {
        if !self.known {
            out.push_str("??%");
            return;
        }

        let value = if self.muted { 0 } else { self.percent };
        out.push_str(&value.to_string());
        out.push('%');
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutState {
    bytes: [u8; 3],
    len: u8,
}

impl LayoutState {
    pub const UNKNOWN: Self = Self {
        bytes: [b'?', b'?', 0],
        len: 2,
    };

    pub const fn from_bytes(bytes: [u8; 3], len: u8) -> Self {
        Self { bytes, len }
    }

    pub fn from_name(name: &str) -> Self {
        if name.contains("English (US)") {
            return Self::from_ascii("us");
        }

        if name.contains("Russian") {
            return Self::from_ascii("ru");
        }

        if name.contains("Ukrainian") {
            return Self::from_ascii("ua");
        }

        let mut bytes = [0u8; 3];
        let mut len = 0usize;

        for ch in name.bytes() {
            if !ch.is_ascii_alphabetic() {
                continue;
            }

            bytes[len] = ch.to_ascii_lowercase();
            len += 1;

            if len == bytes.len() {
                break;
            }
        }

        if len == 0 {
            Self::UNKNOWN
        } else {
            Self::from_bytes(bytes, len as u8)
        }
    }

    pub fn from_ascii(code: &str) -> Self {
        let bytes = code.as_bytes();
        let mut out = [0u8; 3];
        let len = bytes.len().min(out.len());
        out[..len].copy_from_slice(&bytes[..len]);
        Self::from_bytes(out, len as u8)
    }

    fn push_to(self, out: &mut String) {
        out.push_str(std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("??"));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockState {
    bytes: [u8; 22],
    len: u8,
}

impl ClockState {
    pub const fn from_bytes(bytes: [u8; 22], len: u8) -> Self {
        Self { bytes, len }
    }

    fn push_to(self, out: &mut String) {
        out.push_str(std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or(""));
    }
}

pub fn render(
    out: &mut String,
    volume: VolumeState,
    layout: LayoutState,
    time: ClockState,
) -> String {
    out.clear();
    volume.push_to(out);
    out.push(' ');
    layout.push_to(out);
    out.push(' ');
    time.push_to(out);
    out.clone()
}

/// A colour. The mouse stores lighting colours as R, B, G and DPI light colours as R, G, B;
/// [`Rgb::read_rbg`] / [`Rgb::write_rbg`] and friends hide that difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `rrggbb`, with or without a leading `#`.
    pub fn from_hex(s: &str) -> Option<Self> {
        let hex = s.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        Some(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }

    pub fn to_hex(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub(crate) fn read_rgb(bytes: &[u8], at: usize) -> Self {
        Self::new(bytes[at], bytes[at + 1], bytes[at + 2])
    }

    pub(crate) fn write_rgb(self, bytes: &mut [u8], at: usize) {
        bytes[at..at + 3].copy_from_slice(&[self.r, self.g, self.b]);
    }

    /// Lighting colours are stored red, blue, green.
    pub(crate) fn read_rbg(bytes: &[u8], at: usize) -> Self {
        Self::new(bytes[at], bytes[at + 2], bytes[at + 1])
    }

    pub(crate) fn write_rbg(self, bytes: &mut [u8], at: usize) {
        bytes[at..at + 3].copy_from_slice(&[self.r, self.b, self.g]);
    }
}

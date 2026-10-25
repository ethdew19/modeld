//! The button mapping table (command 0x12): 8-byte header + 20 slots of 4 bytes.
//! See PROTOCOL.md.

use crate::{Error, Result};

pub const BUTTON_SLOTS: usize = 20;
const HEADER: usize = 8;
/// Header + 20 slots.
pub(crate) const BUTTONS_LEN: usize = HEADER + BUTTON_SLOTS * 4;

/// Names of the first 6 slots, which are the physical buttons. Note that the official software
/// calls the front side button "button 4", which is slot 5 here.
pub const PHYSICAL_BUTTONS: [&str; 6] = [
    "Left",
    "Right",
    "Middle (wheel)",
    "Rear side button",
    "Front side button",
    "DPI button (bottom)",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0x01,
    Right = 0x02,
    Middle = 0x04,
    Back = 0x08,
    Forward = 0x10,
}

impl MouseButton {
    pub const ALL: [Self; 5] = [
        Self::Left,
        Self::Right,
        Self::Middle,
        Self::Back,
        Self::Forward,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left click",
            Self::Right => "right click",
            Self::Middle => "middle click",
            Self::Back => "back",
            Self::Forward => "forward",
        }
    }

    fn from_byte(b: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| *m as u8 == b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    /// `11 bb 00 00`
    Mouse(MouseButton),
    /// `21 mm kk 00`: kk = USB HID keyboard usage code, mm = modifiers (unconfirmed).
    Key { modifiers: u8, key: u8 },
    /// `41 00 00 00`: cycle DPI stages.
    Dpi,
    /// `70 ii mm rr`: macro, fields unconfirmed (number, play mode, repeat count).
    Macro { index: u8, mode: u8, repeat: u8 },
    /// `50 01 00 00`
    Unused,
    /// Anything else, kept as-is.
    Unknown([u8; 4]),
}

impl ButtonAction {
    fn from_slot(s: [u8; 4]) -> Self {
        match s {
            [0x11, b, 0, 0] => MouseButton::from_byte(b).map_or(Self::Unknown(s), Self::Mouse),
            [0x21, modifiers, key, 0] => Self::Key { modifiers, key },
            [0x41, 0, 0, 0] => Self::Dpi,
            [0x70, index, mode, repeat] => Self::Macro {
                index,
                mode,
                repeat,
            },
            [0x50, 0x01, 0, 0] => Self::Unused,
            _ => Self::Unknown(s),
        }
    }

    fn to_slot(self) -> [u8; 4] {
        match self {
            Self::Mouse(b) => [0x11, b as u8, 0, 0],
            Self::Key { modifiers, key } => [0x21, modifiers, key, 0],
            Self::Dpi => [0x41, 0, 0, 0],
            Self::Macro {
                index,
                mode,
                repeat,
            } => [0x70, index, mode, repeat],
            Self::Unused => [0x50, 0x01, 0, 0],
            Self::Unknown(s) => s,
        }
    }

    pub fn describe(self) -> String {
        match self {
            Self::Mouse(b) => b.name().to_string(),
            Self::Key { modifiers: 0, key } => format!("key {}", key_name(key)),
            Self::Key { modifiers, key } => {
                format!("key {} (modifiers {modifiers:#04x})", key_name(key))
            }
            Self::Dpi => "DPI cycle".to_string(),
            Self::Macro {
                index,
                mode,
                repeat,
            } => {
                format!("macro {index} (mode {mode}, repeat {repeat})")
            }
            Self::Unused => "(unused)".to_string(),
            Self::Unknown(s) => format!(
                "unknown {:02x} {:02x} {:02x} {:02x}",
                s[0], s[1], s[2], s[3]
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buttons {
    pub slots: [ButtonAction; BUTTON_SLOTS],
}

impl Buttons {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < BUTTONS_LEN || bytes[0] != 0x04 || bytes[1] != 0x12 {
            return Err(Error::Parse(
                "not a button table (expected 88 bytes, 04 12 ...)".into(),
            ));
        }
        Ok(Self {
            slots: std::array::from_fn(|i| {
                let at = HEADER + i * 4;
                ButtonAction::from_slot(bytes[at..at + 4].try_into().unwrap())
            }),
        })
    }

    /// The table as the official software writes it: `04 12 00 50` (0x50 = 80 bytes of slots).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = vec![0u8; BUTTONS_LEN];
        b[..4].copy_from_slice(&[0x04, 0x12, 0x00, (BUTTON_SLOTS * 4) as u8]);
        for (i, slot) in self.slots.iter().enumerate() {
            let at = HEADER + i * 4;
            b[at..at + 4].copy_from_slice(&slot.to_slot());
        }
        b
    }
}

/// Name of a USB HID keyboard usage code (HID Usage Tables, page 0x07).
pub fn key_name(code: u8) -> String {
    match code {
        0x04..=0x1d => ((b'A' + code - 0x04) as char).to_string(),
        0x1e..=0x26 => ((b'1' + code - 0x1e) as char).to_string(),
        0x27 => "0".into(),
        0x28 => "Enter".into(),
        0x29 => "Esc".into(),
        0x2a => "Backspace".into(),
        0x2b => "Tab".into(),
        0x2c => "Space".into(),
        0x3a..=0x45 => format!("F{}", code - 0x3a + 1),
        _ => format!("{code:#04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(slot5: [u8; 4]) -> Vec<u8> {
        let mut b = vec![0x04, 0x12, 0, 0, 0, 0, 0, 0];
        for s in [
            [0x11, 1, 0, 0],
            [0x11, 2, 0, 0],
            [0x11, 4, 0, 0],
            [0x11, 8, 0, 0],
            slot5,
            [0x41, 0, 0, 0],
        ] {
            b.extend_from_slice(&s);
        }
        for _ in 6..BUTTON_SLOTS {
            b.extend_from_slice(&[0x50, 0x01, 0, 0]);
        }
        b
    }

    #[test]
    fn decodes_real_tables() {
        let b = Buttons::from_bytes(&table([0x11, 0x10, 0, 0])).unwrap();
        assert_eq!(b.slots[4], ButtonAction::Mouse(MouseButton::Forward));
        assert_eq!(b.slots[5], ButtonAction::Dpi);
        assert_eq!(b.slots[6], ButtonAction::Unused);

        let key = Buttons::from_bytes(&table([0x21, 0, 0x05, 0])).unwrap();
        assert_eq!(key.slots[4].describe(), "key B");

        let mac = Buttons::from_bytes(&table([0x70, 1, 1, 1])).unwrap();
        assert_eq!(
            mac.slots[4],
            ButtonAction::Macro {
                index: 1,
                mode: 1,
                repeat: 1
            }
        );
    }

    #[test]
    fn writes_the_glorious_core_header() {
        let read = table([0x11, 0x10, 0, 0]);
        let written = Buttons::from_bytes(&read).unwrap().to_bytes();
        assert_eq!(&written[..4], [0x04, 0x12, 0x00, 0x50]);
        assert_eq!(written[8..], read[8..]);
    }
}

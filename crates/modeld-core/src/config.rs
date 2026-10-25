//! The 131-byte config blob (command 0x11): DPI, polling rate, lift-off distance, lighting.
//! See PROTOCOL.md for the byte layout.
//!
//! [`Config`] keeps the bytes it was parsed from, and [`Config::to_bytes`] only overwrites
//! the fields it understands. Unknown bytes and unused nibbles survive a read-modify-write.

use crate::{Error, Result, Rgb};

/// Length of the config as the mouse returns it.
pub const CONFIG_LEN: usize = 131;
/// DPI stage slots in the firmware. Glorious Core only shows 6.
pub const STAGES: usize = 8;
pub const MIN_DPI: u16 = 100;
/// The sensor's maximum (stored value 0x77).
pub const MAX_DPI: u16 = 12000;
/// Fastest lighting speed. Glorious Core stops at 3; above 5 the effects stop animating.
pub const MAX_SPEED: u8 = 5;

const POLLING: usize = 10;
const ACTIVE_STAGE: usize = 11;
const STAGE_MASK: usize = 12;
const DPI: usize = 13;
const STAGE_COLORS: usize = 29;
const LIGHT_MODE: usize = 53;
const GLORIOUS: usize = 54;
const SINGLE: usize = 56;
const BREATHING: usize = 60;
const TAIL: usize = 83;
const SEAMLESS: usize = 84;
const GRADIENT: usize = 86;
const RAVE: usize = 116;
const GLITCH: usize = 123;
const WAVE: usize = 124;
const BREATHING_SINGLE: usize = 125;
const LIFT_OFF: usize = 129;
const FLAGS: usize = 130;

/// Bit 7 of the polling byte.
const XY_INDEPENDENT: u8 = 0x80;
/// Bit 1 of the flags byte.
const LIGHTS_OFF: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollingRate {
    Hz125 = 1,
    Hz250 = 2,
    Hz500 = 3,
    Hz1000 = 4,
}

impl PollingRate {
    pub const ALL: [Self; 4] = [Self::Hz125, Self::Hz250, Self::Hz500, Self::Hz1000];

    pub fn hz(self) -> u16 {
        match self {
            Self::Hz125 => 125,
            Self::Hz250 => 250,
            Self::Hz500 => 500,
            Self::Hz1000 => 1000,
        }
    }

    pub fn from_hz(hz: u16) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.hz() == hz)
    }

    fn from_byte(b: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|r| *r as u8 == b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftOff {
    Mm2 = 1,
    Mm3 = 2,
}

impl LiftOff {
    pub fn mm(self) -> u8 {
        match self {
            Self::Mm2 => 2,
            Self::Mm3 => 3,
        }
    }

    pub fn from_mm(mm: u8) -> Option<Self> {
        match mm {
            2 => Some(Self::Mm2),
            3 => Some(Self::Mm3),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DpiStage {
    pub enabled: bool,
    /// DPI in steps of 100. Equal to `y` unless [`Config::xy_independent`] is on.
    pub x: u16,
    pub y: u16,
    /// Colour of the DPI light under the mouse while this stage is active.
    pub color: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingMode {
    Off = 0,
    Glorious = 1,
    Single = 2,
    Breathing = 3,
    Tail = 4,
    SeamlessBreathing = 5,
    /// Not in Glorious Core.
    Gradient = 6,
    Rave = 7,
    /// Not in Glorious Core: a fast moving pattern with odd colours.
    Glitch = 8,
    Wave = 9,
    BreathingSingle = 10,
}

impl LightingMode {
    pub const ALL: [Self; 11] = [
        Self::Off,
        Self::Glorious,
        Self::Single,
        Self::Breathing,
        Self::Tail,
        Self::SeamlessBreathing,
        Self::Gradient,
        Self::Rave,
        Self::Glitch,
        Self::Wave,
        Self::BreathingSingle,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Glorious => "glorious",
            Self::Single => "single",
            Self::Breathing => "breathing",
            Self::Tail => "tail",
            Self::SeamlessBreathing => "seamless-breathing",
            Self::Gradient => "gradient",
            Self::Rave => "rave",
            Self::Glitch => "glitch",
            Self::Wave => "wave",
            Self::BreathingSingle => "breathing-single",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    fn from_byte(b: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| *m as u8 == b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down = 0,
    Up = 1,
}

// Per-mode settings. Brightness is 1-4 (25/50/75/100%), speed is 1-3 in Glorious Core but
// the firmware goes up to MAX_SPEED (anything higher shows as a solid colour).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glorious {
    pub speed: u8,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Single {
    pub brightness: u8,
    pub color: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breathing {
    pub brightness: u8,
    pub speed: u8,
    /// How many of `colors` the effect cycles through (1-7).
    pub count: u8,
    pub colors: [Rgb; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tail {
    pub brightness: u8,
    pub speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seamless {
    pub speed: u8,
}

/// Static gradient along the LED strip. The record has 10 colour slots, only 6 are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gradient {
    pub colors: [Rgb; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rave {
    pub brightness: u8,
    pub speed: u8,
    pub colors: [Rgb; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glitch {
    pub speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wave {
    pub brightness: u8,
    pub speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreathingSingle {
    pub speed: u8,
    pub color: Rgb,
}

/// Every lighting mode keeps its own settings, even while another mode is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lighting {
    pub mode: LightingMode,
    /// Master switch (byte 130 bit 1). Off keeps the mode; the DPI light is unaffected.
    pub enabled: bool,
    pub glorious: Glorious,
    pub single: Single,
    pub breathing: Breathing,
    pub tail: Tail,
    pub seamless: Seamless,
    pub gradient: Gradient,
    pub rave: Rave,
    pub glitch: Glitch,
    pub wave: Wave,
    pub breathing_single: BreathingSingle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The bytes this was parsed from; [`Config::to_bytes`] writes the fields on top.
    raw: Vec<u8>,
    pub polling_rate: PollingRate,
    pub xy_independent: bool,
    /// 1-based. If it points at a disabled stage, the first enabled stage is used on write.
    pub active_stage: u8,
    pub stages: [DpiStage; STAGES],
    pub lift_off: LiftOff,
    pub lighting: Lighting,
}

fn high(b: u8) -> u8 {
    b >> 4
}

fn low(b: u8) -> u8 {
    b & 0x0f
}

fn set_high(b: &mut u8, v: u8) {
    *b = (v & 0x0f) << 4 | (*b & 0x0f);
}

fn set_low(b: &mut u8, v: u8) {
    *b = (*b & 0xf0) | (v & 0x0f);
}

fn dpi_from_byte(v: u8) -> u16 {
    (v as u16 + 1) * 100
}

fn dpi_to_byte(dpi: u16) -> u8 {
    (dpi / 100).saturating_sub(1).min(0xff) as u8
}

impl Config {
    /// Parses the bytes returned by the mouse (at least [`CONFIG_LEN`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < CONFIG_LEN {
            return Err(Error::Parse(format!(
                "config is {} bytes, expected {CONFIG_LEN}",
                bytes.len()
            )));
        }
        if bytes[0] != 0x04 || bytes[1] != 0x11 {
            return Err(Error::Parse(format!(
                "config header is {:02x} {:02x}, expected 04 11",
                bytes[0], bytes[1]
            )));
        }
        let b = &bytes[..CONFIG_LEN];

        let polling_rate = PollingRate::from_byte(b[POLLING] & !XY_INDEPENDENT)
            .ok_or_else(|| Error::Parse(format!("polling rate byte {:02x}", b[POLLING])))?;
        let xy_independent = b[POLLING] & XY_INDEPENDENT != 0;

        let stages = std::array::from_fn(|i| {
            let (x, y) = if xy_independent {
                (b[DPI + i * 2], b[DPI + i * 2 + 1])
            } else {
                (b[DPI + i], b[DPI + i])
            };
            DpiStage {
                enabled: b[STAGE_MASK] & (1 << i) == 0,
                x: dpi_from_byte(x),
                y: dpi_from_byte(y),
                color: Rgb::read_rgb(b, STAGE_COLORS + i * 3),
            }
        });

        let lift_off = match b[LIFT_OFF] {
            1 => LiftOff::Mm2,
            2 => LiftOff::Mm3,
            other => return Err(Error::Parse(format!("lift-off byte {other:02x}"))),
        };

        let mode = LightingMode::from_byte(b[LIGHT_MODE])
            .ok_or_else(|| Error::Parse(format!("lighting mode {:02x}", b[LIGHT_MODE])))?;

        let lighting = Lighting {
            mode,
            enabled: b[FLAGS] & LIGHTS_OFF == 0,
            glorious: Glorious {
                speed: low(b[GLORIOUS]),
                direction: if b[GLORIOUS + 1] == 0 {
                    Direction::Down
                } else {
                    Direction::Up
                },
            },
            single: Single {
                brightness: high(b[SINGLE]),
                color: Rgb::read_rbg(b, SINGLE + 1),
            },
            breathing: Breathing {
                brightness: high(b[BREATHING]),
                speed: low(b[BREATHING]),
                count: b[BREATHING + 1],
                colors: std::array::from_fn(|i| Rgb::read_rbg(b, BREATHING + 2 + i * 3)),
            },
            tail: Tail {
                brightness: high(b[TAIL]),
                speed: low(b[TAIL]),
            },
            seamless: Seamless {
                speed: low(b[SEAMLESS]),
            },
            gradient: Gradient {
                colors: std::array::from_fn(|i| Rgb::read_rbg(b, GRADIENT + i * 3)),
            },
            rave: Rave {
                brightness: high(b[RAVE]),
                speed: low(b[RAVE]),
                colors: std::array::from_fn(|i| Rgb::read_rbg(b, RAVE + 1 + i * 3)),
            },
            glitch: Glitch {
                speed: low(b[GLITCH]),
            },
            wave: Wave {
                brightness: high(b[WAVE]),
                speed: low(b[WAVE]),
            },
            breathing_single: BreathingSingle {
                speed: low(b[BREATHING_SINGLE]),
                color: Rgb::read_rbg(b, BREATHING_SINGLE + 1),
            },
        };

        Ok(Self {
            raw: b.to_vec(),
            polling_rate,
            xy_independent,
            active_stage: high(b[ACTIVE_STAGE]),
            stages,
            lift_off,
            lighting,
        })
    }

    /// The 131 bytes to send back, as read (byte 3 is set to the write length on send).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = self.raw.clone();

        let xy = if self.xy_independent {
            XY_INDEPENDENT
        } else {
            0
        };
        b[POLLING] = xy | self.polling_rate as u8;

        // DPI: both layouts share 13..=28, the unused part is 00.
        b[DPI..DPI + STAGES * 2].fill(0);
        let mut mask = 0u8;
        for (i, stage) in self.stages.iter().enumerate() {
            if self.xy_independent {
                b[DPI + i * 2] = dpi_to_byte(stage.x);
                b[DPI + i * 2 + 1] = dpi_to_byte(stage.y);
            } else {
                b[DPI + i] = dpi_to_byte(stage.x);
            }
            if !stage.enabled {
                mask |= 1 << i;
            }
            stage.color.write_rgb(&mut b, STAGE_COLORS + i * 3);
        }
        b[STAGE_MASK] = mask;
        let enabled = self.stages.iter().filter(|s| s.enabled).count() as u8;
        b[ACTIVE_STAGE] = self.effective_active_stage() << 4 | enabled;

        b[LIFT_OFF] = self.lift_off as u8;

        let l = &self.lighting;
        b[LIGHT_MODE] = l.mode as u8;
        if l.enabled {
            b[FLAGS] &= !LIGHTS_OFF;
        } else {
            b[FLAGS] |= LIGHTS_OFF;
        }

        set_low(&mut b[GLORIOUS], l.glorious.speed);
        b[GLORIOUS + 1] = l.glorious.direction as u8;

        set_high(&mut b[SINGLE], l.single.brightness);
        l.single.color.write_rbg(&mut b, SINGLE + 1);

        set_high(&mut b[BREATHING], l.breathing.brightness);
        set_low(&mut b[BREATHING], l.breathing.speed);
        b[BREATHING + 1] = l.breathing.count;
        for (i, c) in l.breathing.colors.iter().enumerate() {
            c.write_rbg(&mut b, BREATHING + 2 + i * 3);
        }

        set_high(&mut b[TAIL], l.tail.brightness);
        set_low(&mut b[TAIL], l.tail.speed);

        set_low(&mut b[SEAMLESS], l.seamless.speed);

        for (i, c) in l.gradient.colors.iter().enumerate() {
            c.write_rbg(&mut b, GRADIENT + i * 3);
        }

        set_high(&mut b[RAVE], l.rave.brightness);
        set_low(&mut b[RAVE], l.rave.speed);
        for (i, c) in l.rave.colors.iter().enumerate() {
            c.write_rbg(&mut b, RAVE + 1 + i * 3);
        }

        set_low(&mut b[GLITCH], l.glitch.speed);

        set_high(&mut b[WAVE], l.wave.brightness);
        set_low(&mut b[WAVE], l.wave.speed);

        set_low(&mut b[BREATHING_SINGLE], l.breathing_single.speed);
        l.breathing_single
            .color
            .write_rbg(&mut b, BREATHING_SINGLE + 1);

        b
    }

    /// The active stage that will actually be written: `active_stage` if that stage is
    /// enabled, otherwise the first enabled one.
    pub fn effective_active_stage(&self) -> u8 {
        let i = self.active_stage as usize;
        if (1..=STAGES).contains(&i) && self.stages[i - 1].enabled {
            return self.active_stage;
        }
        self.stages
            .iter()
            .position(|s| s.enabled)
            .map_or(1, |i| i as u8 + 1)
    }

    /// Checks everything that would be rejected or misbehave on the mouse.
    pub fn validate(&self) -> Result<()> {
        let invalid = |msg: String| Err(Error::Invalid(msg));

        if !self.stages.iter().any(|s| s.enabled) {
            return invalid("at least one DPI stage must be enabled".into());
        }
        for (i, s) in self.stages.iter().enumerate().filter(|(_, s)| s.enabled) {
            for dpi in [s.x, s.y] {
                if !(MIN_DPI..=MAX_DPI).contains(&dpi) || !dpi.is_multiple_of(100) {
                    return invalid(format!(
                        "stage {}: {dpi} DPI must be a multiple of 100 in {MIN_DPI}-{MAX_DPI}",
                        i + 1
                    ));
                }
            }
            if !self.xy_independent && s.x != s.y {
                return invalid(format!(
                    "stage {}: X and Y differ but XY independent is off",
                    i + 1
                ));
            }
        }

        let l = &self.lighting;
        let brightness = [
            ("single", l.single.brightness),
            ("breathing", l.breathing.brightness),
            ("tail", l.tail.brightness),
            ("rave", l.rave.brightness),
            ("wave", l.wave.brightness),
        ];
        for (mode, value) in brightness {
            if !(1..=4).contains(&value) {
                return invalid(format!("{mode}: brightness must be 1-4 (25-100%)"));
            }
        }
        let speeds = [
            ("glorious", l.glorious.speed),
            ("breathing", l.breathing.speed),
            ("tail", l.tail.speed),
            ("seamless breathing", l.seamless.speed),
            ("rave", l.rave.speed),
            ("glitch", l.glitch.speed),
            ("wave", l.wave.speed),
            ("breathing single", l.breathing_single.speed),
        ];
        for (mode, value) in speeds {
            if !(1..=MAX_SPEED).contains(&value) {
                return invalid(format!("{mode}: speed must be 1-{MAX_SPEED}"));
            }
        }
        if !(1..=7).contains(&l.breathing.count) {
            return invalid("breathing: colour count must be 1-7".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real config read from a Model D (XY independent off, stages 1-4 enabled).
    const REAL: [u8; CONFIG_LEN] = [
        0x04, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x06, 0x04, 0x34, 0xf0, 0x03, 0x07,
        0x0f, 0x1f, 0x31, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
        0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0xff,
        0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x41, 0x00, 0x40, 0xff, 0x00, 0x00,
        0x42, 0x07, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0xff, 0xff, 0xff,
        0xff, 0x00, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0x42, 0x42, 0x40, 0xff, 0x00, 0x00, 0x00,
        0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfa,
        0x00, 0xff, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0x42, 0xff, 0x00, 0x00,
        0x00, 0xff, 0x00, 0x02, 0x42, 0x02, 0xff, 0x00, 0x00, 0x01, 0x00,
    ];

    #[test]
    fn round_trip_is_lossless() {
        let config = Config::from_bytes(&REAL).unwrap();
        assert_eq!(config.to_bytes(), REAL);
    }

    #[test]
    fn decodes_known_fields() {
        let c = Config::from_bytes(&REAL).unwrap();
        assert_eq!(c.polling_rate, PollingRate::Hz1000);
        assert!(!c.xy_independent);
        assert_eq!(c.active_stage, 3);
        let dpis: Vec<u16> = c.stages[..6].iter().map(|s| s.x).collect();
        assert_eq!(dpis, [400, 800, 1600, 3200, 5000, 10000]);
        let enabled: Vec<bool> = c.stages.iter().map(|s| s.enabled).collect();
        assert_eq!(
            enabled,
            [true, true, true, true, false, false, false, false]
        );
        assert_eq!(c.lift_off, LiftOff::Mm2);
        assert_eq!(c.lighting.mode, LightingMode::Glorious);
        assert!(c.lighting.enabled);
        // stored as ff 00 00 in R B G order
        assert_eq!(c.lighting.single.color, Rgb::new(0xff, 0x00, 0x00));
        assert_eq!(c.lighting.breathing.count, 7);
        c.validate().unwrap();
    }

    #[test]
    fn xy_layout_converts_both_ways() {
        let mut c = Config::from_bytes(&REAL).unwrap();
        c.xy_independent = true;
        let on = c.to_bytes();
        assert_eq!(on[POLLING], 0x84);
        assert_eq!(
            &on[13..25],
            [
                0x03, 0x03, 0x07, 0x07, 0x0f, 0x0f, 0x1f, 0x1f, 0x31, 0x31, 0x63, 0x63
            ]
        );

        let mut back = Config::from_bytes(&on).unwrap();
        back.xy_independent = false;
        assert_eq!(back.to_bytes(), REAL);
    }

    #[test]
    fn enabling_a_stage_updates_mask_and_count() {
        let mut c = Config::from_bytes(&REAL).unwrap();
        c.stages[6].enabled = true;
        c.stages[6].x = 800;
        c.stages[6].y = 800;
        let b = c.to_bytes();
        assert_eq!(b[STAGE_MASK], 0xb0);
        assert_eq!(b[ACTIVE_STAGE], 0x35);
        assert_eq!(b[19], 0x07);
    }

    #[test]
    fn disabling_the_active_stage_moves_it() {
        let mut c = Config::from_bytes(&REAL).unwrap();
        c.stages[2].enabled = false; // stage 3, the active one
        assert_eq!(c.to_bytes()[ACTIVE_STAGE], 0x13);
    }

    #[test]
    fn unused_nibbles_survive() {
        let mut c = Config::from_bytes(&REAL).unwrap();
        c.lighting.glorious.speed = 3; // byte 54 is 0x41, high nibble unused
        assert_eq!(c.to_bytes()[GLORIOUS], 0x43);
    }

    #[test]
    fn lights_switch_only_touches_its_bit() {
        let mut c = Config::from_bytes(&REAL).unwrap();
        c.lighting.enabled = false;
        assert_eq!(c.to_bytes()[FLAGS], 0x02);
    }
}

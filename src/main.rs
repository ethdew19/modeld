//! modeld: configure a Glorious Model D (USB 258A:0033, SinoWealth MCU) without Glorious Core.
//!
//! Reverse engineered by diffing configs before/after changes in Glorious Core, plus `set`
//! experiments. OpenRGB's SinowealthController covers the same protocol (lighting only).
//!
//! # Channels
//!
//! All on USB interface 1, which Windows splits into one device per HID collection. Each
//! report ID only works on the collection that owns it ("Incorrect function" otherwise).
//!
//! ```text
//!   Col05  report 5  (6 bytes)    command   05 11 00 00 00 00 = select the config
//!                                           05 01 00 00 00 00 = firmware version (OpenRGB)
//!   Col03  report 4  (520 bytes)  data      the config blob, always read/written whole
//! ```
//!
//! Read  = SET report 5 `05 11 ..` on Col05, then GET report 4 on Col03 (returns 131 bytes).
//! Write = SET report 4 on Col03: the 131 bytes with byte 3 = 0x7b, zero-padded to 520.
//!         Wait ~50 ms before reading again, or bytes 0x2C.. come back as zeros.
//!
//! Debounce is a write-only command on Col05: `05 1a <ms / 2> 00 00 00` (Glorious Core
//! sends it twice). Confirmed with captures at 4 ms (02) and 16 ms (08).
//!
//! Button mappings use command 0x12 the same way as the config: read = `05 12 ..` then
//! GET report 4 (88 bytes = 8 header + 20 slots x 4), write = `04 12 00 50 ..` (0x50 = 80).
//! Slot: [type] [value] 00 00. 11 xx = mouse button bit (01 left, 02 right, 04 middle,
//! 08 back, 10 forward), 21 mm kk 00 = keyboard key (kk = HID usage code, mm = modifiers?),
//! 41 00 = DPI button?, 50 01 = unused slot, 70 01 01 01 = macro (index/mode/repeat?).
//! Glorious Core's "button 4" is slot 5 (the side buttons are numbered the other way round).
//!
//! Profiles exist only in Glorious Core: switching profile just rewrites everything above,
//! with no profile-select command.
//!
//! Macros are write-only (reading with `05 30` makes the device stall). Glorious Core writes
//! report 4 `04 30 02 00 00 00 00 00`, then `01 00 <event count>`, then 3-byte events:
//!
//! ```text
//!   byte 0         byte 1      byte 2
//!   r ttt dddd     dddddddd    code        r = release, t = type (5 = key, 6 = delay/marker?)
//!                                          d = 12-bit delay in ms, code = HID key
//!
//!   "AB" recorded live:  60 db 02 | 50 4e 04 | d4 75 04 | 50 3e 05 | d3 8a 05 | e0 01 02
//!                        219 ms     A down     A up       B down     B up       end?
//!                                   78 ms      1141 ms    62 ms      906 ms
//! ```
//!
//! # Config blob (report 4, 131 bytes)
//!
//! ```text
//!  offset  0    1    2    3    4    5    6    7    8    9    10   11   12   13 ...
//!        ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬──────┐
//!        │ 04 │ 11 │ 00 │ 7b │ 00 │ 00 │ 00 │ 00 │ 64 │ 06 │ 04 │ 34 │ f0 │ DPI  │
//!        └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴──────┘
//!          │    │    │    │    └───────┬──────┘    │    │    │    │    │
//!          │    │    │    │            │           │    │    │    │    └ stage disable mask
//!          │    │    │    │            │           │    │    │    └ active stage | enabled count
//!          │    │    │    │            │           │    │    └ XY flag | polling rate
//!          │    │    │    │            │           │    └ unknown (06)
//!          │    │    │    │            │           └ unknown (64)
//!          │    │    │    │            └ header, unknown (offset/address?)
//!          │    │    │    └ payload length on write: 0x7b = 131 - 8 (00 on read)
//!          │    │    └ header, unknown
//!          │    └ command 0x11 (config), same as in report 5
//!          └ report ID
//! ```
//!
//! ## Performance
//!
//! ```text
//!  10        bit 7: XY independent DPI
//!            low bits: polling rate 01 = 125 Hz, 02 = 250, 03 = 500, 04 = 1000
//!
//!  11        [active stage (from 1) | number of enabled stages]     0x34 = on 3, 4 enabled
//!
//!  12        stage disable mask, bit n-1 set = stage n off           0xf0 = stages 5-8 off
//!
//!  13..=28   DPI per stage, stored value v → dpi = (v + 1) * 100 (max 0x77 = 12000)
//!            XY off: one byte per stage, 13..=20
//!            XY on:  X,Y pairs, stage n at 13 + (n-1)*2
//!
//!  29..=52   DPI light colour per stage, 8 x [R G B] (normal RGB, unlike lighting!)
//!
//!  129       lift-off distance: 01 = 2 mm, 02 = 3 mm
//!
//!  130       flags. bit 1 (0x02) = lighting effects off, independent of the mode (confirmed
//!            for 0-12). The DPI light still works. Other bits have no visible effect. Not exposed by Glorious Core,
//!            which uses lighting mode 0 for "off" instead.
//! ```
//!
//! The firmware has 8 stage slots; Glorious Core only shows 6. Stage 7/8 work once the
//! mask bit is cleared and the count in byte 11 matches.
//!
//! ## Lighting
//!
//! Byte 53 is the active mode. Every mode keeps its own record, even while inactive,
//! stored in mode order. Colours are R, B, G (not RGB!). Brightness/speed share a byte:
//! high nibble brightness 1-4 = 25/50/75/100%, low nibble speed (Glorious Core allows
//! 1-3, the firmware accepts more). "unused" = the mode ignores that half.
//!
//! ```text
//!  mode  bytes      name                     layout
//!  0     -          off                      (no record)
//!  1     54..=55    glorious (rainbow)       [unused | speed] [direction: 00 down, 01 up]
//!  2     56..=59    single colour            [brightness | unused] [R B G]
//!  3     60..=82    breathing                [brightness | speed] [count 1-7] [7 x R B G]
//!  4     83         tail                     [brightness | speed]
//!  5     84         seamless breathing       [unused | speed]
//!  6     85..=115   gradient (hidden)        [unused] [10 x R B G], first 6 used:
//!                                            static gradient along the strip
//!  7     116..=122  rave                     [brightness | speed] [R B G] [R B G]
//!  8     123        "epilepsy" (hidden)      [unused | speed]?
//!  9     124        wave                     [brightness | speed]
//!  10    125..=128  breathing single colour  [unused | speed] [R B G]
//! ```
//!
//! Modes 6 and 8 aren't in Glorious Core.
//!
//! ## Unknown
//!
//! Header bytes 2 and 4..=7, plus 8 (0x64) and 9 (0x06). Nothing in Glorious Core changes
//! them. Byte 130: only bit 1 is known (see above).

use hidapi::{HidApi, HidDevice};

use std::fmt;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum Error {
    /// A collection we need (Col03/Col05) isn't there, so the mouse is probably unplugged.
    NotFound(&'static str),
    /// hidapi failed to open or talk to the mouse.
    Hid(hidapi::HidError),
    /// Reading or writing backup.bin failed.
    Io(std::io::Error),
    /// A command-line argument wasn't a valid number.
    BadArg(String),
    StageOutOfRange(usize),
    OffsetOutOfRange(usize),
    /// A request that's well-formed but not allowed, e.g. disabling the last stage.
    Invalid(&'static str),
    DpiOutOfRange(u32),
    DpiNotMultipleOf100(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotFound(col) => write!(f, "Model D {col} not found, is it plugged in?"),
            Error::Hid(e) => write!(f, "HID error: {e}"),
            Error::Io(e) => write!(f, "{BACKUP}: {e}"),
            Error::BadArg(arg) => write!(f, "invalid argument: {arg:?}"),
            Error::Invalid(why) => write!(f, "{why}"),
            Error::OffsetOutOfRange(o) => {
                write!(f, "offset {o} is outside the config (4-130 are writable)")
            }
            Error::StageOutOfRange(s) => write!(f, "stage {s} doesn't exist, use 1-8"),
            Error::DpiOutOfRange(d) => write!(f, "{d} DPI is out of range"),
            Error::DpiNotMultipleOf100(d) => write!(f, "{d} DPI isn't a multiple of 100"),
        }
    }
}

impl std::error::Error for Error {}

// These let `?` convert library errors into ours automatically.
impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Error::Hid(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Parses "54", "0x36" style numbers.
fn parse_num<T: TryFrom<u32>>(arg: &str) -> Result<T> {
    let n = match arg.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => arg.parse(),
    };
    n.ok()
        .and_then(|n| T::try_from(n).ok())
        .ok_or_else(|| Error::BadArg(arg.to_string()))
}

/// Parses "rrggbb" (optionally with a leading '#') into (r, g, b).
fn parse_color(arg: &str) -> Result<(u8, u8, u8)> {
    let hex = arg.trim_start_matches('#');
    let channel = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .ok_or_else(|| Error::BadArg(arg.to_string()))
    };
    if hex.len() != 6 {
        return Err(Error::BadArg(arg.to_string()));
    }
    Ok((channel(0)?, channel(2)?, channel(4)?))
}

fn parse<T: std::str::FromStr>(arg: &str) -> Result<T> {
    arg.parse().map_err(|_| Error::BadArg(arg.to_string()))
}

const VID: u16 = 0x258A;
const PID: u16 = 0x0033;
const BACKUP: &str = "backup.bin";

/// Byte offset of DPI stage 1. The layout after it depends on XY_INDEPENDENT:
/// off = one byte per stage (13..=18), on = X,Y pairs (13/14 = stage 1, 15/16 = stage 2, ...).
/// The firmware has room for 8 stages (13..=28 in pair layout), the Model D uses 6.
const DPI_STAGES: usize = 13;
/// The firmware has 8 stage slots; Glorious Core only shows 6.
const STAGE_COUNT: usize = 8;
/// Byte offset of [active stage | enabled count].
const ACTIVE_STAGE: usize = 11;
/// Byte offset of the stage disable mask: bit n-1 set = stage n off.
const STAGE_MASK: usize = 12;
/// Byte offset of stage 1's DPI light colour (R G B), 3 bytes per stage.
const STAGE_COLORS: usize = 29;
/// Bit 7 of the polling rate byte: X and Y DPI are set separately.
const XY_INDEPENDENT: u8 = 0x80;
/// Byte offset of the lift-off distance.
const LOD: usize = 129;
/// Byte offset of polling rate.
const POLLING_RATE: usize = 10;

/// Byte offset of the flags byte; bit 1 turns the lighting effects off.
const FLAGS: usize = 130;
const LIGHTS_OFF: u8 = 0x02;

/// Byte offset of the active lighting mode.
const LIGHT_MODE: usize = 53;

/// Byte offset of the lighting direction. Probably belongs to the current lighting mode's
/// record (with the speed nibble at 54), so other modes may keep theirs elsewhere.
const LIGHT_DIRECTION: usize = 55;

/// Which way the lighting effect moves.
#[derive(Debug, Clone, Copy)]
enum Direction {
    Down = 0x00,
    Up = 0x01,
}

impl std::str::FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "down" => Ok(Direction::Down),
            "up" => Ok(Direction::Up),
            _ => Err(()),
        }
    }
}

/// Byte offset of the single-colour mode's [brightness | unused] byte.
const SINGLE_BRIGHTNESS: usize = 56;

/// Lighting brightness, stored in the high nibble of a mode's first byte.
#[derive(Debug, Clone, Copy)]
enum Brightness {
    Pct25 = 1,
    Pct50 = 2,
    Pct75 = 3,
    Pct100 = 4,
}

impl std::str::FromStr for Brightness {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s.trim_end_matches('%') {
            "25" => Ok(Brightness::Pct25),
            "50" => Ok(Brightness::Pct50),
            "75" => Ok(Brightness::Pct75),
            "100" => Ok(Brightness::Pct100),
            _ => Err(()),
        }
    }
}

/// Lift-off distance: how high you can lift the mouse before it stops tracking.
#[derive(Debug, Clone, Copy)]
enum Lod {
    Mm2 = 0x01,
    Mm3 = 0x02,
}

impl std::str::FromStr for Lod {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "2" | "2mm" => Ok(Lod::Mm2),
            "3" | "3mm" => Ok(Lod::Mm3),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PollingRate {
    Hz125 = 0b001,
    Hz250 = 0b010,
    Hz500 = 0b011,
    Hz1000 = 0b100,
}
impl std::str::FromStr for PollingRate {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "125" => Ok(PollingRate::Hz125),
            "250" => Ok(PollingRate::Hz250),
            "500" => Ok(PollingRate::Hz500),
            "1000" => Ok(PollingRate::Hz1000),
            _ => Err(()),
        }
    }
}
const USAGE: &str = "usage:
  modeld dump                 print the mouse's config
  modeld dpi <stage> <dpi> [y]  set a DPI stage (stage 1-8, steps of 100); a different y
                                turns on XY independent
  modeld xy <on|off>          XY independent DPI (off keeps each stage's X)
  modeld stage <n> <on|off>   enable/disable a DPI stage (1-8; 7 and 8 are hidden in Glorious Core)
  modeld stage-color <n> <rrggbb>  DPI light colour for a stage
  modeld lod <2mm|3mm>        set lift-off distance
  modeld restore [file]       write a saved config back to the mouse (default backup.bin)
  modeld lights <on|off>      master switch for the lighting, keeps the mode and colours
  modeld buttons              show the button mapping table
  modeld debounce <ms>        set debounce time (even values, 2-16 ms)
  modeld light-dir <up|down>  set lighting direction
  modeld single-brightness <25|50|75|100>   set single-colour mode brightness
  modeld zone-test <1-10>     mode 6 experiment: only colour n white, the rest black
  modeld set <offset> <value> poke one raw byte (decimal or 0x hex), for experiments
  modeld save <file>          save the current config to a file
  modeld diff [file]          show bytes that differ from a saved config (default backup.bin)";

struct ModelD {
    cmd: HidDevice,  // Col05, report 5
    data: HidDevice, // Col03, report 4
}

impl ModelD {
    fn open(api: &HidApi) -> Result<Self> {
        let find = |col: &'static str| {
            api.device_list()
                .find(|d| {
                    d.vendor_id() == VID
                        && d.product_id() == PID
                        && d.path().to_string_lossy().contains(col)
                })
                .ok_or(Error::NotFound(col))
        };
        Ok(Self {
            cmd: find("Col05")?.open_device(api)?,
            data: find("Col03")?.open_device(api)?,
        })
    }

    fn read_config(&self) -> Result<Vec<u8>> {
        self.cmd.send_feature_report(&[0x05, 0x11, 0, 0, 0, 0])?;
        let mut buf = [0u8; 520];
        buf[0] = 0x04; // tell hidapi which report you want
        let n = self.data.get_feature_report(&mut buf)?;
        Ok(buf[..n].to_vec())
    }

    /// Reads the button table: like read_config, but with command 0x12.
    fn read_buttons(&self) -> Result<Vec<u8>> {
        self.cmd.send_feature_report(&[0x05, 0x12, 0, 0, 0, 0])?;
        let mut buf = [0u8; 520];
        buf[0] = 0x04;
        let n = self.data.get_feature_report(&mut buf)?;
        Ok(buf[..n].to_vec())
    }

    fn write_config(&self, config: &[u8]) -> Result<()> {
        let mut out = [0u8; 520];
        out[..config.len()].copy_from_slice(config);
        out[3] = 0x7b; // payload length (131 - 8 header bytes), from the Glorious Core capture
        self.data.send_feature_report(&out)?;
        // Reading right after a write returns zeros from offset 0x2C onward (found by OpenRGB).
        // Give the firmware time to settle, so the next read - and anything written from
        // it - doesn't pick up those zeros.
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }

    fn set_lod(&self, lod: Lod) -> Result<()> {
        let mut config = self.read_config()?;

        config[LOD] = lod as u8;
        self.write_config(&config)?;
        Ok(())
    }

    /// Writes one raw byte. Bytes 0-3 are the header (report ID, command, write marker),
    /// so those are off limits.
    fn set_raw(&self, offset: usize, value: u8) -> Result<()> {
        let mut config = self.read_config()?;
        if !(4..config.len()).contains(&offset) {
            return Err(Error::OffsetOutOfRange(offset));
        }
        config[offset] = value;
        self.write_config(&config)
    }

    fn set_light_direction(&self, direction: Direction) -> Result<()> {
        let mut config = self.read_config()?;
        config[LIGHT_DIRECTION] = direction as u8;
        self.write_config(&config)
    }

    fn set_single_brightness(&self, brightness: Brightness) -> Result<()> {
        let mut config = self.read_config()?;
        // Replace only the high nibble, keep whatever is in the low one.
        let low = config[SINGLE_BRIGHTNESS] & 0x0f;
        config[SINGLE_BRIGHTNESS] = (brightness as u8) << 4 | low;
        self.write_config(&config)
    }

    /// Experiment for the hidden mode 6: switch to it, black out all 10 colours
    /// (bytes 86..=115, R B G each), then make colour `n` white.
    fn zone_test(&self, n: usize) -> Result<()> {
        if !(1..=10).contains(&n) {
            return Err(Error::BadArg(n.to_string()));
        }
        let mut config = self.read_config()?;
        config[LIGHT_MODE] = 6;
        config[86..=115].fill(0x00);
        let start = 86 + (n - 1) * 3;
        config[start..start + 3].fill(0xff);
        self.write_config(&config)
    }

    /// Sets one stage's DPI. Without `y`, X and Y get the same value. A `y` that differs
    /// from `x` switches XY independent on (converting the other stages) if it isn't already.
    fn set_dpi(&self, stage: usize, x: u32, y: Option<u32>) -> Result<()> {
        if !(1..=STAGE_COUNT).contains(&stage) {
            return Err(Error::StageOutOfRange(stage));
        }
        let x = dpi_to_byte(x)?;
        let y = y.map(dpi_to_byte).transpose()?.unwrap_or(x);

        let mut config = self.read_config()?;
        let mut stages = read_stages(&config);
        stages[stage - 1] = (x, y);
        let xy = x != y || config[POLLING_RATE] & XY_INDEPENDENT != 0;
        write_stages(&mut config, &stages, xy);
        self.write_config(&config)
    }

    /// Turns XY independent on or off, converting every stage to the other layout.
    /// Turning it off keeps each stage's X value.
    /// Enables or disables a stage: flips its bit in the mask and updates the enabled count.
    /// If the active stage gets disabled, the first enabled stage becomes active.
    fn set_stage_enabled(&self, stage: usize, on: bool) -> Result<()> {
        if !(1..=STAGE_COUNT).contains(&stage) {
            return Err(Error::StageOutOfRange(stage));
        }
        let mut config = self.read_config()?;
        let bit = 1u8 << (stage - 1);
        if on {
            config[STAGE_MASK] &= !bit;
        } else {
            config[STAGE_MASK] |= bit;
        }

        // Enabled stages are the 0 bits in the mask.
        let enabled = (!config[STAGE_MASK]).count_ones() as u8;
        if enabled == 0 {
            return Err(Error::Invalid("can't disable the last enabled stage"));
        }

        let mut active = config[ACTIVE_STAGE] >> 4;
        if config[STAGE_MASK] & (1 << (active - 1)) != 0 {
            active = (!config[STAGE_MASK]).trailing_zeros() as u8 + 1;
        }
        config[ACTIVE_STAGE] = active << 4 | enabled;
        self.write_config(&config)
    }

    /// Sets a stage's DPI light colour. Stored in normal R G B order (unlike lighting modes).
    fn set_stage_color(&self, stage: usize, (r, g, b): (u8, u8, u8)) -> Result<()> {
        if !(1..=STAGE_COUNT).contains(&stage) {
            return Err(Error::StageOutOfRange(stage));
        }
        let mut config = self.read_config()?;
        let at = STAGE_COLORS + (stage - 1) * 3;
        config[at..at + 3].copy_from_slice(&[r, g, b]);
        self.write_config(&config)
    }

    /// Hidden master switch: turns the lighting effects off without changing the mode.
    fn set_lights(&self, on: bool) -> Result<()> {
        let mut config = self.read_config()?;
        if on {
            config[FLAGS] &= !LIGHTS_OFF;
        } else {
            config[FLAGS] |= LIGHTS_OFF;
        }
        self.write_config(&config)
    }

    /// Debounce isn't part of the config blob: it's its own command with ms / 2 in byte 2.
    /// Glorious Core sends it twice, so we do too.
    fn set_debounce(&self, ms: u8) -> Result<()> {
        if !(2..=16).contains(&ms) || ms % 2 != 0 {
            return Err(Error::Invalid("debounce must be an even number of ms, 2-16"));
        }
        for _ in 0..2 {
            self.cmd.send_feature_report(&[0x05, 0x1a, ms / 2, 0, 0, 0])?;
        }
        Ok(())
    }

    fn set_xy_independent(&self, on: bool) -> Result<()> {
        let mut config = self.read_config()?;
        let stages = read_stages(&config);
        write_stages(&mut config, &stages, on);
        self.write_config(&config)
    }

    fn set_polling_rate(&self, polling_rate: PollingRate) -> Result<()> {
        let mut config = self.read_config()?;

        // Bit 7 is used for XY independent dpi flag, so we need to & this
        config[POLLING_RATE] = (config[POLLING_RATE] & XY_INDEPENDENT) | polling_rate as u8;
        self.write_config(&config)?;
        Ok(())
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let api = HidApi::new()?;
    let mouse = ModelD::open(&api)?;

    // Keep the first config we ever read, as something to restore if a write goes wrong.
    if !std::path::Path::new(BACKUP).exists() {
        std::fs::write(BACKUP, mouse.read_config()?)?;
        println!("saved backup to {BACKUP}");
    }

    match args.as_slice() {
        ["poll", rate] => mouse.set_polling_rate(parse(rate)?)?,
        ["dump"] => {}
        ["dpi", stage, dpi] => mouse.set_dpi(parse(stage)?, parse(dpi)?, None)?,
        ["dpi", stage, x, y] => mouse.set_dpi(parse(stage)?, parse(x)?, Some(parse(y)?))?,
        ["buttons"] => {
            print_buttons(&mouse.read_buttons()?);
            return Ok(());
        }
        ["debounce", ms] => {
            mouse.set_debounce(parse(ms)?)?;
            // Not in the config blob, so dumping it afterwards wouldn't show anything.
            println!("debounce set to {ms} ms");
            return Ok(());
        }
        ["lights", "on"] => mouse.set_lights(true)?,
        ["lights", "off"] => mouse.set_lights(false)?,
        ["xy", "on"] => mouse.set_xy_independent(true)?,
        ["xy", "off"] => mouse.set_xy_independent(false)?,
        ["stage", n, "on"] => mouse.set_stage_enabled(parse(n)?, true)?,
        ["stage", n, "off"] => mouse.set_stage_enabled(parse(n)?, false)?,
        ["stage-color", n, color] => mouse.set_stage_color(parse(n)?, parse_color(color)?)?,
        ["lod", lod] => mouse.set_lod(parse(lod)?)?,
        ["restore"] => mouse.write_config(&std::fs::read(BACKUP)?)?,
        ["restore", file] => mouse.write_config(&std::fs::read(file)?)?,
        ["light-dir", dir] => mouse.set_light_direction(parse(dir)?)?,
        ["single-brightness", b] => mouse.set_single_brightness(parse(b)?)?,
        ["zone-test", n] => mouse.zone_test(parse(n)?)?,
        ["set", offset, value] => mouse.set_raw(parse_num(offset)?, parse_num(value)?)?,
        ["save", file] => {
            std::fs::write(file, mouse.read_config()?)?;
            println!("saved to {file}");
            return Ok(());
        }
        ["diff"] => return diff(&std::fs::read(BACKUP)?, &mouse.read_config()?),
        ["diff", file] => return diff(&std::fs::read(file)?, &mouse.read_config()?),
        _ => {
            println!("{USAGE}");
            return Ok(());
        }
    }

    // Always show the config afterwards, so every write is verified by a fresh read.
    hexdump(&mouse.read_config()?);
    Ok(())
}

/// DPI → stored byte: (value + 1) * 100 = dpi.
fn dpi_to_byte(dpi: u32) -> Result<u8> {
    if !(400..=12000).contains(&dpi) {
        return Err(Error::DpiOutOfRange(dpi));
    }
    if dpi % 100 != 0 {
        return Err(Error::DpiNotMultipleOf100(dpi));
    }
    Ok((dpi / 100 - 1) as u8)
}

/// Reads every stage as (x, y) stored bytes, whichever layout the config is in.
fn read_stages(config: &[u8]) -> [(u8, u8); STAGE_COUNT] {
    let xy = config[POLLING_RATE] & XY_INDEPENDENT != 0;
    std::array::from_fn(|i| {
        if xy {
            (config[DPI_STAGES + i * 2], config[DPI_STAGES + i * 2 + 1])
        } else {
            let v = config[DPI_STAGES + i];
            (v, v)
        }
    })
}

/// Writes every stage in the layout for `xy`, sets the flag to match, and zeroes the
/// rest of the DPI area (the firmware leaves unused slots at 00).
fn write_stages(config: &mut [u8], stages: &[(u8, u8); STAGE_COUNT], xy: bool) {
    config[DPI_STAGES..DPI_STAGES + 16].fill(0);
    for (i, &(x, y)) in stages.iter().enumerate() {
        if xy {
            config[DPI_STAGES + i * 2] = x;
            config[DPI_STAGES + i * 2 + 1] = y;
        } else {
            config[DPI_STAGES + i] = x;
        }
    }
    if xy {
        config[POLLING_RATE] |= XY_INDEPENDENT;
    } else {
        config[POLLING_RATE] &= !XY_INDEPENDENT;
    }
}

/// Prints the button table: 20 slots of [type] [value] [?] [?] after the 8-byte header.
fn print_buttons(table: &[u8]) {
    for (i, slot) in table[8..].chunks(4).enumerate() {
        let meaning = match slot {
            [0x11, 0x01, ..] => "left click".to_string(),
            [0x11, 0x02, ..] => "right click".to_string(),
            [0x11, 0x04, ..] => "middle click".to_string(),
            [0x11, 0x08, ..] => "back".to_string(),
            [0x11, 0x10, ..] => "forward".to_string(),
            [0x21, mods, key, ..] => match mods {
                0 => format!("key {}", key_name(*key)),
                _ => format!("key {} + modifiers {mods:#04x}?", key_name(*key)),
            },
            [0x41, 0x00, ..] => "DPI button?".to_string(),
            [0x70, a, b, c] => format!("macro (index? {a}, mode? {b}, repeat? {c})"),
            [0x50, 0x01, ..] => "(unused slot)".to_string(),
            _ => "?".to_string(),
        };
        let hex: Vec<String> = slot.iter().map(|b| format!("{b:02x}")).collect();
        println!("button {:2}  {}   {meaning}", i + 1, hex.join(" "));
    }
}

/// Name of a USB HID keyboard usage code (HID Usage Tables, page 0x07).
fn key_name(code: u8) -> String {
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

/// Prints every byte that differs between two configs, in hex and binary
/// (binary because a lot of these settings are packed into bits).
fn diff(old: &[u8], new: &[u8]) -> Result<()> {
    let mut changed = 0;
    println!("offset    old             new");
    for i in 0..old.len().max(new.len()) {
        let (a, b) = (old.get(i), new.get(i));
        if a == b {
            continue;
        }
        let show = |x: Option<&u8>| match x {
            Some(x) => format!("{x:02x} {x:08b}"),
            None => "--".to_string(),
        };
        println!("{i:3} 0x{i:02x}  {}     {}", show(a), show(b));
        changed += 1;
    }
    if changed == 0 {
        println!("(no differences)");
    }
    Ok(())
}

/// Prints bytes like Wireshark: offset, 16 hex bytes, then ASCII.
fn hexdump(bytes: &[u8]) {
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect();
        println!("{:04x}   {:<47}   {ascii}", i * 16, hex.join(" "));
    }
}

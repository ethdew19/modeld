//! `modeld`: configure a Glorious Model D from the command line.

use std::fmt;
use std::process::ExitCode;

use modeld_core::{
    Config, Direction, LiftOff, LightingMode, ModelD, PHYSICAL_BUTTONS, PollingRate, Rgb, STAGES,
};

const USAGE: &str = "\
usage: modeld <command>

  show                          print the settings
  dpi <stage> <dpi> [y-dpi]     set a stage's DPI (stage 1-8, steps of 100);
                                a different Y turns on XY independent
  xy <on|off>                   XY independent DPI (off keeps each stage's X)
  stage <n> <on|off>            enable/disable a DPI stage (7 and 8 are hidden in the official software)
  stage-color <n> <rrggbb>      DPI light colour for a stage
  active <n>                    switch to a DPI stage
  poll <125|250|500|1000>       polling rate in Hz
  lod <2|3>                     lift-off distance in mm
  debounce <ms>                 debounce time (even, 4-16 ms; write-only)
  lights <on|off>               lighting master switch (keeps the mode)
  mode <name>                   lighting mode: off, glorious, single, breathing, tail,
                                seamless-breathing, gradient, rave, glitch, wave,
                                breathing-single
  color <rrggbb>                colour for single / breathing-single mode
  buttons                       print the button mapping

raw access, for experiments:
  dump                          hex dump of the config
  save <file>                   save the raw config to a file
  restore <file>                write a saved config back
  diff <file>                   bytes that differ from a saved config
  set <offset> <value>          write one byte (decimal or 0x hex)";

enum CliError {
    Core(modeld_core::Error),
    Io(std::io::Error),
    BadArg(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::Core(e) => write!(f, "{e}"),
            CliError::Io(e) => write!(f, "{e}"),
            CliError::BadArg(arg) => write!(f, "invalid argument: {arg:?}"),
        }
    }
}

impl From<modeld_core::Error> for CliError {
    fn from(e: modeld_core::Error) -> Self {
        CliError::Core(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

type Result<T> = std::result::Result<T, CliError>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    if args.is_empty() || matches!(args[0], "help" | "-h" | "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[&str]) -> Result<()> {
    let mouse = ModelD::open()?;

    // Commands that change the typed config: read, modify, write, show.
    let modify = |f: &dyn Fn(&mut Config) -> Result<()>| -> Result<()> {
        let mut config = mouse.read_config()?;
        f(&mut config)?;
        mouse.write_config(&config)?;
        print_config(&mouse.read_config()?);
        Ok(())
    };

    match args {
        ["show"] => print_config(&mouse.read_config()?),

        ["dpi", stage, dpi] => modify(&|c| {
            let s = stage_mut(c, stage)?;
            let dpi = num(dpi)?;
            (s.x, s.y) = (dpi, dpi);
            Ok(())
        })?,
        ["dpi", stage, x, y] => modify(&|c| {
            let (x, y) = (num(x)?, num(y)?);
            if x != y {
                c.xy_independent = true;
            }
            let s = stage_mut(c, stage)?;
            (s.x, s.y) = (x, y);
            Ok(())
        })?,
        ["xy", on] => modify(&|c| {
            c.xy_independent = on_off(on)?;
            if !c.xy_independent {
                for s in &mut c.stages {
                    s.y = s.x;
                }
            }
            Ok(())
        })?,
        ["stage", n, on] => modify(&|c| {
            stage_mut(c, n)?.enabled = on_off(on)?;
            Ok(())
        })?,
        ["stage-color", n, color] => modify(&|c| {
            stage_mut(c, n)?.color = rgb(color)?;
            Ok(())
        })?,
        ["active", n] => modify(&|c| {
            stage_mut(c, n)?;
            c.active_stage = num(n)?;
            Ok(())
        })?,
        ["poll", hz] => modify(&|c| {
            c.polling_rate = PollingRate::from_hz(num(hz)?).ok_or(bad(hz))?;
            Ok(())
        })?,
        ["lod", mm] => modify(&|c| {
            c.lift_off = LiftOff::from_mm(num(mm.trim_end_matches("mm"))?).ok_or(bad(mm))?;
            Ok(())
        })?,
        ["lights", on] => modify(&|c| {
            c.lighting.enabled = on_off(on)?;
            Ok(())
        })?,
        ["mode", name] => modify(&|c| {
            c.lighting.mode = LightingMode::from_name(name).ok_or(bad(name))?;
            Ok(())
        })?,
        ["color", color] => modify(&|c| {
            let color = rgb(color)?;
            match c.lighting.mode {
                LightingMode::Single => c.lighting.single.color = color,
                LightingMode::BreathingSingle => c.lighting.breathing_single.color = color,
                _ => return Err(bad("color only applies to single / breathing-single mode")),
            }
            Ok(())
        })?,

        ["debounce", ms] => {
            mouse.set_debounce(num(ms)?)?;
            println!("debounce set to {ms} ms");
        }
        ["buttons"] => {
            let buttons = mouse.read_buttons()?;
            for (i, action) in buttons.slots.iter().enumerate() {
                let name = PHYSICAL_BUTTONS.get(i).copied().unwrap_or("-");
                println!("slot {:2}  {name:<20}  {}", i + 1, action.describe());
            }
        }

        ["dump"] => hexdump(&mouse.read_config_raw()?),
        ["save", file] => {
            std::fs::write(file, mouse.read_config_raw()?)?;
            println!("saved to {file}");
        }
        ["restore", file] => {
            mouse.write_config_raw(&std::fs::read(file)?)?;
            hexdump(&mouse.read_config_raw()?);
        }
        ["diff", file] => diff(&std::fs::read(file)?, &mouse.read_config_raw()?),
        ["set", offset, value] => {
            let (offset, value): (usize, u8) = (num_hex(offset)?, num_hex(value)?);
            let mut raw = mouse.read_config_raw()?;
            // Bytes 0-3 are the header (report ID, command, ?, length).
            if !(4..raw.len()).contains(&offset) {
                return Err(bad(&offset.to_string()));
            }
            raw[offset] = value;
            mouse.write_config_raw(&raw)?;
            hexdump(&mouse.read_config_raw()?);
        }

        _ => {
            println!("{USAGE}");
        }
    }
    Ok(())
}

fn bad(arg: &str) -> CliError {
    CliError::BadArg(arg.to_string())
}

fn num<T: std::str::FromStr>(arg: &str) -> Result<T> {
    arg.parse().map_err(|_| bad(arg))
}

/// Parses "54" or "0x36".
fn num_hex<T: TryFrom<u32>>(arg: &str) -> Result<T> {
    let n = match arg.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => arg.parse().ok(),
    };
    n.and_then(|n| T::try_from(n).ok()).ok_or_else(|| bad(arg))
}

fn on_off(arg: &str) -> Result<bool> {
    match arg {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(bad(arg)),
    }
}

fn rgb(arg: &str) -> Result<Rgb> {
    Rgb::from_hex(arg).ok_or_else(|| bad(arg))
}

fn stage_mut<'a>(c: &'a mut Config, arg: &str) -> Result<&'a mut modeld_core::DpiStage> {
    let n: usize = num(arg)?;
    if !(1..=STAGES).contains(&n) {
        return Err(bad(arg));
    }
    Ok(&mut c.stages[n - 1])
}

fn print_config(c: &Config) {
    println!("polling rate   {} Hz", c.polling_rate.hz());
    println!("lift-off       {} mm", c.lift_off.mm());
    println!(
        "XY independent {}",
        if c.xy_independent { "on" } else { "off" }
    );
    println!("DPI stages");
    for (i, s) in c.stages.iter().enumerate() {
        let active = if i + 1 == c.effective_active_stage() as usize {
            ">"
        } else {
            " "
        };
        let state = if s.enabled { "on " } else { "off" };
        let dpi = if c.xy_independent {
            format!("{} x {}", s.x, s.y)
        } else {
            s.x.to_string()
        };
        println!(
            "  {active} {}  {state}  {dpi:>11} DPI   #{}",
            i + 1,
            s.color.to_hex()
        );
    }

    let l = &c.lighting;
    let pct = |b: u8| b as u16 * 25;
    let colors = |cs: &[Rgb]| {
        cs.iter()
            .map(|c| format!("#{}", c.to_hex()))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mode = match l.mode {
        LightingMode::Off => String::new(),
        LightingMode::Glorious => format!(
            "speed {}, {}",
            l.glorious.speed,
            if l.glorious.direction == Direction::Up {
                "up"
            } else {
                "down"
            }
        ),
        LightingMode::Single => format!(
            "{}%, #{}",
            pct(l.single.brightness),
            l.single.color.to_hex()
        ),
        LightingMode::Breathing => format!(
            "{}%, speed {}, {}",
            pct(l.breathing.brightness),
            l.breathing.speed,
            colors(&l.breathing.colors[..l.breathing.count.clamp(1, 7) as usize])
        ),
        LightingMode::Tail => format!("{}%, speed {}", pct(l.tail.brightness), l.tail.speed),
        LightingMode::SeamlessBreathing => format!("speed {}", l.seamless.speed),
        LightingMode::Gradient => colors(&l.gradient.colors),
        LightingMode::Rave => format!(
            "{}%, speed {}, {}",
            pct(l.rave.brightness),
            l.rave.speed,
            colors(&l.rave.colors)
        ),
        LightingMode::Glitch => format!("speed {}", l.glitch.speed),
        LightingMode::Wave => format!("{}%, speed {}", pct(l.wave.brightness), l.wave.speed),
        LightingMode::BreathingSingle => format!(
            "speed {}, #{}",
            l.breathing_single.speed,
            l.breathing_single.color.to_hex()
        ),
    };
    let off = if l.enabled {
        ""
    } else {
        "  (lights switched off)"
    };
    println!("lighting       {} {mode}{off}", l.mode.name());
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

/// Prints every byte that differs, in hex and binary (many settings are packed into bits).
fn diff(old: &[u8], new: &[u8]) {
    println!("offset    old             new");
    let mut changed = false;
    for i in 0..old.len().max(new.len()) {
        let (a, b) = (old.get(i), new.get(i));
        if a == b {
            continue;
        }
        let show = |x: Option<&u8>| x.map_or("--".to_string(), |x| format!("{x:02x} {x:08b}"));
        println!("{i:3} 0x{i:02x}  {}     {}", show(a), show(b));
        changed = true;
    }
    if !changed {
        println!("(no differences)");
    }
}

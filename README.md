# modeld

Configure a **Glorious Model D** (wired, USB `258A:0033`) without Glorious Core: a Rust library,
a command-line tool and a GUI. It talks to the mouse's firmware over HID feature reports (via
[hidapi](https://crates.io/crates/hidapi)), so you don't need a driver or admin rights.

The protocol was reverse engineered with help from Claude and is documented in
[PROTOCOL.md](PROTOCOL.md), including some things Glorious Core doesn't expose:

- **8 DPI stages** instead of 6
- two hidden lighting modes: a **static gradient** and **glitch**
- lighting **speeds 4 and 5** (Glorious Core stops at 3), and **breathing with 1-7 colours**
- a lighting **master switch** that keeps your mode and colours

## Features

| | Library | CLI | GUI |
|---|:---:|:---:|:---:|
| DPI stages (value, on/off, light colour, XY independent) | ✓ | ✓ | ✓ |
| Polling rate, lift-off distance | ✓ | ✓ | ✓ |
| Debounce time | ✓ | ✓ | ✓ |
| Lighting (all 11 modes) | ✓ | mode/colour | ✓ |
| Button mapping (mouse buttons, DPI, keyboard keys) | ✓ | read | ✓ |

Macros and profiles are left out on purpose. The macro format is documented in
[PROTOCOL.md](PROTOCOL.md#macros-command-0x30) in case someone wants it.

## Usage

### GUI

```
cargo run --release -p modeld-gui
```

Edit settings, then click **Apply**. **Reload** discards your edits and reads the mouse again.

### CLI

```
cargo run --release -p modeld-cli -- <command>
```

```
modeld show                     print the settings
modeld dpi 3 1600               stage 3 to 1600 DPI
modeld dpi 3 1600 800           different Y DPI (turns on XY independent)
modeld stage 7 on               enable a hidden 7th stage
modeld poll 1000                polling rate
modeld mode gradient            lighting mode
modeld lights off               lighting off, keeps the mode
modeld debounce 4               debounce time in ms
modeld buttons                  button mapping
modeld help                     everything else, including raw byte access
```

### Library

```rust
use modeld_core::ModelD;

let mouse = ModelD::open()?;
let mut config = mouse.read_config()?;
config.stages[0].x = 800;
config.stages[0].y = 800;
mouse.write_config(&config)?;
```

`Config` keeps the bytes it was read from, so settings it doesn't understand survive a write.

## Layout

```
crates/
  modeld-core/   protocol library (hidapi)
  modeld-cli/    `modeld` command-line tool
  modeld-gui/    egui app
```

## Notes

- **Close Glorious Core** while using this. It doesn't read settings from the mouse, and it
  overwrites everything with its own copy the next time it saves.
- Only tested on Windows 10 with the wired Model D. Other SinoWealth-based Glorious mice (Model
  O, Model D-) probably use a similar layout, but this crate only opens `258A:0033`.
- Building the GUI needs Rust 1.95 or newer (the library and CLI need 1.88).
- Not affiliated with Glorious. You use this at your own risk, though nothing here has been able
  to damage the mouse so far. Writing a saved config back with `modeld restore <file>` undoes
  experiments.

## License

[MIT](LICENSE)

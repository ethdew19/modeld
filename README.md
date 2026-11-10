# modeld

A Rust library, command-line tool and GUI for configuring the **Glorious Model D** (wired, USB
`258A:0033`) without the official software. It talks to the mouse's firmware over HID feature reports (via
[hidapi](https://crates.io/crates/hidapi)), so you don't need a driver or admin rights.

The protocol was reverse engineered with help from Claude and is documented in
[PROTOCOL.md](PROTOCOL.md), and includes some features the official software doesn't expose.

## Features

- DPI stages: value, on/off, light colour, XY independent
- Polling rate and lift-off distance
- Debounce time
- Lighting: all 11 modes with their colours, speed and brightness
- Button mapping: mouse buttons, DPI cycle, keyboard keys

Macros are left out on purpose. The macro format is documented in
[PROTOCOL.md](PROTOCOL.md#macros-command-0x30) in case someone wants it.

## Usage

### GUI

```
cargo run --release -p modeld-gui
```

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

## Notes

- Only tested on Windows 10 with the wired Model D.
- Not affiliated with Glorious. Use at your own risk.

## License

[MIT](LICENSE)

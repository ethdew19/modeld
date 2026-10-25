# Glorious Model D configuration protocol

Reverse engineered from a wired Glorious Model D (USB `258A:0033`, SinoWealth MCU) by diffing
the config before/after changes in Glorious Core, poking single bytes, and a few USB captures
(USBPcap + Wireshark). [OpenRGB's SinowealthController][openrgb] covers the lighting part of
the same protocol.

Everything marked *confirmed* was verified on the mouse. Other values are best guesses.

[openrgb]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/tree/master/Controllers/SinowealthController

## Contents

- [Transport](#transport)
- [Config blob (command `0x11`)](#config-blob-command-0x11)
  - [Header](#header)
  - [Performance](#performance)
  - [DPI stages](#dpi-stages)
  - [Lighting](#lighting)
- [Button table (command `0x12`)](#button-table-command-0x12)
- [Debounce (command `0x1a`)](#debounce-command-0x1a)
- [Macros (command `0x30`)](#macros-command-0x30)
- [Other commands](#other-commands)
- [Unknown bytes](#unknown-bytes)

## Transport

The mouse has two USB interfaces. Interface 0 is the normal mouse. Interface 1 holds a keyboard
collection (for key bindings), consumer control (media keys) and three vendor-defined
collections (usage page `0xFF00`). Windows exposes every collection as its own HID device, and
a feature report only works on the collection that owns it. Asking the wrong one fails with
"Incorrect function".

| Collection | Report | Size | Use |
|---|---|---|---|
| `MI_01&Col05` | 5 | 6 bytes | commands: `05 <cmd> <arg> 00 00 00` |
| `MI_01&Col03` | 4 | 520 bytes | data: `04 <cmd> ...`, read or written whole |
| `MI_01&Col04` | | | unused for configuration |

**Read** = SET_REPORT 5 `05 <cmd> 00 00 00 00` on Col05, then GET_REPORT 4 on Col03.
The mouse returns only as many bytes as the data needs (131 for the config).

**Write** = SET_REPORT 4 on Col03 with the data zero-padded to 520 bytes.

Wait ~50 ms after a write before reading again, or bytes `0x2C..` come back as zeros (found by
OpenRGB). Writing such a read back would wipe the lighting settings.

| Command | Data | Direction |
|---|---|---|
| `0x01` | firmware version string | read via report 5 |
| `0x11` | config blob, 131 bytes | read + write |
| `0x12` | button table, 88 bytes | read + write |
| `0x1a` | debounce time | write only (argument in the command) |
| `0x30` | macro | write only, reading stalls the device |

Glorious Core keeps its own copy of everything and never reads it back from the mouse. When
anything changes it rewrites it all: `05 1a` debounce, `05 11` + read config, `04 30` macros,
`04 12` buttons, `04 11` config. **Profiles exist only in Glorious Core.** Switching profile is
the same full rewrite, with no profile-select command.

## Config blob (command `0x11`)

131 bytes. Colours in the lighting section are stored **R, B, G**, but the DPI light colours
are normal **R, G, B**.

### Header

```
offset  0    1    2    3    4    5    6    7
      ┌────┬────┬────┬────┬────┬────┬────┬────┐
      │ 04 │ 11 │ 00 │ 7b │ 00 │ 00 │ 00 │ 00 │
      └────┴────┴────┴────┴────┴────┴────┴────┘
        │    │    │    │    └───────┬──────┘
        │    │    │    │            └ unknown (offset/address?)
        │    │    │    └ payload length on write: 0x7b = 131 - 8 (00 on read)
        │    │    └ unknown
        │    └ command 0x11
        └ report ID 4
```

### Performance

| Offset | Meaning |
|---|---|
| 8 | unknown, `0x64` |
| 9 | unknown, `0x06` |
| 10 | bit 7: **XY independent DPI**. Low bits: **polling rate**, `01` 125 Hz, `02` 250, `03` 500, `04` 1000 (*confirmed*) |
| 11 | high nibble: **active DPI stage** (1-based), low nibble: **number of enabled stages** (*confirmed*) |
| 12 | **stage disable mask**, bit n-1 set = stage n off (*confirmed*) |
| 13..=28 | DPI values, see below |
| 29..=52 | **DPI light colour** per stage, 8 × `R G B` (*confirmed*) |
| 53..=128 | lighting, see below |
| 129 | **lift-off distance**: `01` 2 mm, `02` 3 mm (*confirmed*) |
| 130 | flags. Bit 1 (`0x02`) = **lighting off**, independent of the mode. The DPI light still works (*confirmed*). Other bits do nothing visible |

### DPI stages

The firmware has **8 stage slots**. Glorious Core only shows 6, but stages 7 and 8 work once
their mask bit is cleared and the count in byte 11 matches (*confirmed*).

A stored value `v` means `(v + 1) × 100` DPI. The sensor's maximum is `0x77` = 12000.

The layout of bytes 13..=28 depends on the XY flag (byte 10, bit 7):

```
XY off:  13 14 15 16 17 18 19 20        one byte per stage, 21..=28 = 00
         s1 s2 s3 s4 s5 s6 s7 s8

XY on:   13 14  15 16  17 18  ...  27 28     X,Y pair per stage
         s1     s2     s3          s8
```

Example, stages 400 / 800 / 1600 / 3200 / 5000 / 10000:

```
XY off:  03 07 0f 1f 31 63 00 00 00 00 ...
XY on:   03 03 07 07 0f 0f 1f 1f 31 31 63 63 00 00 00 00
```

Example of byte 11 and 12: `34 f0` = on stage 3, 4 stages enabled, stages 5-8 disabled.

### Lighting

Byte 53 is the active mode. Every mode keeps its own record, also while inactive, stored in
mode order. Brightness and speed share one byte, `[brightness | speed]`:

- **brightness** (high nibble): 1-4 = 25 / 50 / 75 / 100 %
- **speed** (low nibble): Glorious Core offers 1-3, the firmware goes up to 5. Values above 5
  stop the animation and show a solid colour (*confirmed*)

"unused" means the mode ignores that half. It still holds a value, and you should preserve it.

| Mode | Bytes | Name | Layout |
|---|---|---|---|
| 0 | | off | no record |
| 1 | 54..=55 | glorious (rainbow) | `[unused \| speed]` `[direction: 00 down, 01 up]` |
| 2 | 56..=59 | single colour | `[brightness \| unused]` `R B G` |
| 3 | 60..=82 | breathing | `[brightness \| speed]` `[count 1-7]` 7 × `R B G` |
| 4 | 83 | tail | `[brightness \| speed]` |
| 5 | 84 | seamless breathing | `[unused \| speed]` |
| 6 | 85..=115 | **gradient** (hidden) | `[unused]` 10 × `R B G`, only the first 6 are used |
| 7 | 116..=122 | rave | `[brightness \| speed]` `R B G` `R B G` |
| 8 | 123 | **glitch** (hidden) | `[unused \| speed]` |
| 9 | 124 | wave | `[brightness \| speed]` |
| 10 | 125..=128 | breathing single colour | `[unused \| speed]` `R B G` |

Notes:
- **Breathing** respects the count. Count 2 cycles through only the first two colours, which
  Glorious Core can't do (*confirmed*).
- **Gradient** (mode 6) isn't in Glorious Core. It's a static gradient along the LED strip, made
  from colour stops 1-6. Stops 7-10 have no effect (*confirmed*).
- **Glitch** (mode 8) isn't in Glorious Core either. It's a fast moving pattern with odd
  colours. The name is ours.

## Button table (command `0x12`)

88 bytes: 8-byte header `04 12 00 50 00 00 00 00` (`0x50` = 80 = length of the slots), then
20 slots of 4 bytes. Read and write work like the config (*confirmed*, including writes).

Slots 1-6 are the physical buttons: left, right, middle, rear side, front side, DPI (bottom).
**Glorious Core calls the front side button "button 4"**, which is slot 5.

| Slot bytes | Action |
|---|---|
| `11 bb 00 00` | mouse button, `bb`: `01` left, `02` right, `04` middle, `08` back, `10` forward |
| `21 mm kk 00` | keyboard key, `kk` = USB HID usage code (`04` = A, `05` = B, ...). `mm` is probably modifiers |
| `41 00 00 00` | DPI cycle |
| `70 ii mm rr` | macro, probably number / play mode / repeat count (`70 01 01 01` for the first macro) |
| `50 01 00 00` | unused slot (7-20) |

## Debounce (command `0x1a`)

`05 1a <ms / 2> 00 00 00` on Col05, for 4-16 ms in steps of 2 (Glorious Core's range). Glorious Core sends it twice. It's not stored in the config
blob, and there's no known way to read it back (*confirmed*: 4 ms → `02`, 16 ms → `08`).

Glorious Core never reads it either. In every capture its only read is the config (`05 11` +
GET report 4), so the debounce it displays comes from its own saved state. GET report 5 after
`05 1a` stalls instead of echoing the value.

## Macros (command `0x30`)

Write-only: sending `05 30` to read makes the device stall (it recovers, but don't do it).
Glorious Core writes:

```
04 30 02 00 00 00 00 00      header (byte 2 = 02, meaning unknown)
01 00 06                     01 = ?, 00 06 = number of events?
60 db 02 | 50 4e 04 | d4 75 04 | 50 3e 05 | d3 8a 05 | e0 01 02
```

Every event is 3 bytes:

```
byte 0       byte 1     byte 2
r ttt dddd   dddddddd   code       r = 1 release, 0 press
                                   t = type: 5 key, 6 delay/marker?
                                   d = 12-bit delay in ms
                                   code = HID key code
```

The example is "AB" recorded live (*confirmed* delays):

| Event | Meaning |
|---|---|
| `60 db 02` | 219 ms initial delay |
| `50 4e 04` | A down, 78 ms |
| `d4 75 04` | A up, 1141 ms |
| `50 3e 05` | B down, 62 ms |
| `d3 8a 05` | B up, 906 ms |
| `e0 01 02` | end marker? |

## Other commands

- `05 01` then GET report 5 on Col05: firmware version. Returns `05 01 56 31 30 32` = "V102"
  (*confirmed*).

## Unknown bytes

- Header bytes 2 and 4..=7 of the data reports. The macro write uses byte 2 = `02`, so byte 2
  might be a sub-index.
- Config byte 8 (`0x64`) and byte 9 (`0x06`). Nothing in Glorious Core changes them.
- Config byte 85 (mode 6's first byte) has no visible effect.

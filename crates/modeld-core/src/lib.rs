//! Talk to a Glorious Model D (USB `258A:0033`) without Glorious Core.
//!
//! - [`ModelD`] opens the mouse and moves bytes to and from it.
//! - [`Config`] is the typed 131-byte settings blob (DPI, polling, lighting, ...).
//! - [`Buttons`] is the button mapping table.
//!
//! The byte layouts are documented in `PROTOCOL.md` at the repository root.

mod buttons;
mod color;
mod config;
mod device;
mod error;

pub use buttons::{BUTTON_SLOTS, ButtonAction, Buttons, MouseButton, PHYSICAL_BUTTONS, key_name};
pub use color::Rgb;
pub use config::{
    Breathing, BreathingSingle, CONFIG_LEN, Config, Direction, DpiStage, Glitch, Glorious,
    Gradient, LiftOff, Lighting, LightingMode, MAX_DPI, MAX_SPEED, MIN_DPI, PollingRate, Rave,
    STAGES, Seamless, Single, Tail, Wave,
};
pub use device::ModelD;
pub use error::{Error, Result};

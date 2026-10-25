//! Moving bytes to and from the mouse. See PROTOCOL.md for the channels.

use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::buttons::BUTTONS_LEN;
use crate::{Buttons, CONFIG_LEN, Config, Error, Result};

const VID: u16 = 0x258A;
const PID: u16 = 0x0033;

/// Every report 4 transfer is this long on the wire, whatever it carries.
const DATA_REPORT_LEN: usize = 520;

const CMD_CONFIG: u8 = 0x11;
const CMD_BUTTONS: u8 = 0x12;
const CMD_DEBOUNCE: u8 = 0x1a;

/// Reading right after a write returns zeros from offset 0x2C onward (found by OpenRGB),
/// so give the firmware a moment after every write.
const SETTLE: Duration = Duration::from_millis(50);

/// An open Glorious Model D.
pub struct ModelD {
    /// Col05: report 5, 6-byte commands.
    cmd: HidDevice,
    /// Col03: report 4, 520-byte data.
    data: HidDevice,
    // Kept alive for as long as the devices are open.
    _api: HidApi,
}

impl ModelD {
    /// Finds the mouse and opens its command and data collections.
    pub fn open() -> Result<Self> {
        let api = HidApi::new()?;
        let find = |col: &'static str| {
            api.device_list()
                .find(|d| {
                    d.vendor_id() == VID
                        && d.product_id() == PID
                        && d.path().to_string_lossy().contains(col)
                })
                .ok_or(Error::NotFound(col))
        };
        let cmd = find("Col05")?.open_device(&api)?;
        let data = find("Col03")?.open_device(&api)?;
        Ok(Self {
            cmd,
            data,
            _api: api,
        })
    }

    fn command(&self, cmd: u8, arg: u8) -> Result<()> {
        self.cmd.send_feature_report(&[0x05, cmd, arg, 0, 0, 0])?;
        Ok(())
    }

    fn read_data(&self, cmd: u8) -> Result<Vec<u8>> {
        self.command(cmd, 0)?;
        let mut buf = [0u8; DATA_REPORT_LEN];
        buf[0] = 0x04;
        let n = self.data.get_feature_report(&mut buf)?;
        Ok(buf[..n].to_vec())
    }

    fn write_data(&self, bytes: &[u8]) -> Result<()> {
        let mut out = [0u8; DATA_REPORT_LEN];
        out[..bytes.len()].copy_from_slice(bytes);
        self.data.send_feature_report(&out)?;
        thread::sleep(SETTLE);
        Ok(())
    }

    /// The config exactly as the mouse returns it (131 bytes).
    pub fn read_config_raw(&self) -> Result<Vec<u8>> {
        self.read_data(CMD_CONFIG)
    }

    /// Writes raw config bytes. Byte 3 is set to the payload length (131 - 8 = 0x7b).
    /// No validation: prefer [`ModelD::write_config`].
    pub fn write_config_raw(&self, bytes: &[u8]) -> Result<()> {
        if bytes.len() != CONFIG_LEN || bytes[..2] != [0x04, 0x11] {
            return Err(Error::Invalid(format!(
                "a config is {CONFIG_LEN} bytes starting 04 11"
            )));
        }
        let mut b = bytes.to_vec();
        b[3] = (CONFIG_LEN - 8) as u8;
        self.write_data(&b)
    }

    pub fn read_config(&self) -> Result<Config> {
        Config::from_bytes(&self.read_config_raw()?)
    }

    pub fn write_config(&self, config: &Config) -> Result<()> {
        config.validate()?;
        self.write_config_raw(&config.to_bytes())
    }

    pub fn read_buttons_raw(&self) -> Result<Vec<u8>> {
        self.read_data(CMD_BUTTONS)
    }

    pub fn read_buttons(&self) -> Result<Buttons> {
        Buttons::from_bytes(&self.read_buttons_raw()?)
    }

    pub fn write_buttons(&self, buttons: &Buttons) -> Result<()> {
        let b = buttons.to_bytes();
        debug_assert_eq!(b.len(), BUTTONS_LEN);
        self.write_data(&b)
    }

    /// Debounce is write-only: `05 1a <ms / 2>`. The official software sends it twice.
    pub fn set_debounce(&self, ms: u8) -> Result<()> {
        if !(4..=16).contains(&ms) || !ms.is_multiple_of(2) {
            return Err(Error::Invalid(
                "debounce must be an even number of ms, 4-16".into(),
            ));
        }
        for _ in 0..2 {
            self.command(CMD_DEBOUNCE, ms / 2)?;
        }
        Ok(())
    }
}

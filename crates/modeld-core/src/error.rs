use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A HID collection we need (Col03/Col05) isn't there, so the mouse is probably unplugged.
    NotFound(&'static str),
    /// hidapi failed to open or talk to the mouse.
    Hid(hidapi::HidError),
    /// Bytes from the mouse (or a file) don't look like a valid config / button table.
    Parse(String),
    /// A setting that can't be written, e.g. a DPI out of range.
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotFound(col) => write!(f, "Model D {col} not found, is it plugged in?"),
            Error::Hid(e) => write!(f, "HID error: {e}"),
            Error::Parse(why) => write!(f, "unexpected data: {why}"),
            Error::Invalid(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Error::Hid(e)
    }
}

use interfaces::InterfacesError;
use procfs::ProcError;
use rppal::i2c::Error as I2cError;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::Error as IoError,
    path::PathBuf,
};

/// Errors that occur at runtime.
#[derive(Debug)]
pub enum ErrorKind {
    /// Occurs when piped input has not the right format (`<hum>,<temp>`).
    InvalidInputFormat,
    /// Occurs when unable to access network interfaces.
    InterfaceErr(InterfacesError),
    /// Occurs when given network interface is not found.
    InterfaceNotFound(String),
    /// Occurs when unable to setup I2C bus.
    I2cAccessErr(I2cError),
    /// Occurs when unable to initialize I2C display (SSD1306).
    I2cInitErr,
    /// Occurs when unable to write to I2C display (SSD1306).
    I2cWriteErr,
    /// Occurs when unable to access `/proc` filesystem.
    ProcFsErr(ProcError),
    /// Occurs when unable to open file.
    FileOpenErr(PathBuf, IoError),
    /// Occurs when unable to read from file.
    FileReadErr(PathBuf, IoError),
    /// Occurs when unable to read from standard input.
    StdinErr(IoError),
    /// Occurs when mpsc message passing between threads fails.
    MsgPassingErr,
    /// Any other error.
    Other(String),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInputFormat => {
                write!(f, "invalid input format, use '<hum>,<temp>' instead")
            }
            Self::InterfaceErr(err) => write!(f, "unable to access network interfaces: {}", err),
            Self::InterfaceNotFound(interface) => {
                write!(f, "network interface not found: '{}'", interface)
            }
            Self::I2cAccessErr(err) => write!(f, "unable to access I2C device: {}", err),
            Self::I2cInitErr => write!(f, "unable to initialize I2C display"),
            Self::I2cWriteErr => write!(f, "unable to write to I2C display"),
            Self::ProcFsErr(err) => write!(f, "unable to access '/proc' filesystem: {}", err),
            Self::FileOpenErr(path, err) => {
                write!(f, "unable to open '{}': {}", path.display(), err)
            }
            Self::FileReadErr(path, err) => {
                write!(f, "unable to read from '{}': {}", path.display(), err)
            }
            Self::StdinErr(err) => write!(f, "unable to read from standard input: {}", err),
            Self::MsgPassingErr => write!(f, "unable to send messages between threads"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for ErrorKind {}

impl From<&str> for ErrorKind {
    fn from(msg: &str) -> Self {
        ErrorKind::Other(msg.to_string())
    }
}

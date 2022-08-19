use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

/// Enum representing handled runtime errors.
#[derive(Debug)]
pub enum ErrorKind {
    /// Occurs when unable to parse type.
    ParseErr(String),

    /// Occurs when network interface is not found.
    InterfaceNotFound(String),

    /// Occurs when unable to retrieve network interface info.
    InterfaceInfoErr(String),

    /// Occurs when network interface was not assigned an IPv4.
    IPv4NotFound(String),

    /// Occurs when file could not be read.
    InaccessibleFile(PathBuf),

    /// Occurs when input contains invalid UTF-8.
    InvalidUtf8(Option<PathBuf>),

    /// Occurs when list of system processes could not be retrieved.
    ProcListErr,

    /// Occurs when fed invalid humidity & temperature data.
    InvalidHumTemp,

    /// Occurs when fed invalid input data.
    InvalidInput,

    /// Occurs when unable to register SIGINT event handler.
    SigIntHandlerErr,

    /// Occurs when unable to setup I2C bus.
    I2cSetupErr,

    /// Occurs when unable to write to I2C.
    I2cWriteErr,

    /// Occurs when unable to retrieve KernelStats information.
    KernelStatsErr,

    /// Occurs when unable to join thread.
    ThreadJoinErr(String),
}

/// Implementing Display trait for ErrorKind enum.
impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseErr(value) => write!(f, "unable to parse `{}`", value),
            Self::IPv4NotFound(interface) => {
                write!(f, "no IPv4 found for `{interface}` network interface")
            }
            Self::InterfaceInfoErr(interface) => write!(f, "failed to get `{}` info", interface),
            Self::InterfaceNotFound(interface) => {
                write!(f, "`{interface}` network interface not found")
            }
            Self::InaccessibleFile(filepath) => write!(f, "unable to access {:?}", filepath),
            Self::InvalidUtf8(file) => match file {
                Some(filepath) => write!(f, "{:?} contains invalid UTF-8", filepath),
                None => write!(f, "input contains invalid UTF-8"),
            },
            Self::ProcListErr => write!(f, "unable to retrieve process list"),
            Self::InvalidHumTemp => {
                write!(f, "invalid input format; please use `<hum>,<temp>` instead")
            }
            Self::InvalidInput => write!(
                f,
                "invalid input format; please use `<hum>,<temp>,<csv_status>` instead"
            ),
            Self::SigIntHandlerErr => write!(f, "unable to register SIGINT event handler"),
            Self::I2cSetupErr => write!(f, "unable to setup I2C bus"),
            Self::I2cWriteErr => write!(f, "unable to write to I2C display"),
            Self::KernelStatsErr => write!(
                f,
                "unable to retrieve kernel stat info (unable to access /proc/stat)"
            ),
            Self::ThreadJoinErr(err) => write!(f, "unable to join thread `{}`", err),
        }
    }
}

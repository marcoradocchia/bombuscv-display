pub use clap::Parser;

/// CLI tool to print bombuscv-rs information to I2C display.
#[derive(Parser, Debug)]
#[clap(
    author = "Marco Radocchia <marco.radocchia@outlook.com>",
    version,
    about,
    long_about = None
)]
pub struct Args {
    /// Filesystem path to CPU thermal info.
    #[clap(
        short,
        long,
        value_parser,
        default_value = "/sys/class/thermal/thermal_zone0/temp"
    )]
    pub thermal: String,

    /// Network interface name (IPv4 field).
    #[clap(short, long, value_parser, default_value = "wlan0")]
    pub interface: String,

    /// System readings (CPU, Memory) interval in ms.
    // TODO: needs to be >= 1.
    #[clap(short, long, value_parser, default_value_t = 2)]
    pub interval: u64,

    // TODO: add options to let the user choose I2C pins on RaspberryPi 4 (older RaspberryPis don't
    // support it).
    // /// GPIO pin for SCL (IIC) connection.
    // #[clap(long, value_parser)]
    // pub scl: u8,
    //
    // /// GPIO pin for SDA (IIC) connection.
    // #[clap(long, value_parser)]
    // pub sda: u8,
}

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
    /// CPU `temp` file path.
    #[clap(
        short,
        long,
        value_parser,
        default_value = "/sys/class/thermal/thermal_zone0/temp"
    )]
    pub thermal: String,

    /// Network interface name for local IPv4 stamp.
    #[clap(short, long, value_parser, default_value = "wlan0")]
    pub interface: String,

    // TODO: add options to let the user choose I2C pins on RaspberryPi 4 (older RaspberryPis don't
    // support it).
    //
    // /// GPIO pin for SCL (I2C) connection.
    // #[clap(long, value_parser)]
    // pub scl: u8,
    //
    // /// GPIO pin for SDA (I2C) connection.
    // #[clap(long, value_parser)]
    // pub sda: u8,
}

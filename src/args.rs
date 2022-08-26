pub use clap::{value_parser, Parser};
use ssd1306::prelude::Brightness;

fn parse_brightness(value: &str) -> Result<Brightness, String> {
    Ok(match value {
        "dimmest" => Brightness::DIMMEST,
        "dim" => Brightness::DIM,
        "normal" => Brightness::NORMAL,
        "bright" => Brightness::BRIGHT,
        "brightest" => Brightness::BRIGHTEST,
        _ => return Err(String::from("invalid brightness level")),
    })
}

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
    /// Cpu usage/temperature readings delay in ms (>=100).
    #[clap(
        short,
		long,
		value_parser = value_parser!(u64).range(100..20000),
		default_value_t = 2000
    )]
    pub delay: u64,
    /// Display brightness.
    #[clap(
        short,
		long,
        value_parser = parse_brightness,
		possible_values = ["dimmest", "dim", "normal", "bright", "brightest"],
        default_value = "brightest",
    )]
    pub brightness: Brightness,
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

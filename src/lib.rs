use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use interfaces::{Interface, Kind};
use procfs::{process::all_processes, KernelStats};
use rppal::i2c::{self, I2c};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::Read,
    net::IpAddr,
    ops::Index,
    path::{Path, PathBuf},
};

/// Enum representing handled runtime errors.
#[derive(Debug)]
pub enum ErrorKind<'a> {
    /// Occurs when network interface is not found.
    InterfaceNotFound(&'a str),

    /// Occurs when network interface was not assigned an IPv4.
    IPv4NotFound(&'a str),

    /// Occurs when file could not be read.
    InaccessibleFile(&'a Path),

    /// Occurs when list of system processes could not be retrieved.
    ProcListErr,

    /// Occurs when fed invalid humidity & temperature data.
    InvalidHumTemp,

    /// Occurs when invalid input was piped to the program.
    InvalidInput,

    /// Occurs when unable to register SIGINT event handler.
    SigIntHandlerErr,

    /// Occurs when unable to setup I2C bus.
    I2cSetupErr,

    /// Occurs when unable to write to I2C.
    I2cWriteErr,

    /// Occurs when unable to retrieve KernelStats information.
    KernelStatsErr,
}

/// Implementing Display trait for ErrorKind enum.
impl Display for ErrorKind<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IPv4NotFound(interface) => {
                write!(f, "no IPv4 found for `{interface}` network interface")
            }
            Self::InterfaceNotFound(interface) => {
                write!(f, "`{interface}` network interface not found")
            }
            Self::InaccessibleFile(filename) => write!(f, "impossible to access {:?}", filename),
            Self::ProcListErr => write!(f, "unable to retrieve process list"),
            Self::InvalidHumTemp => {
                write!(f, "invalid input format; please use `<hum>,<temp>` instead")
            }
            Self::InvalidInput => write!(f, "invalid input piped to the program"),
            Self::SigIntHandlerErr => write!(f, "unable to register SIGINT event handler"),
            Self::I2cSetupErr => write!(f, "unable to setup I2C bus"),
            Self::I2cWriteErr => write!(f, "unable to write to I2C display"),
            Self::KernelStatsErr => write!(
                f,
                "unable to retrieve kernel stat info (unable to access /proc/stat)"
            ),
        }
    }
}

/// Humidity and Temperature measure.
#[derive(Debug)]
pub struct Measure {
    humidity: f32,
    temperature: f32,
}

impl<'a> Measure {
    // Construct `Measure`.
    pub fn new(humidity: f32, temperature: f32) -> Self {
        Self {
            humidity,
            temperature,
        }
    }

    /// Construct `Measure` from csv string <hum,temp>.
    pub fn from_csv(data: &str) -> Result<Self, ErrorKind<'a>> {
        let splits: Vec<f32> = data.split(',').map(|val| val.parse().unwrap()).collect();

        if splits.len() == 2 {
            Ok(Measure::new(*splits.index(0), *splits.index(1)))
        } else {
            Err(ErrorKind::InvalidHumTemp)
        }
    }
}

impl Display for Measure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "H: {}% T: {}C", self.humidity, self.temperature)
    }
}

/// Retrieve local IPv4 of network interface.
pub fn local_ipv4(interface: &str) -> Result<IpAddr, ErrorKind> {
    if let Some(interface) =
        Interface::get_by_name(interface).expect("failed to get {interface} info")
    {
        for addr in &interface.addresses {
            if addr.kind == Kind::Ipv4 {
                return Ok(addr.addr.unwrap().ip());
            }
        }
    } else {
        return Err(ErrorKind::InterfaceNotFound(interface));
    }

    Err(ErrorKind::IPv4NotFound(interface))
}

/// CPU info.
///
/// # Fields
///
/// - thermal_zone: filesystem path to CPU thermal info
/// - idle_time: idle time from /proc/stat
/// - total_time: total time from /proc/stat
#[derive(Debug, Clone)]
pub struct Cpu {
    thermal_zone: PathBuf,
    idle_time: u64,
    total_time: u64,
    temp: f32,
    usage: f64,
}

impl Cpu {
    // Construct `Cpu` given thermal_zone path.
    pub fn new(thermal_zone: &str) -> Result<Self, ErrorKind> {
        // Retrieve current idle and total times.
        let (idle_time, total_time) = Cpu::get_times()?;

        Ok(Self {
            thermal_zone: PathBuf::from(thermal_zone),
            idle_time,
            total_time,
            temp: 0.0,
            usage: 0.0,
        })
    }

    /// Get time information from /proc/stat on Linux filesystem.
    fn get_times<'a>() -> Result<(u64, u64), ErrorKind<'a>> {
        // Read /proc/stat information and retrieve `cpu` row.
        let cpu = if let Ok(stat) = KernelStats::new() {
            stat.total
        } else {
            return Err(ErrorKind::KernelStatsErr);
        };

        // Calculate the total time.
        Ok((
            cpu.idle,
            cpu.user
                + cpu.nice
                + cpu.system
                + cpu.idle
                + cpu.iowait.unwrap_or(0)
                + cpu.irq.unwrap_or(0)
                + cpu.softirq.unwrap_or(0),
        ))
    }

    /// Retrieve CPU package temperature in Celsius degrees.
    fn temp(&mut self) -> Result<(), ErrorKind<'_>> {
        let mut temp = String::new();

        if let Ok(mut file) = File::open(&self.thermal_zone) {
            file.read_to_string(&mut temp)
                .expect("unable to read `{thermal_zone}` file");

            self.temp = temp
                .trim()
                .parse::<f32>()
                .expect("unable to parse `{thermal_zone}` content to f32")
                / 1000.0;

            return Ok(());
        }

        return Err(ErrorKind::InaccessibleFile(&self.thermal_zone));
    }

    /// Retrieves CPU overall percentage usage.
    fn usage(&mut self) -> Result<(), ErrorKind<'_>> {
        let (idle_time, total_time) = Cpu::get_times()?;

        // Total CPU usage ([0-100]%).
        let usage = (1.0
            - (idle_time - self.idle_time) as f64 / (total_time - self.total_time) as f64)
            * 100.0;

        // Update values.
        self.total_time = total_time;
        self.idle_time = idle_time;

        // Calculate percentage subtracting idling time fraction from total time.
        self.usage = usage;

        Ok(())
    }

    /// Retrieve CPU information.
    pub fn read_info(&mut self) -> Result<String, ErrorKind<'_>> {
        self.usage().unwrap(); // NOTE: This error should be handled.
        self.temp().unwrap(); // NOTE: This error should be handled.

        Ok(self.to_string())
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}% {:.1}C", self.usage, self.temp)
    }
}

/// Retrieves disk free space.
pub fn disk_free<'a>() -> Result<(), ErrorKind<'a>> {
    unimplemented!();
}

/// Check for running process returnin bool whether the process is running or not.
pub fn pgrep(name: &str) -> Result<bool, ErrorKind> {
    if let Ok(proc_list) = all_processes() {
        for proc in proc_list {
            if let Ok(exe_path) = proc.unwrap().exe() {
                if exe_path.file_stem().unwrap() == name {
                    return Ok(true);
                }
            }
        }
    } else {
        return Err(ErrorKind::ProcListErr);
    }

    Ok(false)
}

pub struct I2cDisplay {
    disp: Ssd1306<I2CInterface<I2c>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
}

impl I2cDisplay {
    /// Initialize & setup SH1106 I2C display.
    pub fn new<'a>() -> Result<Self, ErrorKind<'a>> {
        // TODO: change here to let the user specify custom pins
        if let Ok(i2c) = i2c::I2c::new() {
            let mut disp = Ssd1306::new(
                I2CDisplayInterface::new(i2c),
                DisplaySize128x64,
                DisplayRotation::Rotate0,
            )
            .into_buffered_graphics_mode();

            // Init & flush display.
            if disp.init().is_err() || disp.flush().is_err() {
                return Err(ErrorKind::I2cWriteErr);
            }

            Ok(Self { disp })
        } else {
            Err(ErrorKind::I2cSetupErr)
        }
    }

    /// Refresh display screen.
    pub fn refresh_display<'a>(&mut self, lines: &str) -> Result<(), ErrorKind<'a>> {
        // Clear the display buffer.
        self.disp.clear();

        // Draw text to display and flush.
        if Text::with_baseline(
            lines,
            Point::zero(),
            MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
            embedded_graphics::text::Baseline::Top,
        )
        .draw(&mut self.disp)
        .is_err()
            || self.disp.flush().is_err()
        {
            return Err(ErrorKind::I2cWriteErr);
        }

        Ok(())
    }
}

/// Test module.
#[cfg(test)]
mod tests {
    use super::Cpu;
    use std::{thread, time::Duration};

    /// Test cpu.usage().
    #[test]
    fn cpu_usage() {
        let mut cpu = Cpu::new("/sys/class/thermal/thermal_zone0").unwrap();

        for _ in 0..10 {
            thread::sleep(Duration::from_secs(1));
            println!("CPU: {:#?}", cpu.read_info().unwrap());
        }
    }
}

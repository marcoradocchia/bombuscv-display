mod error;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
pub use error::ErrorKind;
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
    path::PathBuf,
    process::Command,
};

/// Pipeline standard input.
#[derive(Debug)]
pub struct Input {
    /// Humidity and temperature measure.
    pub measure: Measure,
    /// CSV logging status:
    /// true -> logging to file
    /// false -> not logging to file
    pub csv: bool,
}

impl Input {
    /// Construct `Input`.
    pub fn new(measure: Measure, csv: bool) -> Self {
        Self { measure, csv }
    }

    /// Construct `Input` from CSV string <hum,temp,csv>.
    pub fn from_csv(data: &str) -> Result<Self, ErrorKind> {
        // IMPORTANT TODO: maybe a deserializer would improve the code.
        let splits: Vec<&str> = data.split(',').collect();
        if !splits.len() == 3 {
            return Err(ErrorKind::InvalidInput);
        }

        // Vector lenght checked, so indexing won't couse panics.
        let csv = splits
            .index(2)
            .parse()
            .map_err(|_| ErrorKind::ParseErr(splits.index(2).to_string()))?;

        let parse_f32 = |value: &str| -> Result<f32, ErrorKind> {
            value
                .parse::<f32>()
                .map_err(|_| ErrorKind::ParseErr(value.to_string()))
        };

        Ok(Self {
            measure: Measure::new(parse_f32(*splits.index(0))?, parse_f32(*splits.index(1))?),
            csv,
        })
    }
}

/// Humidity and Temperature measure.
#[derive(Debug)]
pub struct Measure {
    /// Humidity value.
    humidity: f32,
    /// Temperature value.
    temperature: f32,
}

impl Measure {
    // Construct `Measure`.
    pub fn new(humidity: f32, temperature: f32) -> Self {
        Self {
            humidity,
            temperature,
        }
    }

    /// Construct `Measure` from array of f32 values [<hum>, <temp>].
    pub fn from_array(data: [f32; 2]) -> Self {
        Self {
            humidity: data[0],
            temperature: data[1],
        }
    }

    /// Construct `Measure` from CSV string <hum,temp>.
    pub fn from_csv(data: &str) -> Result<Self, ErrorKind> {
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
    if let Some(interface) = Interface::get_by_name(interface)
        .map_err(|_| ErrorKind::InterfaceInfoErr(interface.to_string()))?
    {
        for addr in &interface.addresses {
            if addr.kind == Kind::Ipv4 {
                match addr.addr {
                    Some(socket_addr) => return Ok(socket_addr.ip()),
                    None => continue,
                }
            }
        }
    } else {
        return Err(ErrorKind::InterfaceNotFound(interface.to_string()));
    }

    Err(ErrorKind::IPv4NotFound(interface.to_string()))
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
    fn get_times() -> Result<(u64, u64), ErrorKind> {
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
    fn temp(&mut self) -> Result<(), ErrorKind> {
        let mut temp = String::new();

        if let Ok(mut file) = File::open(&self.thermal_zone) {
            file.read_to_string(&mut temp)
                .map_err(|_| ErrorKind::InvalidUtf8(Some(self.thermal_zone.clone())))?;

            self.temp = temp
                .trim()
                .parse::<f32>()
                .map_err(|_| ErrorKind::ParseErr(temp))?
                / 1000.0;

            return Ok(());
        }

        Err(ErrorKind::InaccessibleFile(self.thermal_zone.clone()))
    }

    /// Retrieves CPU overall percentage usage.
    fn usage(&mut self) -> Result<(), ErrorKind> {
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
    pub fn read_info(&mut self) -> Result<String, ErrorKind> {
        self.usage()?;
        self.temp()?;

        Ok(self.to_string())
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}% {:.1}C", self.usage, self.temp)
    }
}

/// Retrieves disk free space.
pub fn disk_free() -> Result<String, ErrorKind> {
    // Spawn command and collect output.
    let output = Command::new("df")
        .args(["-h", "--output=avail", "/"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).map_err(|_| ErrorKind::InvalidUtf8(None))?;
    let stdout: Vec<&str> = stdout.split('\n').collect();

    // Trim leading white spaces.
    Ok(stdout[1].trim_start().to_string())
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
    pub fn new() -> Result<Self, ErrorKind> {
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
    pub fn refresh_display(&mut self, lines: &str) -> Result<(), ErrorKind> {
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
    use std::{process::Command, thread, time::Duration};

    /// Test cpu.usage().
    #[test]
    fn cpu_usage() {
        let mut cpu = Cpu::new("/sys/class/thermal/thermal_zone0").unwrap();

        for _ in 0..10 {
            thread::sleep(Duration::from_secs(1));
            println!("CPU: {:#?}", cpu.read_info().unwrap());
        }
    }

    /// Root partition ("/") free space.
    #[test]
    fn disk_free() {
        // Spawn command and collect output.
        let output = Command::new("df")
            .args(["-h", "--output=avail", "/"])
            .output()
            .unwrap();
        // Convert Vec<u8> output to String.
        let stdout = String::from_utf8(output.stdout).unwrap();
        // Split stdout lines and collect into Vec<&str>.
        let stdout: Vec<&str> = stdout.split("\n").collect();

        // Trim leading white spaces.
        println!("{}", stdout[1].trim_start());
    }
}

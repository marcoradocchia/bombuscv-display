use clap::Parser;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use interfaces::{Interface, Kind};
use procfs::process::all_processes;
use rppal::i2c::{self, I2c};
use signal_hook::{consts::SIGINT, flag::register};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, BufRead, Read},
    net::{IpAddr, Ipv4Addr},
    ops::Index,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

/// CLI tool to print bombuscv-rs information to I2C display.
#[derive(Parser, Debug)]
#[clap(
    author = "Marco Radocchia <marco.radocchia@outlook.com>",
    version,
    about,
    long_about = None
)]
struct Args {
    /// CPU `temp` file path.
    #[clap(
        short,
        long,
        value_parser,
        default_value = "/sys/class/thermal/thermal_zone0/temp"
    )]
    thermal: String,

    /// Network interface name for local IPv4 stamp.
    #[clap(short, long, value_parser, default_value = "wlan0")]
    interface: String,

    /// GPIO pin for SCL (I2C) connection.
    #[clap(long, value_parser)]
    scl: u8,

    /// GPIO pin for SDA (I2C) connection.
    #[clap(long, value_parser)]
    sda: u8,
}

/// Enum representing handled runtime errors.
enum ErrorKind<'a> {
    /// Occurs when network interface is not found.
    InterfaceNotFound(&'a str),

    /// Occurs when network interface was not assigned an IPv4.
    IPv4NotFound(&'a str),

    /// Occurs when file could not be read.
    InaccessibleFile(&'a str),

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
            Self::InaccessibleFile(filename) => write!(f, "impossible to access `{filename}` file"),
            Self::ProcListErr => write!(f, "unable to retrieve process list"),
            Self::InvalidHumTemp => {
                write!(f, "invalid input format; please use `<hum>,<temp>` instead")
            }
            Self::InvalidInput => write!(f, "invalid input piped to the program"),
            Self::SigIntHandlerErr => write!(f, "unable to register SIGINT event handler"),
            Self::I2cSetupErr => write!(f, "unable to setup I2C bus"),
            Self::I2cWriteErr => write!(f, "unable to write to I2C display"),
        }
    }
}

/// Humidity and Temperature measure.
#[derive(Debug)]
struct Measure {
    humidity: f32,
    temperature: f32,
}

impl<'a> Measure {
    // Construct `Measure`.
    fn new(humidity: f32, temperature: f32) -> Self {
        Self {
            humidity,
            temperature,
        }
    }

    /// Construct `Measure` from string <hum,temp>.
    fn from_str(data: &str) -> Result<Self, ErrorKind<'a>> {
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
        write!(f, "H: {}%, T: {}°C", self.humidity, self.temperature)
    }
}

/// Retrieve local IPv4 of network interface.
fn local_ipv4(interface: &str) -> Result<IpAddr, ErrorKind> {
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

/// Retrieve CPU package temperature.
fn cpu_temp(thermal_zone: &str) -> Result<f32, ErrorKind> {
    let mut temp = String::new();

    if let Ok(mut file) = File::open(thermal_zone) {
        file.read_to_string(&mut temp)
            .expect("unable to read `{thermal_zone}` file");
        return Ok(temp
            .trim()
            .parse::<f32>()
            .expect("unable to parse `{thermal_zone}` content to f32")
            / 1000.0);
    }

    return Err(ErrorKind::InaccessibleFile(thermal_zone));
}

/// Check for running process returnin bool whether the process is running or not.
fn pgrep(name: &str) -> Result<bool, ErrorKind> {
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

struct I2cDisplay {
    disp: Ssd1306<I2CInterface<I2c>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
}

impl I2cDisplay {
    /// Initialize & setup SH1106 I2C display.
    fn new<'a>() -> Result<Self, ErrorKind<'a>> {
        // TODO: change here to let the user specify custom pins
        if let Ok(i2c) = i2c::I2c::new() {
            let interface = I2CDisplayInterface::new(i2c);
            Ok(Self {
                disp: Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                    .into_buffered_graphics_mode(),
            })
        } else {
            Err(ErrorKind::I2cSetupErr)
        }
    }

    /// Refresh display screen.
    fn refresh_display<'a>(&mut self, lines: &str) -> Result<(), ErrorKind<'a>> {
        // WARNING: it would be a better practice to implement From trait for ErrorKind in order to
        // convert DI::Error to ErrorKind variant and use ? operator to return error.
        if self.disp.init().is_err() {
            return Err(ErrorKind::I2cWriteErr);
        }
        if self.disp.flush().is_err() {
            return Err(ErrorKind::I2cWriteErr);
        }

        // Draw text to display.
        if Text::with_baseline(
            lines,
            Point::zero(),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            embedded_graphics::text::Baseline::Top
        )
        .draw(&mut self.disp)
        .is_err()
        {
            return Err(ErrorKind::I2cWriteErr);
        }

        if self.disp.flush().is_err() {
            return Err(ErrorKind::I2cWriteErr);
        }
        Ok(())
    }
}

/// Run application and catch errors.
fn run(args: &Args) -> Result<(), ErrorKind> {
    // Register signal-hook for SIGINT (Ctrl-C) events: in this case error is unrecoverable.
    let term = Arc::new(AtomicBool::new(false));
    if register(SIGINT, Arc::clone(&term)).is_err() {
        return Err(ErrorKind::SigIntHandlerErr);
    };

    // Sender/Receiver for measure values.
    let (tx_measure, rx_measure) = mpsc::channel();

    let measure_handle = thread::spawn(move || -> Result<(), ErrorKind> {
        // Read data from stdin (used in this case to pipe from datalogger, program).
        // https://github.com/marcoradocchia/datalogger
        for line in io::stdin().lock().lines() {
            if let Ok(line) = line {
                tx_measure
                    .send(Measure::from_str(&line))
                    .expect("unable to send hum_temp data between threads");
            } else {
                return Err(ErrorKind::InvalidInput);
            }
        }

        Ok(())
    });

    // Initialize I2C  display.
    let mut i2c_display = I2cDisplay::new()?;

    // Grab the first measure.
    let mut measure: Measure = rx_measure
        .recv()
        .expect("unable to receive measure from measure thread")?;

    // Start grabber loop: loop guard is `received SIGINT`.
    while !term.load(Ordering::Relaxed) {
        // This sets approx display refresh rate.
        if let Ok(new_measure) = rx_measure.recv_timeout(Duration::from_millis(2000)) {
            measure = new_measure?
        }

        // Refresh I2C display.
        i2c_display.refresh_display(&format!(
            "{}\nIP: {}\nCPU: {}°C\nBOMBUSCV: {}",
            measure,
            local_ipv4(&args.interface).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            cpu_temp(&args.thermal)?,
            if pgrep("bombuscv")? { "running" } else { "--" }
        ))?;
    }

    measure_handle
        .join()
        .expect("unable to join measure_handle thread")?;
    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

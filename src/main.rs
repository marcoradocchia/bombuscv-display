mod args;
mod display;
mod error;
mod measure;
mod sys_info;

use args::{Args, Parser};
use chrono::Local;
use display::I2cDisplay;
use error::ErrorKind;
use interfaces::Interface;
use measure::Measure;
use signal_hook::{consts::SIGUSR1, flag::register};
use std::{
    fmt::{self, Display, Formatter},
    io::{self, BufRead},
    net::{IpAddr, Ipv4Addr},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
use sys_info::{disk_free, pgrep, Cpu, CpuInfo, InterfaceIPv4, Meminfo, MeminfoPerc};

type Result<T> = std::result::Result<T, ErrorKind>;

/// Display screen data.
struct Screen {
    /// `datalogger` humidity & temperature measure values.
    measure: Measure,
    /// Network interface.
    interface: Interface,
    /// Cpu usage and temperature values.
    cpu: CpuInfo,
    /// Memory (RAM) information from `/proc/meminfo`.
    mem: Meminfo,
    /// Free disk space.
    disk: String,
    /// `datalogger` CSV file printing.
    ///
    /// # Note
    ///
    /// This program assumes to be run in a pipeline with `datalogger` with starting CSV behaviour
    /// turned off, so this field needs to be initialized as `false`.
    logging: bool,
    /// `datalogger` process running.
    datalogger: bool,
    /// `bombuscv` process running.
    bombuscv: bool,
}

impl Screen {
    fn new(interface: &str) -> Result<Self> {
        Ok(Self {
            measure: Measure::default(),
            interface: Interface::get_by_name(interface)
                .map_err(ErrorKind::InterfaceErr)?
                .ok_or_else(|| ErrorKind::InterfaceNotFound(interface.to_string()))?,
            cpu: CpuInfo::default(),
            mem: Meminfo::new().map_err(ErrorKind::ProcFsErr)?,
            disk: String::from("--"),
            logging: false,
            datalogger: pgrep("datalogger")?,
            bombuscv: pgrep("bombuscv")?,
        })
    }

    fn update(
        &mut self,
        measure: Option<Measure>,
        cpu: Option<CpuInfo>,
        sigusr1_received: bool,
    ) -> Result<()> {
        if let Some(measure) = measure {
            self.measure = measure;
        };
        if let Some(cpu) = cpu {
            self.cpu = cpu;
        };
        self.mem = Meminfo::new().map_err(ErrorKind::ProcFsErr)?;
        self.disk = disk_free()?;
        // If SIGUSR1 is received swap logging status.
        if sigusr1_received {
            self.logging = !self.logging
        }
        self.datalogger = pgrep("datalogger")?; // TODO: Here check logging status
        self.bombuscv = pgrep("bombuscv")?;

        Ok(())
    }
}

impl Display for Screen {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{}\nIP: {}\nCPU: {}\nMEM: {:.1}% DISK: {}\nDATALOGGER: {}\nBOMBUSCV: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            self.measure,
            self.interface
                .local_ipv4()
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            self.cpu,
            self.mem.free_percent(),
            self.disk,
            if self.datalogger && self.logging {
                "logging"
            } else {
                "--"
            },
            if self.bombuscv { "running" } else { "--" },
        )
    }
}

/// Run application and catch errors
fn run(args: Args) -> Result<()> {
    let mut i2c_display = I2cDisplay::new(args.brightness)?;
    let mut screen: Screen = Screen::new(&args.interface)?;

    // Multithreading channels.
    let (tx_measure, rx_measure) = mpsc::channel();
    let (tx_cpu, rx_cpu) = mpsc::channel();

    // Thread handling humidity and temperature data piped to the program.
    let measure_thread = thread::spawn(move || -> Result<()> {
        loop {
            // Read data from stdin (used in this case to pipe from datalogger, program).
            // https://github.com/marcoradocchia/datalogger
            for line in io::stdin().lock().lines() {
                tx_measure
                    .send(Measure::from_csv(&line.map_err(ErrorKind::StdinErr)?)?)
                    .map_err(|_| ErrorKind::MsgPassingErr)?;
            }
        }
    });

    // Thread handling system information.
    let cpu_thread = thread::spawn(move || -> Result<()> {
        let mut cpu = Cpu::new(&args.thermal)?;

        loop {
            tx_cpu
                .send(cpu.info()?)
                .map_err(|_| ErrorKind::MsgPassingErr)?;

            thread::sleep(Duration::from_millis(args.delay));
        }
    });

    // Register signal hook for SIGUSR1 events.
    // Signal received means `datalogger` changed CSV printing behaviour:
    // +-----------+-----------+
    // |BEFORE SIG |AFTER SIG  |
    // +-----------+-----------+
    // |logging    |not logging|
    // +-----------+-----------+
    // |not logging|logging    |
    // +-----------+-----------+
    let sigusr1 = Arc::new(AtomicBool::new(false));
    // Set `sig` to true when the program receives a SIGUSR1 signal.
    register(SIGUSR1, Arc::clone(&sigusr1))
        .map_err(|_| "unable to register SIGUSR1 event handler")?;

    // Refresh display at 1Hz.
    loop {
        let instant = Instant::now();

        if measure_thread.is_finished() {
            return measure_thread
                .join()
                .map_err(|_| "unable to join humidity/temperature thread")?;
        }
        if cpu_thread.is_finished() {
            return cpu_thread
                .join()
                .map_err(|_| "unable to join cpu usage/temperature thread")?;
        }

        // Update screen & refresh I2C display.
        screen.update(
            rx_measure.recv_timeout(Duration::ZERO).ok(),
            rx_cpu.recv_timeout(Duration::ZERO).ok(),
            // Restore original `sigusr1` before it was changed by signal received, in
            // order to be able to receive other signals.
            sigusr1
                .load(Ordering::Relaxed)
                .then(|| sigusr1.store(false, Ordering::Relaxed))
                .is_some(),
        )?;
        // dbg!(screen.datalogger, screen.logging);
        i2c_display.refresh_display(&screen.to_string())?;

        // Sleep for 1 second (1Hz refresh rate) corrected by the time spent measuring: if elapsed
        // time is grates than the specified interval, this means the measuring process took
        // longer than expected, so don't wait at all since we're already late.
        if let Some(delay) = Duration::from_secs(1).checked_sub(instant.elapsed()) {
            thread::sleep(delay);
        }
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("error: {e}.");
        process::exit(1);
    }
}

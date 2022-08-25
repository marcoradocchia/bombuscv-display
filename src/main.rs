mod args;
mod display;
mod measure;
mod sys_info;

use anyhow::{anyhow, Result};
use args::{Args, Parser};
use chrono::Local;
use display::I2cDisplay;
use interfaces::Interface;
use measure::Measure;
use std::{
    fmt::{self, Display, Formatter},
    io::{self, BufRead},
    net::{IpAddr, Ipv4Addr},
    process,
    sync::mpsc,
    thread,
    time::Duration,
};
use sys_info::{Cpu, disk_free, pgrep, InterfaceIPv4, Meminfo, MeminfoPerc, CpuInfo};

/// Display screen data.
struct Screen {
    measure: Measure,
    interface: Interface,
    cpu: CpuInfo,
    mem: Meminfo,
    disk: String,
    datalogger: bool,
    bombuscv: bool,
}

impl Screen {
    fn new(interface: &str) -> Result<Self> {
        Ok(Self {
            measure: Measure::default(),
            interface: Interface::get_by_name(interface)?
                .ok_or_else(|| anyhow!("'{}' network interface not found", interface))?,
            cpu: CpuInfo::default(),
            mem: Meminfo::new()?,
            disk: String::from("--"),
            datalogger: pgrep("datalogger")?,
            bombuscv: pgrep("bombuscv")?,
        })
    }

    fn update(
        &mut self,
        measure: Option<Measure>,
        cpu: Option<CpuInfo>,
    ) -> Result<()> {
        if let Some(measure) = measure {
            self.measure = measure;
        };
        if let Some(cpu) = cpu {
            self.cpu = cpu;
        };
        self.mem = Meminfo::new()?;
        self.disk = disk_free()?;
        self.datalogger = pgrep("datalogger")?; // Here check logging status
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
            self.interface.local_ipv4().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            self.cpu,
            self.mem.free_percent(),
            self.disk,
            if self.datalogger { "logging" } else { "--" },
            if self.bombuscv { "running" } else { "--" },
        )
    }
}

/// Run application and catch errors
fn run(args: Args) -> Result<()> {
    let mut i2c_display = I2cDisplay::new()?;
    let mut screen: Screen = Screen::new(&args.interface)?;

    // Multithreading channels.
    let (tx_measure, rx_measure) = mpsc::channel();
    let (tx_cpu, rx_cpu) = mpsc::channel();

    // Thread handling humidity and temperature data piped to the program.
    thread::spawn(move || -> Result<()> {
        loop {
            // Read data from stdin (used in this case to pipe from datalogger, program).
            // https://github.com/marcoradocchia/datalogger
            for line in io::stdin().lock().lines() {
                tx_measure.send(Measure::from_csv(&line?)?)?;
            }
        }
    });

    // Thread handling system information.
    thread::spawn(move || -> Result<()> {
        let mut cpu = Cpu::new(&args.thermal)?;
        loop {
            tx_cpu.send(cpu.info()?)?;
            thread::sleep(Duration::from_millis(args.delay));
        }
    });

    loop {
        screen.update(
            rx_measure.recv_timeout(Duration::ZERO).ok(),
            rx_cpu.recv_timeout(Duration::ZERO).ok(),
        )?;
        // Refresh I2C display.
        i2c_display.refresh_display(&screen.to_string())?;
        thread::sleep(Duration::from_secs(1));
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

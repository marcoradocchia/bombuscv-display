pub use procfs::Meminfo;

use crate::{ErrorKind, Result};
use interfaces::{Interface, Kind};
use procfs::{process::all_processes, KernelStats};
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::Read,
    net::IpAddr,
    path::PathBuf,
    process::Command,
};

/// Cpu usage and temperature information.
pub struct CpuInfo {
    usage: f32,
    temp: f32,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            usage: 0.0,
            temp: 0.0,
        }
    }
}

impl Display for CpuInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}% {:.1}C", self.usage, self.temp)
    }
}

/// Cpu.
///
/// # Fields
///
/// - thermal_zone: filesystem path to CPU thermal info
/// - idle_time: idle time from /proc/stat
/// - total_time: total time from /proc/stat
/// - usage: `Cpu` usage since last update
/// - temp: `Cpu` temperature since last update
#[derive(Debug, Clone)]
pub struct Cpu {
    thermal_zone: PathBuf,
    idle_time: u64,
    total_time: u64,
}

impl Cpu {
    /// Construct `Cpu` with the given `thermal_zone` path.
    pub fn new(thermal_zone: &str) -> Result<Self> {
        // Retrieve current idle and total times.
        let (idle_time, total_time) = Cpu::get_times()?;

        Ok(Self {
            thermal_zone: PathBuf::from(thermal_zone),
            idle_time,
            total_time,
        })
    }

    /// Return time information from `/proc/stat` on Linux filesystem as `(<cpu_idle>, <cpu_total>)`.
    fn get_times() -> Result<(u64, u64)> {
        // Read /proc/stat information and retrieve `cpu` row.
        let cpu = KernelStats::new().map_err(ErrorKind::ProcFsErr)?.total;

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

    /// Return `Cpu` package temperature in *Celsius degrees*.
    pub fn temp(&mut self) -> Result<f32> {
        let mut temp = String::new();

        let mut file = File::open(&self.thermal_zone)
            .map_err(|err| ErrorKind::FileOpenErr(self.thermal_zone.to_owned(), err))?;
        file.read_to_string(&mut temp)
            .map_err(|err| ErrorKind::FileReadErr(self.thermal_zone.to_owned(), err))?;

        // Safe to unwrap here, guaranteed to have correct format.
        Ok(temp.trim().parse::<f32>().unwrap() / 1000.0)
    }

    /// Return `Cpu` overall percentage usage.
    pub fn usage(&mut self) -> Result<f32> {
        let (idle_time, total_time) = Cpu::get_times()?;

        // Total CPU usage ([0-100]%).
        let usage = (1.0
            - (idle_time - self.idle_time) as f32 / (total_time - self.total_time) as f32)
            * 100.0;

        // Update values.
        self.total_time = total_time;
        self.idle_time = idle_time;

        Ok(usage)
    }

    /// Update `usage` and `temperature` fields, as well as `Cpu` times.
    pub fn info(&mut self) -> Result<CpuInfo> {
        Ok(CpuInfo {
            usage: self.usage()?,
            temp: self.temp()?,
        })
    }
}

/// Memory info values expressed as percentage.
pub trait MeminfoPerc {
    /// Convert absolute *kB* value (as found in `/proc/meminfo`) to percentage with respect to
    /// total memory.
    fn percentage(&self, value: u64) -> f32;

    /// Return used memory percentage.
    fn used_percent(&self) -> f32;

    /// Return free memory percentage.
    fn free_percent(&self) -> f32;
}

impl MeminfoPerc for Meminfo {
    /// Convert absolute kB value to percentage with respect to total memory.
    fn percentage(&self, value: u64) -> f32 {
        ((value as f64 / self.mem_total as f64) * 100.0) as f32
    }

    /// Return free memory percentage.
    fn free_percent(&self) -> f32 {
        self.percentage(self.mem_free)
    }

    /// Return used memory percentage.
    fn used_percent(&self) -> f32 {
        100.0 - self.free_percent()
    }
}

pub trait InterfaceIPv4 {
    /// Return **IPv4** Address of given interface if present, None otherwhise.
    fn local_ipv4(&self) -> Option<IpAddr>;
}

impl InterfaceIPv4 for Interface {
    /// Return **IPv4** Address of given interface if present, None otherwhise.
    fn local_ipv4(&self) -> Option<IpAddr> {
        for addr in &self.addresses {
            if addr.kind == Kind::Ipv4 {
                if let Some(socket_addr) = addr.addr {
                    return Some(socket_addr.ip());
                }
            }
        }

        None
    }
}

// TODO: remove dependency to other shell commands.
/// Return free disk space in *human readable format*.
pub fn disk_free() -> Result<String> {
    // Spawn `df` command with human readable parameter `-h` on `/` and collect output.
    let output = Command::new("df")
        .args(["-h", "--output=avail", "/"])
        .output()
        .map_err(|_| "unable to execute `df` command")?;

    Ok(String::from_utf8(output.stdout)
        .unwrap() // Safe to unwrap, command output is guaranteed to be UTF-8.
        .split('\n')
        .collect::<Vec<&str>>()[1]
        .trim_start()
        .to_string())
}

/// Check for running process returning true if the process is running, false if not.
pub fn pgrep(name: &str) -> Result<bool> {
    for proc in all_processes().map_err(ErrorKind::ProcFsErr)? {
        if let Ok(exe_path) = proc.map_err(ErrorKind::ProcFsErr)?.exe() {
            // Filename guaranteed, safe to unwrap.
            if exe_path.file_stem().unwrap() == name {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

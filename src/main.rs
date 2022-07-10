use interfaces::{Interface, Kind};
use procfs::process::all_processes;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, BufRead, Read},
    net::IpAddr,
    process,
    sync::mpsc,
    thread,
};

const INTERFACE_NAME: &str = "wlp1s0";
// const THERMAL_ZONE: &str = "/sys/class/thermal/thermal_zone0/temp";
const THERMAL_ZONE: &str = "/sys/class/hwmon/hwmon5/temp1_input";

enum ErrorKind {
    /// Occurs when network interface is not found.
    InterfaceNotFound,
    /// Occurs when network interface was not assigned an IPv4.
    IPv4NotFound,
    /// Occurs when file could not be read.
    InaccessibleFile(&'static str),
    /// Occurs when list of system processes could not be retrieved.
    ProcListErr,
    /// Occurs when fed invalid humidity & temperature data.
    InvalidHumTemp(&'static str)
}

/// Implementing Display trait for ErrorKind enum.
impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IPv4NotFound => format!("no IPv4 found for `{INTERFACE_NAME}` network interface"),
            Self::InterfaceNotFound => format!("`{INTERFACE_NAME}` network interface not found"),
            Self::InaccessibleFile(filename) => format!("impossible to access `{filename}` file"),
            Self::ProcListErr => "unable to retrieve process list".to_string(),
            Self::InvalidHumTemp(data) => format!("`{data}` is not a valid data format; please use `<hum>,<temp>` instead")
        }
        .fmt(f)
    }
}

/// Retrieve local IPv4 of network interface.
fn local_ipv4() -> Result<IpAddr, ErrorKind> {
    let interface = match Interface::get_by_name(INTERFACE_NAME)
        .expect("failed to get {INTERFACE_NAME} info")
    {
        Some(interface) => interface,
        None => return Err(ErrorKind::InterfaceNotFound),
    };

    for addr in &interface.addresses {
        if addr.kind == Kind::Ipv4 {
            return Ok(addr.addr.unwrap().ip());
        }
    }

    Err(ErrorKind::IPv4NotFound)
}

/// Retrieve CPU package temperature.
fn cpu_temp() -> Result<f32, ErrorKind> {
    let mut temp = String::new();

    match File::open(THERMAL_ZONE) {
        Ok(mut file) => file
            .read_to_string(&mut temp)
            .expect("unable to read `{THERMAL_ZONE}` file"),
        Err(_) => return Err(ErrorKind::InaccessibleFile(THERMAL_ZONE)),
    };

    Ok(temp
        .trim()
        .parse::<f32>()
        .expect("unable to parse `{THERMAL_ZONE}` content to f32")
        / 1000.0)
}

/// Retrieve environment humidity and temperature from stdin.
fn hum_temp(data: &str) -> Result<(f32, f32), ErrorKind> {
    let splits = data.split(',').collect::<Vec<&str>>();
    // if splits
}

/// Check for running process (`pgrep`).
fn pgrep(name: &str) -> Result<bool, ErrorKind> {
    let proc_list = match all_processes() {
        Ok(proc_list) => proc_list,
        Err(_) => return Err(ErrorKind::ProcListErr),
    };

    for proc in proc_list {
        if let Ok(exe_path) = proc.unwrap().exe() {
            if exe_path.file_stem().unwrap() == name {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Run application and catch errors.
fn run() -> Result<(), ErrorKind> {
    let (tx, rx) = mpsc::channel::<(f32, f32)>();

    let hum_temp_thread_handle = thread::spawn(|| -> Result<(), ErrorKind> {
        for line in io::stdin().lock().lines() {
            let data = line.
            tx.send(hum_temp(&data)?).expect("unable to send hum_temp data between threads");
        }

        Ok(())
    });

    println!(
        "IP: {}\nCPU: {}°C\nBOMBUSCV: {}",
        local_ipv4()?,
        cpu_temp()?,
        if pgrep("youterm")? { "running" } else { "--" }
    );

    hum_temp_thread_handle.join().unwrap();
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

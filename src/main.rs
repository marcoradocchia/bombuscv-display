mod args;

use args::{Args, Parser};
use bombuscv_display::{local_ipv4, pgrep, Cpu, ErrorKind, I2cDisplay, Measure};
use chrono::Local;
use signal_hook::{consts::SIGINT, flag::register};
use std::{
    io::{self, BufRead},
    net::{IpAddr, Ipv4Addr},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

/// Run application and catch errors.
fn run(args: &Args) -> Result<(), ErrorKind> {
    // Register signal-hook for SIGINT (Ctrl-C) events: in this case error is unrecoverable.
    let term = Arc::new(AtomicBool::new(false));
    if register(SIGINT, Arc::clone(&term)).is_err() {
        return Err(ErrorKind::SigIntHandlerErr);
    };

    // Initialize CPU info.
    let mut cpu = Cpu::new(&args.thermal)?;
    // Initialize I2C display.
    let mut i2c_display = I2cDisplay::new()?;

    // Sender/Receiver for measure values.
    let (tx_measure, rx_measure) = mpsc::channel();
    // Sender/Receiver for cpu values.
    let (tx_cpu, rx_cpu) = mpsc::channel();

    let measure_handle = thread::spawn(move || -> Result<(), ErrorKind> {
        // Read data from stdin (used in this case to pipe from datalogger, program).
        // https://github.com/marcoradocchia/datalogger
        for line in io::stdin().lock().lines() {
            if let Ok(line) = line {
                tx_measure
                    .send(Measure::from_csv(&line))
                    .expect("unable to send hum_temp data between threads");
            } else {
                return Err(ErrorKind::InvalidInput);
            }
        }

        Ok(())
    });

    let cpu_handle = thread::spawn(move || -> Result<(), ErrorKind<'_>> {
        // Retrieve CPU temperature and usage every 1 second and send it to main thread.
        // Start grabber loop: loop guard is `received SIGINT`.
        while !term.load(Ordering::Relaxed) {
            tx_cpu
                .send(cpu.read_info().unwrap())
                .expect("unable to send cpua data between threads");

            // Sleep 2 second.
            thread::sleep(Duration::from_secs(2));
        }
        Ok(())
    });

    // Grab the first measure.
    let mut measure: Measure = rx_measure
        .recv()
        .expect("unable to receive measure from measure thread")?;

    // This sets approx display refresh rate.
    for cpu_read in rx_cpu {
        // Don't wait for measure if it is not immediatly received.
        if let Ok(new_measure) = rx_measure.recv_timeout(Duration::ZERO) {
            measure = new_measure?
        }

        // Refresh I2C display.
        i2c_display.refresh_display(&format!(
            "{}\n{}\nIP: {}\nCPU: {}\nBOMBUSCV: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            measure,
            local_ipv4(&args.interface).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            cpu_read,
            if pgrep("bombuscv")? { "running" } else { "--" }
        ))?;
    }

    measure_handle
        .join()
        .expect("unable to join measure_handle thread")?;
    cpu_handle
        .join()
        .expect("unable to join cpu_handle thread")?;
    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

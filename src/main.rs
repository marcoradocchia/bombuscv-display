mod args;

use args::{Args, Parser};
use bombuscv_display::{cpu_temp, cpu_usage, local_ipv4, pgrep, ErrorKind, I2cDisplay, Measure};
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

    // Sender/Receiver for measure values.
    let (tx_measure, rx_measure) = mpsc::channel();

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

    // Initialize I2C  display.
    let mut i2c_display = I2cDisplay::new()?;

    // Grab the first measure.
    let mut measure: Measure = rx_measure
        .recv()
        .expect("unable to receive measure from measure thread")?;

    // Start grabber loop: loop guard is `received SIGINT`.
    while !term.load(Ordering::Relaxed) {
        // This sets approx display refresh rate.
        if let Ok(new_measure) = rx_measure.recv_timeout(Duration::from_secs(1)) {
            measure = new_measure?
        }

        // Refresh I2C display.
        i2c_display.refresh_display(&format!(
            "{}\n{}\nIP: {}\nCPU: {}% {:.1}C\nBOMBUSCV: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            measure,
            local_ipv4(&args.interface).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            cpu_usage()?,
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

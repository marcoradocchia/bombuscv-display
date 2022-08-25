use anyhow::{anyhow, Result};
use std::{
    fmt::{self, Display, Formatter},
    ops::Index,
};

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

    /// Construct `Measure` from CSV string <hum,temp>.
    pub fn from_csv(data: &str) -> Result<Self> {
        // TODO: panics on invalid input format
        let splits: Vec<f32> = data.split(',').map(|val| val.parse().unwrap()).collect();

        // Invalid input from datalogger (not <hum,temp> format).
        if !splits.len() == 2 {
            return Err(anyhow!(
                "invalid input format; please use `<hum>,<temp>` instead"
            ));
        }

        Ok(Measure::new(*splits.index(0), *splits.index(1)))
    }
}

impl Default for Measure {
    fn default() -> Self {
        Self {
            humidity: 0.0,
            temperature: 0.0,
        }
    }
}

impl Display for Measure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "H: {}% T: {}C", self.humidity, self.temperature)
    }
}

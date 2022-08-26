use crate::{ErrorKind, Result};
use std::fmt::{self, Display, Formatter};

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
        let splits: Vec<&str> = data.split(',').collect();

        if !splits.len() == 2 {
            return Err(ErrorKind::InvalidInputFormat);
        }

        let parse_f32 = |val: &str| -> Result<f32> {
            val.parse::<f32>()
                .map_err(|_| ErrorKind::InvalidInputFormat)
        };

        Ok(Measure::new(parse_f32(splits[0])?, parse_f32(splits[1])?))
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

use crate::{ErrorKind, Result};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use rppal::i2c::I2c;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

pub struct I2cDisplay {
    disp: Ssd1306<I2CInterface<I2c>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
}

impl I2cDisplay {
    /// Setup & initialize SH1106 I2C display.
    pub fn new(brightness: Brightness) -> Result<Self> {
        // TODO: change here to let the user specify custom pins?
        let mut disp = Ssd1306::new(
            I2CDisplayInterface::new(I2c::new().map_err(ErrorKind::I2cAccessErr)?),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        disp.init().map_err(|_| ErrorKind::I2cInitErr)?;
        disp.set_brightness(brightness) // Set display brightness.
            .map_err(|_| ErrorKind::I2cInitErr)?;
        disp.flush().map_err(|_| ErrorKind::I2cWriteErr)?;

        Ok(Self { disp })
    }

    /// Refresh display screen.
    pub fn refresh_display(&mut self, lines: &str) -> Result<()> {
        // Clear the display buffer.
        self.disp.clear();

        // Draw text to display and flush.
        Text::with_baseline(
            lines,
            Point::zero(),
            MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
            embedded_graphics::text::Baseline::Top,
        )
        .draw(&mut self.disp)
        .map_err(|_| ErrorKind::I2cWriteErr)?;

        self.disp.flush().map_err(|_| ErrorKind::I2cWriteErr)?;

        Ok(())
    }
}

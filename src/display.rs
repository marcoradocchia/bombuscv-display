// TODO: convert unwraps.
use anyhow::Result;
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
    /// Initialize & setup SH1106 I2C display.
    pub fn new() -> Result<Self> {
        // TODO: change here to let the user specify custom pins
        let mut disp = Ssd1306::new(
            I2CDisplayInterface::new(I2c::new()?),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        // Init & flush display.
        disp.init().unwrap();
        disp.flush().unwrap();

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
        .draw(&mut self.disp).unwrap();
        self.disp.flush().unwrap();

        Ok(())
    }
}

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_time::{Duration, Ticker};
use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_hal::{Async, i2c::master::I2c};
use heapless::format;
use ssd1306::{I2CDisplayInterface, mode::BufferedGraphicsModeAsync, prelude::*};
use ssd1306::{Ssd1306Async, rotation::DisplayRotation, size::DisplaySize128x64};

use crate::movement::last_speed_values;

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct SilvanoBotDisplay<'a> {
    display: Ssd1306Async<
        I2CInterface<I2c<'a, Async>>,
        DisplaySize128x64,
        BufferedGraphicsModeAsync<DisplaySize128x64>,
    >,
}

impl<'a> SilvanoBotDisplay<'a> {
    pub async fn new(bus: I2c<'a, Async>) -> Self {
        let interface = I2CDisplayInterface::new(bus);
        let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().await.expect("Can't init display");
        Self { display }
    }

    pub async fn update(&mut self) {
        let mut counter = COUNTER.load(Ordering::Relaxed);
        counter += 1;
        COUNTER.store(counter, Ordering::Relaxed);

        let (left, right) = last_speed_values();
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        self.display.clear(BinaryColor::Off).unwrap();
        let output = format!(40; "left = {}", left).expect("Can't format string");
        Text::with_baseline(&output, Point::zero(), text_style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();
        let output = format!(40; "right = {}", right).expect("Can't format string");
        Text::with_baseline(&output, Point::new(0, 11), text_style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();
        let output = format!(40; "count = {}", counter).expect("Can't format string");
        Text::with_baseline(&output, Point::new(0, 22), text_style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();
        self.display.flush().await.unwrap();
    }
}

#[embassy_executor::task]
pub async fn display_task(mut display: SilvanoBotDisplay<'static>) -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(16));
    loop {
        display.update().await;
        ticker.next().await;
    }
}

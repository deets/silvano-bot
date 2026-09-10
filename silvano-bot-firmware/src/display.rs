use core::sync::atomic::{AtomicU32, Ordering};

use embassy_time::{Duration, Ticker};
use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder},
    text::{Baseline, Text},
};
use esp_hal::{Async, i2c::master::I2c};
use heapless::{HistoryBuf, format};
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
    speed_values: HistoryBuf<(u8, u8), 128>,
}

impl<'a> SilvanoBotDisplay<'a> {
    pub async fn new(bus: I2c<'a, Async>) -> Self {
        let interface = I2CDisplayInterface::new(bus);
        let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().await.expect("Can't init display");
        Self {
            display,
            speed_values: HistoryBuf::new(),
        }
    }

    pub async fn update(&mut self) {
        let mut counter = COUNTER.load(Ordering::Relaxed);
        counter += 1;
        COUNTER.store(counter, Ordering::Relaxed);
        let display = &mut self.display;

        self.speed_values.write(last_speed_values());
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        display.clear(BinaryColor::Off).unwrap();
        let output = format!(40; "count = {}", counter).expect("Can't format string");
        Text::with_baseline(&output, Point::zero(), text_style, Baseline::Top)
            .draw(display)
            .unwrap();
        // The display is 64 pixels high, we remove 16 for the top line, from the 48
        // we can make two rows ~24 pixels, the spread of values between 0..255
        // is ~11

        let style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        for (x, (left, right)) in self.speed_values.oldest_ordered().enumerate() {
            Circle::new(Point::new(x as i32, 16 + (*left as i32) / 11), 1)
                .into_styled(style)
                .draw(display)
                .unwrap();
            Circle::new(Point::new(x as i32, 16 + 24 + (*right as i32) / 11), 1)
                .into_styled(style)
                .draw(display)
                .unwrap();
        }
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

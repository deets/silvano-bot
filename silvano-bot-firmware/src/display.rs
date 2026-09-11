use core::sync::atomic::AtomicU32;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use embassy_time::{Duration, Ticker};
use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyleBuilder},
    text::{Baseline, Text},
};
use esp_hal::{Async, i2c::master::I2c};
use heapless::{HistoryBuf, format};
use ssd1306::{I2CDisplayInterface, mode::BufferedGraphicsModeAsync, prelude::*};
use ssd1306::{Ssd1306Async, rotation::DisplayRotation, size::DisplaySize128x64};

use crate::movement::{MD23State, codepos, last_error};

type DisplayType<'a> = Ssd1306Async<
    I2CInterface<I2c<'a, Async>>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub struct SilvanoBotDisplay<'a> {
    display: DisplayType<'a>,
    state_values: HistoryBuf<MD23State, 128>,
    receiver: Receiver<'static, NoopRawMutex, MD23State, 16>,
    last_index: Option<usize>,
    progress: usize,
    liveness: usize,
}

impl<'a> SilvanoBotDisplay<'a> {
    pub async fn new(
        bus: I2c<'a, Async>,
        receiver: Receiver<'static, NoopRawMutex, MD23State, 16>,
    ) -> Self {
        let interface = I2CDisplayInterface::new(bus);
        let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().await.expect("Can't init display");
        Self {
            display,
            state_values: HistoryBuf::new(),
            receiver,
            last_index: None,
            progress: 0,
            liveness: 0,
        }
    }

    pub async fn update(&mut self) {
        let ((sx, sy), (ex, ey)) = self.movement_task_progress_indicator();
        let ((dsx, dsy), (dex, dey)) = self.display_task_progress_indicator();
        self.update_state().await;

        let display = &mut self.display;
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        display.clear(BinaryColor::Off).unwrap();
        let output = if let Some(error) = last_error() {
            format!(40; "Error: {}, Codep: {}", error, codepos()).expect("Can't format string")
        } else {
            format!(40; "Codepos: {}", codepos()).expect("Can't format string")
        };

        Text::with_baseline(&output, Point::new(4, 0), text_style, Baseline::Top)
            .draw(display)
            .unwrap();
        // The display is 64 pixels high, we remove 16 for the top line, from the 48
        // we can make two rows ~24 pixels, the spread of values between 0..255
        // is ~11
        let style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        for (x, state) in self.state_values.oldest_ordered().enumerate() {
            let (left, right) = state.speeds;
            Circle::new(Point::new(x as i32, 16 + (left as i32) / 11), 1)
                .into_styled(style)
                .draw(display)
                .unwrap();
            Circle::new(Point::new(x as i32, 16 + 24 + (right as i32) / 11), 1)
                .into_styled(style)
                .draw(display)
                .unwrap();
        }

        Line::new(Point::new(126 + sx, 1 + sy), Point::new(126 + ex, 1 + ey))
            .into_styled(style)
            .draw(display)
            .unwrap();

        Line::new(Point::new(dsx, dsy), Point::new(dex, dey))
            .into_styled(style)
            .draw(display)
            .unwrap();

        display.flush().await.unwrap();
    }

    fn movement_task_progress_indicator(&mut self) -> ((i32, i32), (i32, i32)) {
        // It appears as if the movement taks dies, to debug this
        // progressi is only made when the state values change
        let current_index = self.state_values.recent_index();
        if current_index != self.last_index {
            self.progress += 1;
        }
        self.last_index = current_index;
        let pos = self.progress % 8;
        // Going in a 3x3 circle
        let (sx, sy) = match pos {
            0 => (-1, -1),
            1 => (-1, 0),
            2 => (-1, 1),
            3 => (0, 1),
            4 => (1, 1),
            5 => (1, 0),
            6 => (1, -1),
            7 => (0, -1),
            _ => unreachable!(),
        };
        ((sx, sy), (-sx, -sy))
    }

    fn display_task_progress_indicator(&mut self) -> ((i32, i32), (i32, i32)) {
        self.liveness += 1;
        // Going in a 3x3 circle
        let v = self.liveness as i32 % 14;
        if v < 8 {
            ((0, v), (2, v))
        } else {
            ((0, 14 - v), (2, 14 - v))
        }
    }

    async fn update_state(&mut self) {
        while !self.receiver.is_empty() {
            let state = self.receiver.receive().await;
            self.state_values.write(state);
        }
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

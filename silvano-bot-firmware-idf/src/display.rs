use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyleBuilder},
    text::{Baseline, Text},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use ssd1306::{I2CDisplayInterface, mode::BufferedGraphicsMode, prelude::*};
use ssd1306::{Ssd1306, rotation::DisplayRotation, size::DisplaySize128x64};

type DisplayType<'a> = Ssd1306<
    I2CInterface<I2cDriver<'a>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub struct SilvanoBotDisplay<'a> {
    display: DisplayType<'a>,
    last_index: Option<usize>,
    progress: usize,
    liveness: usize,
}

impl<'a> SilvanoBotDisplay<'a> {
    pub fn new(bus: I2cDriver<'a>) -> Self {
        let interface = I2CDisplayInterface::new(bus);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().expect("Can't init display");
        Self {
            display,
            last_index: None,
            progress: 0,
            liveness: 0,
        }
    }

    pub fn update(&mut self) {
        // let ((sx, sy), (ex, ey)) = self.movement_task_progress_indicator();
        let ((dsx, dsy), (dex, dey)) = self.display_task_progress_indicator();
        // self.update_state().await;

        let display = &mut self.display;
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        display.clear(BinaryColor::Off).unwrap();
        // let output = if let Some(error) = last_error() {
        //     format!(40; "Error: {}, Codep: {}", error, codepos()).expect("Can't format string")
        // } else {
        //     format!(40; "Codepos: {}", codepos()).expect("Can't format string")
        // };

        Text::with_baseline("Test", Point::new(4, 0), text_style, Baseline::Top)
            .draw(display)
            .unwrap();
        // // The display is 64 pixels high, we remove 16 for the top line, from the 48
        // // we can make two rows ~24 pixels, the spread of values between 0..255
        // // is ~11
        let style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        // for (x, state) in self.state_values.oldest_ordered().enumerate() {
        //     let (left, right) = state.speeds;
        //     Circle::new(Point::new(x as i32, 16 + (left as i32) / 11), 1)
        //         .into_styled(style)
        //         .draw(display)
        //         .unwrap();
        //     Circle::new(Point::new(x as i32, 16 + 24 + (right as i32) / 11), 1)
        //         .into_styled(style)
        //         .draw(display)
        //         .unwrap();
        // }

        // Line::new(Point::new(126 + sx, 1 + sy), Point::new(126 + ex, 1 + ey))
        //     .into_styled(style)
        //     .draw(display)
        //     .unwrap();

        Line::new(Point::new(dsx, dsy), Point::new(dex, dey))
            .into_styled(style)
            .draw(display)
            .unwrap();

        display.flush().unwrap();
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
}

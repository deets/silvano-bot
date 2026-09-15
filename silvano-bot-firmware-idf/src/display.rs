use std::sync::{Arc, Mutex};

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

use crate::{eyes::draw_eye, movement::Movement};

type DisplayType<'a> = Ssd1306<
    I2CInterface<I2cDriver<'a>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub struct SilvanoBotDisplay<'a> {
    display: DisplayType<'a>,
    eye_movement: Arc<Mutex<Movement>>,
    liveness: usize,
}

impl<'a> SilvanoBotDisplay<'a> {
    pub fn new(bus: I2cDriver<'a>, eye_movement: Arc<Mutex<Movement>>) -> Self {
        let interface = I2CDisplayInterface::new(bus);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().expect("Can't init display");
        Self {
            display,
            eye_movement,
            liveness: 0,
        }
    }

    pub fn update(&mut self) {
        let (left, right) = self.eye_movement();
        let ((dsx, dsy), (dex, dey)) = self.display_task_progress_indicator();
        let display = &mut self.display;
        display.clear(BinaryColor::Off).unwrap();
        let style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        Line::new(Point::new(dsx, dsy), Point::new(dex, dey))
            .into_styled(style)
            .draw(display)
            .unwrap();
        draw_eye(left, 0.0, Point::new(8, 8), 48, display).unwrap();
        draw_eye(right, 0.0, Point::new(128 - 48 - 8 - 1, 8), 48, display).unwrap();
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

    fn eye_movement(&self) -> (f64, f64) {
        let guard = self.eye_movement.lock().unwrap();
        (guard.left as f64, guard.right as f64)
    }
}

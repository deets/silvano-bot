use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use embedded_graphics::{
    geometry::Point,
    image::{Image, ImageRaw},
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

include!(concat!(env!("OUT_DIR"), "/pcb.rs"));

type DisplayType<'a> = Ssd1306<
    I2CInterface<I2cDriver<'a>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub struct SilvanoBotDisplay<'a> {
    display: DisplayType<'a>,
    eye_movement: Arc<Mutex<Movement>>,
    liveness: usize,
    start: Instant,
}

// For a nice bouncing effect, this should
// return a value that spans
//   0..max
// and oscillates from 0 -> max - 1 -> 0
// in a linear fashion.
fn mirror(v: i32, max: i32) -> i32 {
    let segment = v % (max * 2);
    if segment > (max - 1) {
        max + (max - segment) - 1
    } else {
        segment
    }
}

const RAW_IMAGE: ImageRaw<BinaryColor> = ImageRaw::<BinaryColor>::new(IMAGE, IMAGE_WIDTH);

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
            start: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let offset = (self.start.elapsed().as_millis() / 50) as i32;
        let image_x = mirror(offset, 256 - 128);
        let image_y = mirror(offset, 256 - 64);
        let image = Image::new(&RAW_IMAGE, Point::new(-image_x, -image_y));

        let (left_right, top_down) = self.eye_movement();
        let display = &mut self.display;
        display.clear(BinaryColor::Off).unwrap();
        image.draw(display).unwrap();
        draw_eye(left_right, top_down, Point::new(8, 8), 48, display).unwrap();
        draw_eye(
            left_right,
            top_down,
            Point::new(128 - 48 - 8 - 1, 8),
            48,
            display,
        )
        .unwrap();
        display.flush().unwrap();
    }

    fn eye_movement(&self) -> (f64, f64) {
        let guard = self.eye_movement.lock().unwrap();
        (-guard.x as f64, guard.y as f64)
    }
}

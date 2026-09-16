//! # Example: 7-Segment Digital Clock
//!
//! An example displaying a digital clock using the `eg-seven-segment` crate.

use std::{
    f64::consts::PI,
    time::{Duration, Instant},
};

use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use eyes::draw_eye;

mod eyes;

include!(concat!(env!("OUT_DIR"), "/pcb.rs"));

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

fn main() {
    let mut display = SimulatorDisplay::<BinaryColor>::new(Size::new(128, 64));

    let settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledWhite)
        .build();

    let mut window = Window::new("Digital clock", &settings);
    let start = Instant::now();
    let raw_image = ImageRaw::<BinaryColor>::new(IMAGE, IMAGE_WIDTH);

    loop {
        let left_right = (start.elapsed().as_secs_f64() * 2.0 * PI * 0.5).sin();
        let top_down = (start.elapsed().as_secs_f64() * 2.0 * PI * 0.2).sin();

        let offset = (start.elapsed().as_millis() / 50) as i32;
        let image_x = mirror(offset, 256 - 128);
        let image_y = mirror(offset, 256 - 64);
        let image = Image::new(&raw_image, Point::new(-image_x, -image_y));

        display.clear(BinaryColor::Off).unwrap();
        image.draw(&mut display).unwrap();
        draw_eye(left_right, top_down, Point::new(8, 8), 48, &mut display).unwrap();
        draw_eye(
            left_right,
            top_down,
            Point::new(128 - 48 - 8 - 1, 8),
            48,
            &mut display,
        )
        .unwrap();

        window.update(&mut display);

        if window.events().any(|event| event == SimulatorEvent::Quit) {
            break;
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror() {
        let results: Vec<_> = (0..16).map(|v| mirror(v, 8)).collect();
        assert_eq!(
            results,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 7, 6, 5, 4, 3, 2, 1, 0]
        );
    }
}

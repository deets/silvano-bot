//! # Example: 7-Segment Digital Clock
//!
//! An example displaying a digital clock using the `eg-seven-segment` crate.

use std::{
    f64::consts::PI,
    time::{Duration, Instant},
};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use eyes::draw_eye;

mod eyes;

fn main() {
    let mut display = SimulatorDisplay::<BinaryColor>::new(Size::new(128, 64));

    let settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledWhite)
        .build();

    let mut window = Window::new("Digital clock", &settings);
    let start = Instant::now();
    loop {
        let left_right = (start.elapsed().as_secs_f64() * 2.0 * PI * 0.5).sin();
        let top_down = (start.elapsed().as_secs_f64() * 2.0 * PI * 0.2).sin();
        display.clear(BinaryColor::Off).unwrap();
        draw_eye(left_right, top_down, Point::new(8, 8), 48, &mut display).unwrap();
        draw_eye(
            left_right,
            top_down,
            Point::new(128 - 48 - 8 - 1, 8),
            48,
            &mut display,
        )
        .unwrap();

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline(
            &format!("LR: {:.3}, TD: {:.3}", left_right, top_down),
            Point::new(0, 0),
            text_style,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();

        window.update(&mut display);

        if window.events().any(|event| event == SimulatorEvent::Quit) {
            break;
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}

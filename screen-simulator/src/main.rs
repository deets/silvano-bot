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
    primitives::{Circle, PrimitiveStyleBuilder},
    text::{Baseline, Text},
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};

/// Draws a digital clock with the current local time to the specified display
fn draw_eye<D>(
    left_right: f64,
    top_down: f64,
    top_left: Point,
    diameter: u32,
    display: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let outline_style = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .build();
    Circle::new(top_left, diameter)
        .into_styled(outline_style)
        .draw(display)?;

    let filled_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::On)
        .build();
    let pupil_diameter = diameter as i32 / 3;

    // Clamp the values so that the pupil
    // never leaves the eye.
    let rad = top_down.atan2(left_right);
    let mut length = left_right.powi(2) + top_down.powi(2).sqrt();
    length = if length > 1.0 { 1.0 } else { length };
    let left_right = rad.cos() * length;
    let top_down = rad.sin() * length;

    let left_right_offset =
        ((diameter as f64 / 2.0 - 1.0 - pupil_diameter as f64 / 2.0) * left_right) as i32;
    let top_down_offset =
        ((diameter as f64 / 2.0 - 1.0 - pupil_diameter as f64 / 2.0) * top_down) as i32;

    let pupil_tl = top_left
        + Point::new(
            diameter as i32 / 2 - pupil_diameter / 2 + left_right_offset,
            diameter as i32 / 2 - pupil_diameter / 2 + top_down_offset,
        );
    Circle::new(pupil_tl, pupil_diameter as u32)
        .into_styled(filled_style)
        .draw(display)?;

    let highlight_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::Off)
        .build();
    let highlight_diameter = pupil_diameter / 4;
    let highlight_tl = pupil_tl + Point::new(highlight_diameter, highlight_diameter);
    Circle::new(highlight_tl, highlight_diameter as u32)
        .into_styled(highlight_style)
        .draw(display)?;

    Ok(())
}

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

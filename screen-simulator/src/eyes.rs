use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder},
};
use num_traits::NumCast;

/// Draws a digital clock with the current local time to the specified display
pub fn draw_eye<D, T>(
    left_right: T,
    top_down: T,
    top_left: Point,
    diameter: u32,
    display: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
    T: num_traits::real::Real + From<i32>,
{
    let outline_style = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .fill_color(BinaryColor::Off)
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
    length = if length > T::one() { T::one() } else { length };
    let left_right = rad.cos() * length;
    let top_down = rad.sin() * length;
    let two = T::one() + T::one();
    let left_right_offset = <i32 as NumCast>::from(
        (<T as NumCast>::from(diameter).unwrap() / two
            - T::one()
            - <T as NumCast>::from(pupil_diameter).unwrap() / two)
            * left_right,
    )
    .unwrap();
    let top_down_offset = <i32 as NumCast>::from(
        (<T as NumCast>::from(diameter).unwrap() / two
            - T::one()
            - <T as NumCast>::from(pupil_diameter).unwrap() / two)
            * top_down,
    )
    .unwrap();

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

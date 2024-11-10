use core::f64;

use crate::{interval::Interval, vec3::Vec3};

pub const PI: f64 = std::f64::consts::PI;
pub const INFINITY: f64 = f64::INFINITY;

#[inline]
pub fn degrees_to_rads(degrees: f64) -> f64 {
    degrees * PI / 180f64
}

#[inline]
pub fn random_float() -> f64 {
    fastrand::f64()
}

#[inline]
pub fn random_float_in_range(min: f64, max: f64) -> f64 {
    return min + (max - min) * random_float();
}

#[inline]
pub fn linear_to_gamma(linear_component: f64) -> f64 {
    if linear_component > 0f64 {
        return f64::sqrt(linear_component);
    } else {
        return 0f64;
    }
}

pub fn write_rgb8_color_as_text_to_stream(vec: &Vec3, stream: &mut dyn std::io::Write) {
    let r = linear_to_gamma(vec.x);
    let g = linear_to_gamma(vec.y);
    let b = linear_to_gamma(vec.z);

    let intensity = Interval::new(0f64, 0.999f64);

    let r_byte: u8 = (intensity.clamp(r) * 256f64) as u8;
    let g_byte: u8 = (intensity.clamp(g) * 256f64) as u8;
    let b_byte: u8 = (intensity.clamp(b) * 256f64) as u8;

    stream
        .write(format!("{} {} {}\n", r_byte, g_byte, b_byte).as_bytes())
        .expect("Unable to write to stream");
}

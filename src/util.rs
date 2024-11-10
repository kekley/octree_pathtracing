use core::f64;

pub const PI: f64 = std::f64::consts::PI;
pub const INFINITY: f64 = f64::INFINITY;

pub fn degrees_to_rads(degrees: f64) -> f64 {
    degrees * PI / 180f64
}

use core::f64;

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

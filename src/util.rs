use core::f64;

use fastrand::Rng;

use crate::{interval::Interval, vec3::Vec3};

pub const PI: f64 = std::f64::consts::PI;
pub const INFINITY: f64 = f64::INFINITY;

#[inline]
pub fn degrees_to_rads(degrees: f64) -> f64 {
    degrees * PI / 180f64
}

#[inline]
pub fn random_float(rng: &mut Rng) -> f64 {
    rng.f64()
}

#[inline]
pub fn random_float_in_range(rng: &mut Rng, min: f64, max: f64) -> f64 {
    return min + (max - min) * random_float(rng);
}

#[inline]
pub fn linear_to_gamma(linear_component: f64) -> f64 {
    if linear_component > 0f64 {
        return f64::sqrt(linear_component);
    } else {
        return 0f64;
    }
}
#[inline]
pub fn random_vec(rng: &mut Rng) -> Vec3 {
    Vec3::new(random_float(rng), random_float(rng), random_float(rng))
}
#[inline]

pub fn random_vec_in_range(rng: &mut Rng, min: f64, max: f64) -> Vec3 {
    Vec3::new(
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
    )
}
#[inline]

pub fn random_unit_vec(rng: &mut Rng) -> Vec3 {
    loop {
        let p = random_vec_in_range(rng, -1f64, 1f64);
        let len_sq = p.length_squared();

        if 1e-160 < len_sq && len_sq <= 1f64 {
            return p / f64::sqrt(len_sq);
        }
    }
}
#[inline]
pub fn new_rand_unit_vec(rng: &mut Rng) -> Vec3 {
    random_vec(rng).normalize()
}
#[inline]
pub fn random_on_hemisphere(rng: &mut Rng, normal: Vec3) -> Vec3 {
    let on_sphere = random_unit_vec(rng);
    if on_sphere.dot(normal) > 0f64 {
        return on_sphere;
    } else {
        return -on_sphere;
    }
}

#[inline]
pub fn random_in_unit_disk(rng: &mut Rng) -> Vec3 {
    loop {
        let p = Vec3::new(
            random_float_in_range(rng, -1.0, 1.0),
            random_float_in_range(rng, -1.0, 1.0),
            0.0,
        );

        if p.length_squared() < 1.0 {
            return p;
        }
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

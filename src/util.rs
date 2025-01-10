use core::f32;

use glam::Vec3A as Vec3;
use rand::{rngs::StdRng, Rng, RngCore};

use crate::{axis::Axis, interval::Interval};

pub const PI: f32 = std::f32::consts::PI;
#[inline]
pub fn degrees_to_rads(degrees: f32) -> f32 {
    degrees * PI / 180f32
}

#[inline]
pub fn random_float(rng: &mut StdRng) -> f32 {
    rng.gen::<f32>()
}

#[inline]
pub fn random_int(rng: &mut StdRng, min: i64, max: i64) -> i64 {
    random_float(rng) as i64
}

#[inline]
pub fn random_float_in_range(rng: &mut StdRng, min: f32, max: f32) -> f32 {
    return min + (max - min) * random_float(rng);
}

#[inline]
pub fn linear_to_gamma(linear_component: f32) -> f32 {
    if linear_component > 0f32 {
        return f32::sqrt(linear_component);
    } else {
        return 0f32;
    }
}
#[inline]
pub fn random_vec(rng: &mut StdRng) -> Vec3 {
    Vec3::new(random_float(rng), random_float(rng), random_float(rng))
}
#[inline]

pub fn random_vec_in_range(rng: &mut StdRng, min: f32, max: f32) -> Vec3 {
    Vec3::new(
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
    )
}
#[inline]

pub fn random_unit_vec(rng: &mut StdRng) -> Vec3 {
    loop {
        let p = random_vec_in_range(rng, -1f32, 1f32);
        let len_sq = p.length_squared();

        if 1e-160 < len_sq && len_sq <= 1f32 {
            return p / f32::sqrt(len_sq);
        }
    }
}
#[inline]
pub fn random_on_hemisphere(rng: &mut StdRng, normal: Vec3) -> Vec3 {
    let on_sphere = random_unit_vec(rng);
    if on_sphere.dot(normal) > 0f32 {
        return on_sphere;
    } else {
        return -on_sphere;
    }
}

#[inline]
pub fn step(edge: f32, x: f32) -> f32 {
    match x <= edge {
        true => 0.0,
        false => 1.0,
    }
}

pub fn step_vec(edge: f32, x: Vec3) -> Vec3 {
    Vec3::new(step(edge, x.x), step(edge, x.y), step(edge, x.z))
}

#[inline]
pub fn random_in_unit_disk(rng: &mut StdRng) -> Vec3 {
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
pub fn write_rgb8_color_as_text_to_stream(vec: &glam::Vec3, stream: &mut dyn std::io::Write) {
    let r = linear_to_gamma(vec.x);
    let g = linear_to_gamma(vec.y);
    let b = linear_to_gamma(vec.z);

    let intensity = Interval::new(0f32, 0.999f32);

    let r_byte: u8 = (intensity.clamp(r) * 256f32) as u8;
    let g_byte: u8 = (intensity.clamp(g) * 256f32) as u8;
    let b_byte: u8 = (intensity.clamp(b) * 256f32) as u8;

    stream
        .write(format!("{} {} {}\n", r_byte, g_byte, b_byte).as_bytes())
        .expect("Unable to write to stream");
}
pub fn write_rgb8_color_to_stream(vec: &glam::Vec3, stream: &mut dyn std::io::Write) {
    let r = linear_to_gamma(vec.x);
    let g = linear_to_gamma(vec.y);
    let b = linear_to_gamma(vec.z);

    let intensity = Interval::new(0f32, 0.999f32);

    let r_byte: u8 = (intensity.clamp(r) * 256f32) as u8;
    let g_byte: u8 = (intensity.clamp(g) * 256f32) as u8;
    let b_byte: u8 = (intensity.clamp(b) * 256f32) as u8;
    let buf: [u8; 3] = [r_byte, g_byte, b_byte];
    stream.write(&buf).unwrap();
}

pub fn near_zero(vec3: &Vec3) -> bool {
    let s = 1e-8;
    vec3.x.abs() < s && vec3.y.abs() < s && vec3.z.abs() < s
}

pub fn defocus_disk_sample(rng: &mut StdRng, center: Vec3, disc_u: Vec3, disc_v: Vec3) -> Vec3 {
    let p = random_in_unit_disk(rng);
    center + (p.x * disc_u) + (p.y * disc_v)
}

#[inline]
pub fn sample_square(rng: &mut StdRng) -> Vec3 {
    Vec3::new(random_float(rng) - 0.5f32, random_float(rng) - 0.5f32, 0.0)
}

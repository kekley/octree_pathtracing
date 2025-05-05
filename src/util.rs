use core::f32;
use std::f32::consts::PI;

use rand::{rngs::StdRng, Rng};

use glam::{Vec2, Vec3A};
use rand_distr::{Distribution, UnitDisc};

#[inline]
pub fn degrees_to_rads(degrees: f32) -> f32 {
    degrees * PI / 180.0
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
    if linear_component > 0.0 {
        return f32::sqrt(linear_component);
    } else {
        return 0.0;
    }
}
#[inline]
pub fn random_vec(rng: &mut StdRng) -> Vec3A {
    Vec3A::new(random_float(rng), random_float(rng), random_float(rng))
}
#[inline]

pub fn random_vec_in_range(rng: &mut StdRng, min: f32, max: f32) -> Vec3A {
    Vec3A::new(
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
        random_float_in_range(rng, min, max),
    )
}
#[inline]

pub fn random_unit_vec(rng: &mut StdRng) -> Vec3A {
    loop {
        let p = random_vec_in_range(rng, -1.0, 1.0);
        let len_sq = p.length_squared();

        if 1e-160 < len_sq && len_sq <= 1.0 {
            return p / f32::sqrt(len_sq);
        }
    }
}
#[inline]
pub fn random_on_hemisphere(rng: &mut StdRng, normal: Vec3A) -> Vec3A {
    let on_sphere = random_unit_vec(rng);
    if on_sphere.dot(normal) > 0.0 {
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
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn step_vec(edge: f32, x: Vec3A) -> Vec3A {
    Vec3A::new(step(edge, x.x), step(edge, x.y), step(edge, x.z))
}

#[inline]
pub fn random_in_unit_disk(rng: &mut StdRng) -> Vec2 {
    let a: [f32; 2] = UnitDisc.sample(rng);
    Vec2::from_array(a)
}

pub fn near_zero(vec: &Vec3A) -> bool {
    let s = 1e-8;
    vec.x.abs() < s && vec.y.abs() < s && vec.z.abs() < s
}

pub fn defocus_disk_sample(rng: &mut StdRng, center: Vec3A, disc_u: Vec3A, disc_v: Vec3A) -> Vec3A {
    let p = random_in_unit_disk(rng);
    center + (p.x * disc_u) + (p.y * disc_v)
}

#[inline]
pub fn sample_square(rng: &mut StdRng) -> Vec2 {
    Vec2::new(random_float(rng) - 0.5, random_float(rng) - 0.5)
}
pub fn find_msb(mut x: i32) -> i32 {
    let mut res = -1;
    if x < 0 {
        x = !x;
    }
    for i in 0..32 {
        let mask = 0x80000000u32 as i32 >> i;
        if x & mask != 0 {
            res = 31 - i;
            break;
        }
    }
    res
}
pub fn find_msb_u32(x: u32) -> u32 {
    // Decide what to do when x is zero.
    // One common strategy is to define the msb of 0 as 0.
    if x == 0 {
        return -1i32 as u32;
    }
    // The bit-length of a u32 is 32 bits.
    // The built-in function `leading_zeros()` returns the number of zeros from the most significant bit down to the first 1.
    // For example, if x is 16 (0b0001_0000), then x.leading_zeros() returns 27.
    // Since the highest index in a 32-bit number is 31, subtracting gives us:
    //   31 - 27 = 4, which is indeed the index of the most significant bit (since 16 == 2^4).
    31 - x.leading_zeros()
}
pub fn angle_distance(a1: f32, a2: f32) -> f32 {
    let diff = (a1 - a2).abs() % (2.0 * PI);
    if diff > PI {
        2.0 * PI - diff
    } else {
        diff
    }
}

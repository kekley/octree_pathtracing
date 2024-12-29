use std::f32::INFINITY;

use crate::{vec3::Vec3, HitRecord};
#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_dir: Vec3,
    pub hit: HitRecord,
}

impl Ray {
    pub const EPSILON: f32 = 0.00000005;
    pub const OFFSET: f32 = 0.000001;

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
    #[inline]

    pub fn new(point: Vec3, direction: Vec3) -> Self {
        Self {
            origin: point,
            direction,
            inv_dir: 1.0 / direction,
            hit: HitRecord::default(),
        }
    }
    #[inline]
    pub fn create_at(point: Vec3, direction: Vec3, time: f32) -> Self {
        Self {
            origin: point,
            direction: direction,
            inv_dir: 1.0 / direction,
            hit: HitRecord {
                t: INFINITY,
                u: 0.0,
                v: 0.0,
                mat_idx: 0,
                outward_normal: Vec3::ZERO,
            },
        }
    }
}

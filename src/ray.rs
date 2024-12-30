use std::f32::INFINITY;

use crate::HitRecord;
use glam::Vec3A as Vec3;
#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_dir: Vec3,
    pub distance_travelled: f32,
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
            distance_travelled: 0.0,
        }
    }
}

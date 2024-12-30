use crate::{HittableIdx, Interval, Ray};
use glam::Vec3A as Vec3;

pub struct Translation {
    hittable_idx: HittableIdx,
    offset: Vec3,
    sin_theta: f32,
    cos_theta: f32,
}

impl Translation {
    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        todo!()
    }
}

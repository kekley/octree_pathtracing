use glam::Vec3A;

use super::{hittable::HittableIdx, interval::Interval, ray::Ray};

pub struct Translation {
    hittable_idx: HittableIdx,
    offset: Vec3A,
    sin_theta: f32,
    cos_theta: f32,
}

impl Translation {
    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        todo!()
    }
}

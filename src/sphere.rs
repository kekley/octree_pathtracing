use std::f32::INFINITY;

use crate::aabb::AABB;
use crate::util::PI;
use crate::{interval::Interval, ray::Ray};
use glam::Vec3A as Vec3;

#[derive(Debug, Clone)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
    material_idx: u32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, material_idx: u32) -> Self {
        Self {
            center: center,
            radius: radius,
            material_idx,
        }
    }

    pub fn bbox(&self) -> AABB {
        let radius_vec: Vec3 = Vec3::splat(self.radius);

        let bbox = AABB::from_points(self.center - radius_vec, self.center + radius_vec);
        bbox
    }
    #[inline]
    pub fn hit(&self, ray: &mut Ray) -> bool {
        todo!("Implement Sphere::hit");
        let origin_to_center = self.center - ray.origin;
        let a = ray.direction.length_squared();
        let h = ray.direction.dot(origin_to_center);
        let c = origin_to_center.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;

        if discriminant < 0f32 {
            ray.hit.t = INFINITY;
            return false;
        }

        let sqrt_discriminant = discriminant.sqrt();

        let mut root = (h - sqrt_discriminant) / a;

        let point = ray.at(root);
        let t = root;
        let outward_normal = (point - self.center) / self.radius;
        let (u, v) = Self::get_uv(outward_normal);
        ray.hit.t = t;
        ray.hit.u = u;
        ray.hit.v = v;
        ray.hit.outward_normal = outward_normal;
        ray.hit.current_material = self.material_idx;
    }

    pub fn get_uv(point: Vec3) -> (f32, f32) {
        let theta = (-point.y).acos();

        let phi = (-point.z).atan2(point.x) + PI;

        let u = phi / (2.0 * PI);

        let v = theta / PI;
        (u, v)
    }
}

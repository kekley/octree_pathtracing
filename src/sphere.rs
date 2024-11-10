use std::cmp::max;
use std::sync::Arc;

use crate::material::Material;
use crate::{hittable::HitRecord, interval::Interval, ray::Ray, vec3::Vec3};

#[derive(Debug, Clone)]
pub struct Sphere {
    center: Vec3,
    radius: f64,
    material: Arc<Material>,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f64, material: Arc<Material>) -> Self {
        Self {
            center,
            radius: f64::max(radius, 0f64),
            material: material,
        }
    }

    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let origin_to_center = self.center - ray.origin;
        let a = ray.direction.length_squared();
        let h = ray.direction.dot(origin_to_center);
        let c = origin_to_center.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;

        if discriminant < 0f64 {
            return None;
        }

        let sqrt_discriminant = f64::sqrt(discriminant);

        let mut root = (h - sqrt_discriminant) / a;

        if !ray_t.surrounds(root) {
            root = (h + sqrt_discriminant) / a;

            if !ray_t.surrounds(root) {
                return None;
            }
        }

        let point = ray.at(root);
        let t = root;
        let outward_normal = (point - self.center) / self.radius;

        let mut hit_record = HitRecord {
            point,
            normal: Vec3::default(),
            t,
            front_face: false,
            material: self.material.clone(),
        };

        hit_record.set_face_normal(&ray, outward_normal);

        Some(hit_record)
    }
}

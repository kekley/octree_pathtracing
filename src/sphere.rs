use crate::aabb::AABB;
use crate::material::Material;
use crate::util::PI;
use crate::{hittable::HitRecord, interval::Interval, ray::Ray, vec3::Vec3};

#[derive(Debug, Clone)]
pub struct Sphere {
    center: Ray,
    radius: f64,
    material_idx: u16,
    pub bbox: AABB,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f64, material_idx: u16) -> Self {
        let radius_vec = Vec3::splat(radius);
        let bbox = AABB::from_points(center - radius_vec, center + radius_vec);
        Self {
            center: Ray::new(center, Vec3::ZERO),
            radius: f64::max(radius, 0f64),
            material_idx,
            bbox,
        }
    }
    pub fn new_moving(center1: Vec3, center2: Vec3, radius: f64, material_idx: u16) -> Self {
        let center_ray = Ray::new(center1, center2 - center1);
        let radius_vec = Vec3::splat(radius);
        let box1 = AABB::from_points(
            center_ray.at(0.0) - radius_vec,
            center_ray.at(0.0) + radius_vec,
        );

        let box2 = AABB::from_points(
            center_ray.at(1.0) - radius_vec,
            center_ray.at(1.0) + radius_vec,
        );
        let bbox = AABB::from_boxes(&box1, &box2);
        Self {
            center: center_ray,
            radius: radius.max(0.0),
            material_idx,
            bbox,
        }
    }
    #[inline]
    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let current_center = self.center.at(ray.time);
        let origin_to_center = current_center - ray.origin;
        let a = ray.direction.length_squared();
        let h = ray.direction.dot(origin_to_center);
        let c = origin_to_center.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;

        if discriminant < 0f64 {
            return None;
        }

        let sqrt_discriminant = discriminant.sqrt();

        let mut root = (h - sqrt_discriminant) / a;

        if !ray_t.surrounds(root) {
            root = (h + sqrt_discriminant) / a;

            if !ray_t.surrounds(root) {
                return None;
            }
        }

        let point = ray.at(root);
        let t = root;
        let outward_normal = (point - current_center) / self.radius;
        let (u, v) = Self::get_uv(outward_normal);
        let mut rec = HitRecord {
            point,
            normal: Vec3::default(),
            t,
            front_face: false,
            material_idx: self.material_idx,
            u,
            v,
        };

        rec.set_face_normal(&ray, outward_normal);

        Some(rec)
    }

    pub fn get_uv(point: Vec3) -> (f64, f64) {
        let theta = (-point.y).acos();

        let phi = (-point.z).atan2(point.x) + PI;

        let u = phi / (2.0 * PI);

        let v = theta / PI;
        (u, v)
    }
}

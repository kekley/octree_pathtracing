use std::f32::INFINITY;

use fastrand::Rng;

use crate::{
    hittable::HitRecord,
    interval::{self, Interval},
    ray::Ray,
    util::random_float,
    vec3::Vec3,
};

#[derive(Debug, Clone)]
pub struct AABB {
    pub x_interval: Interval,
    pub y_interval: Interval,
    pub z_interval: Interval,
}

impl Default for AABB {
    fn default() -> Self {
        AABB {
            x_interval: Interval::EMPTY,
            y_interval: Interval::EMPTY,
            z_interval: Interval::EMPTY,
        }
    }
}

impl AABB {
    pub const EMPTY: AABB = AABB::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
    pub const UNIVERSE: AABB =
        AABB::new(Interval::UNIVERSE, Interval::UNIVERSE, Interval::UNIVERSE);
    pub fn longest_axis(&self) -> u8 {
        if self.x_interval.size() > self.y_interval.size() {
            if self.x_interval.size() > self.z_interval.size() {
                return 0;
            } else {
                return 2;
            }
        } else {
            if self.y_interval.size() > self.z_interval.size() {
                return 1;
            } else {
                return 2;
            }
        }
    }
    #[inline]
    pub const fn new(interval_x: Interval, interval_y: Interval, interval_z: Interval) -> Self {
        Self {
            x_interval: interval_x,
            y_interval: interval_y,
            z_interval: interval_z,
        }
    }
    pub fn from_boxes(a: &AABB, b: &AABB) -> Self {
        let x = Interval::from_intervals(&a.x_interval, &b.x_interval);
        let y = Interval::from_intervals(&a.y_interval, &b.y_interval);
        let z = Interval::from_intervals(&a.z_interval, &b.z_interval);
        Self {
            x_interval: x,
            y_interval: y,
            z_interval: z,
        }
    }
    pub fn from_points(a: Vec3, b: Vec3) -> Self {
        let x_interval = Interval::new(f64::min(a.x, b.x), f64::max(a.x, b.x));
        let y_interval = Interval::new(f64::min(a.y, b.y), f64::max(a.y, b.y));
        let z_interval = Interval::new(f64::min(a.z, b.z), f64::max(a.z, b.z));

        Self {
            x_interval,
            y_interval,
            z_interval,
        }
    }

    pub fn get_interval(&self, n: u8) -> &Interval {
        if n == 1 {
            return &self.y_interval;
        }
        if n == 2 {
            return &self.z_interval;
        }

        return &self.x_interval;
    }

    pub fn hit(&self, ray: &Ray, mut ray_t: Interval) -> Option<HitRecord> {
        let ray_origin: &Vec3 = &ray.origin;
        let ray_dir: &Vec3 = &ray.direction;
        for axis in 0..3 {
            let axis_interval = self.get_interval(axis);
            let axis_dir_inverse = 1.0 / ray_dir.get_axis(axis);

            let t0 = (axis_interval.min - ray_origin.get_axis(axis)) * axis_dir_inverse;
            let t1 = (axis_interval.max - ray_origin.get_axis(axis)) * axis_dir_inverse;

            if t0 < t1 {
                ray_t.min = t0.max(ray_t.min);

                ray_t.max = t1.min(ray_t.max);
            } else {
                ray_t.min = t1.max(ray_t.min);

                ray_t.max = t0.min(ray_t.max);
            }
            if ray_t.max <= ray_t.min {
                return None;
            }
        }
        let mut rng = Rng::new();
        let mut rec = HitRecord {
            point: ray.at(ray_t.min),
            normal: Vec3::splat(1.0),
            t: ray_t.min,
            front_face: true,
            material: crate::material::Material::Lambertian {
                albedo: Vec3::splat(random_float(&mut rng)),
            },
        };
        return Some(rec);
    }
}

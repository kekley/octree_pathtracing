use core::f32;
use std::f32::INFINITY;

use crate::{
    hittable::HitRecord,
    interval::{self, Interval},
    ray::Ray,
    vec3::{Axis, Vec3},
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
    #[inline]
    pub fn longest_axis(&self) -> Axis {
        let longest = self.x_interval.size().max(self.y_interval.size());
        let longest = longest.max(self.z_interval.size());
        if longest == self.x_interval.size() {
            Axis::X
        } else if longest == self.y_interval.size() {
            Axis::Y
        } else {
            Axis::Z
        }
    }
    #[inline]
    pub fn area(&self) -> f32 {
        let size_x = self.x_interval.size();
        let size_y = self.y_interval.size();
        let size_z = self.z_interval.size();
        2.0 * (size_x * size_y + size_x * size_z + size_y * size_z)
    }
    #[inline]
    pub fn centroid(&self, axis: Axis) -> f32 {
        self.get_interval(axis).max - self.get_interval(axis).min
    }
    #[inline]
    pub const fn new(interval_x: Interval, interval_y: Interval, interval_z: Interval) -> Self {
        Self {
            x_interval: interval_x,
            y_interval: interval_y,
            z_interval: interval_z,
        }
    }
    #[inline]
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
    #[inline]
    pub fn from_points(a: Vec3, b: Vec3) -> Self {
        let x_interval = Interval::new(f32::min(a.x, b.x), f32::max(a.x, b.x));
        let y_interval = Interval::new(f32::min(a.y, b.y), f32::max(a.y, b.y));
        let z_interval = Interval::new(f32::min(a.z, b.z), f32::max(a.z, b.z));

        Self {
            x_interval,
            y_interval,
            z_interval,
        }
    }
    #[inline]
    pub fn extent(&self) -> Vec3 {
        Vec3::new(
            self.x_interval.size(),
            self.y_interval.size(),
            self.z_interval.size(),
        )
    }
    #[inline]
    pub fn get_interval(&self, axis: Axis) -> &Interval {
        match axis {
            Axis::X => &self.x_interval,
            Axis::Y => &self.y_interval,
            Axis::Z => &self.z_interval,
        }
    }
    #[inline]
    pub fn intersects(&self, ray: &Ray, mut ray_t: Interval) -> f32 {
        for axis in Axis::iter() {
            let axis_interval = self.get_interval(*axis);
            let axis_dir_inverse = ray.inv_dir.get_axis(*axis);

            let t0 = (axis_interval.min - ray.origin.get_axis(*axis)) * axis_dir_inverse;
            let t1 = (axis_interval.max - ray.origin.get_axis(*axis)) * axis_dir_inverse;

            if t0 < t1 {
                ray_t.min = t0.max(ray_t.min);

                ray_t.max = t1.min(ray_t.max);
            } else {
                ray_t.min = t1.max(ray_t.min);

                ray_t.max = t0.min(ray_t.max);
            }
            if ray_t.max <= ray_t.min {
                return INFINITY;
            }
        }
        return ray_t.min;
    }
}

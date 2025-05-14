use core::f32;
use std::f32::{INFINITY, NEG_INFINITY};

use crate::{ray_tracing::axis::Axis, ray_tracing::interval::Interval, ray_tracing::ray::Ray};
use glam::Vec3A;

use super::axis::AxisOps;
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3A,
    pub max: Vec3A,
}

impl Default for AABB {
    fn default() -> Self {
        AABB {
            min: Vec3A::ZERO,
            max: Vec3A::ZERO,
        }
    }
}

impl AABB {
    pub const EMPTY: AABB = AABB::new(
        Vec3A::new(INFINITY, INFINITY, INFINITY),
        Vec3A::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY),
    );
    pub const UNIVERSE: AABB = AABB::new(
        Vec3A::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY),
        Vec3A::new(INFINITY, INFINITY, INFINITY),
    );

    #[inline]
    pub const fn new(min: Vec3A, max: Vec3A) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn longest_axis(&self) -> Axis {
        let extents = self.extent();
        if extents.x > extents.y && extents.x > extents.z {
            Axis::X
        } else if extents.y > extents.x && extents.y > extents.z {
            Axis::Y
        } else {
            Axis::Z
        }
    }

    #[inline]
    pub fn area(&self) -> f32 {
        let e = self.extent();
        2.0 * (e.x * e.y + e.x * e.z + e.y * e.z)
    }

    #[inline]
    pub fn centroid(&self, axis: Axis) -> f32 {
        (self.get_interval(axis).min + self.get_interval(axis).max) / 2.0
    }

    #[inline]
    pub fn from_aabb(a: &AABB, b: &AABB) -> Self {
        AABB {
            min: Vec3A::new(
                a.min.x.min(b.min.x),
                a.min.y.min(b.min.y),
                a.min.z.min(b.min.z),
            ),
            max: Vec3A::new(
                a.max.x.max(b.max.x),
                a.max.y.max(b.max.y),
                a.max.z.max(b.max.z),
            ),
        }
    }

    #[inline]
    pub fn from_points(a: Vec3A, b: Vec3A) -> Self {
        AABB {
            min: Vec3A::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Vec3A::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    #[inline]
    pub fn extent(&self) -> Vec3A {
        self.max - self.min
    }

    #[inline]
    pub fn get_interval(&self, axis: Axis) -> Interval {
        match axis {
            Axis::X => Interval::new(self.min.x, self.max.x),
            Axis::Y => Interval::new(self.min.y, self.max.y),
            Axis::Z => Interval::new(self.min.z, self.max.z),
        }
    }

    pub fn intersects(&self, ray: &Ray) -> bool {
        let mut t_min = -INFINITY;
        let mut t_max = INFINITY;
        for &axis in Axis::iter() {
            let box_axis_min = self.get_interval(axis).min;
            let box_axis_max = self.get_interval(axis).max;
            let ray_axis_origin = ray.origin.get_axis(axis);
            let ray_axis_dir_inverse = (1.0 / ray.get_direction()).get_axis(axis);

            let t0 = (box_axis_min - ray_axis_origin) * ray_axis_dir_inverse;
            let t1 = (box_axis_max - ray_axis_origin) * ray_axis_dir_inverse;

            if t0 < t1 {
                t_max = t_max.min(t1);
                t_min = t_min.max(t0);
            } else {
                t_max = t_max.min(t0);
                t_min = t_min.max(t1);
            }
            if t_max <= t_min {
                dbg!("t_min: {}", t_min);
                return false;
            }
        }

        true
    }

    #[inline]
    pub fn intersects_new(&self, ray: &Ray) -> (f32, f32) {
        let box_min = self.min;
        let box_max = self.max;
        let ray_origin = ray.origin;
        let t_bot = (box_min - ray_origin) * ray.get_inverse_direction();
        let t_top = (box_max - ray_origin) * ray.get_inverse_direction();

        let mins = t_bot.min(t_top);
        let maxs = t_bot.max(t_top);

        let mut t0 = mins.max_element();
        let t1 = maxs.min_element();

        if !t0.is_finite() {
            dbg!("s");
            t0 = t1;
        }
        (t0, t1)
    }
}

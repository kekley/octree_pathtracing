use core::f32;
use std::{
    arch::x86_64::{
        _mm_cmple_ps, _mm_cvtss_f32, _mm_max_ps, _mm_min_ps, _mm_movemask_ps, _mm_mul_ps,
        _mm_set1_ps, _mm_sub_ps,
    },
    f32::{INFINITY, NEG_INFINITY},
};

use crate::{axis::Axis, get_axis, interval::Interval, ray::Ray};
use glam::Vec3A as Vec3;
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for AABB {
    fn default() -> Self {
        AABB {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        }
    }
}

impl AABB {
    pub const EMPTY: AABB = AABB::new(
        Vec3::new(INFINITY, INFINITY, INFINITY),
        Vec3::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY),
    );
    pub const UNIVERSE: AABB = AABB::new(
        Vec3::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY),
        Vec3::new(INFINITY, INFINITY, INFINITY),
    );

    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
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
            min: Vec3::new(
                a.min.x.min(b.min.x),
                a.min.y.min(b.min.y),
                a.min.z.min(b.min.z),
            ),
            max: Vec3::new(
                a.max.x.max(b.max.x),
                a.max.y.max(b.max.y),
                a.max.z.max(b.max.z),
            ),
        }
    }

    #[inline]
    pub fn from_points(a: Vec3, b: Vec3) -> Self {
        AABB {
            min: Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    #[inline]
    pub fn extent(&self) -> Vec3 {
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

    #[inline]
    pub fn intersects(&self, ray: &Ray, mut ray_t: Interval) -> f32 {
        for axis in Axis::iter() {
            let axis_interval = self.get_interval(*axis);
            let axis_dir_inverse = get_axis(&ray.inv_dir, *axis);

            let t0 = (axis_interval.min - get_axis(&ray.origin, *axis)) * axis_dir_inverse;
            let t1 = (axis_interval.max - get_axis(&ray.origin, *axis)) * axis_dir_inverse;

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
        ray_t.min
    }
}

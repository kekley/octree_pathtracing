use core::f32;
use std::{
    arch::x86_64::{
        _mm_cmple_ps, _mm_cvtss_f32, _mm_max_ps, _mm_min_ps, _mm_movemask_ps, _mm_mul_ps,
        _mm_set1_ps, _mm_sub_ps,
    },
    f32::{INFINITY, NEG_INFINITY},
};

use crate::{
    axis::{Axis, AxisOps},
    interval::Interval,
    ray::Ray,
};
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

    pub fn intersect(&self, ray: &mut Ray) -> bool {
        let ix = ray.origin.x - (ray.origin.x + ray.direction.x * Ray::OFFSET).floor();
        let iy = ray.origin.y - (ray.origin.y + ray.direction.y * Ray::OFFSET).floor();
        let iz = ray.origin.z - (ray.origin.z + ray.direction.z * Ray::OFFSET).floor();
        let mut t;
        let mut u;
        let mut v;
        let mut hit = false;

        ray.hit.t_next = ray.hit.t;

        t = (self.min.x - ix) / ray.direction.x;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = iz + ray.direction.z * t;
            v = iy + ray.direction.y * t;
            if u >= self.min.z && u <= self.max.z && v >= self.min.y && v <= self.max.y {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(-1.0, 0.0, 0.0);
            }
        }

        t = (self.max.x - ix) / ray.direction.x;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = iz + ray.direction.z * t;
            v = iy + ray.direction.y * t;
            if u >= self.min.z && u <= self.max.z && v >= self.min.y && v <= self.max.y {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = 1.0 - u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(1.0, 0.0, 0.0);
            }
        }

        t = (self.min.y - iy) / ray.direction.y;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iz + ray.direction.z * t;
            if u >= self.min.x && u <= self.max.x && v >= self.min.z && v <= self.max.z {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, -1.0, 0.0);
            }
        }

        t = (self.max.y - iy) / ray.direction.y;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iz + ray.direction.z * t;
            if u >= self.min.x && u <= self.max.x && v >= self.min.z && v <= self.max.z {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 1.0, 0.0);
            }
        }

        t = (self.min.z - iz) / ray.direction.z;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iy + ray.direction.y * t;
            if u >= self.min.x && u <= self.max.x && v >= self.min.y && v <= self.max.y {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = 1.0 - u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 0.0, -1.0);
            }
        }

        t = (self.max.z - iz) / ray.direction.z;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iy + ray.direction.y * t;
            if u >= self.min.x && u <= self.max.x && v >= self.min.y && v <= self.max.y {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 0.0, 1.0);
            }
        }

        hit
    }

    #[inline]
    pub fn intersects(&self, ray: &Ray) -> f32 {
        let mut t_min = NEG_INFINITY;
        let mut t_max = INFINITY;
        for &axis in Axis::iter() {
            let box_axis_interval = self.get_interval(axis);
            let ray_dir_axis_inverse = (1.0 / ray.direction).get_axis(axis);

            let (t0, t1) = if ray_dir_axis_inverse >= 0.0 {
                (
                    (box_axis_interval.min - ray.origin.get_axis(axis)) * ray_dir_axis_inverse,
                    (box_axis_interval.max - ray.origin.get_axis(axis)) * ray_dir_axis_inverse,
                )
            } else {
                (
                    (box_axis_interval.max - ray.origin.get_axis(axis)) * ray_dir_axis_inverse,
                    (box_axis_interval.min - ray.origin.get_axis(axis)) * ray_dir_axis_inverse,
                )
            };

            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        }
        t_min
    }
}

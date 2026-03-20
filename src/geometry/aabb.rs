use glam::{Vec2, Vec3A, Vec3Swizzles as _};

use crate::{
    geometry::axis::Axis,
    mix_vec2,
    path_tracing::{Intersect, hit_record::HitRecord, ray::Ray},
    step_vec3,
};

use super::interval::Interval;

pub const LEFT: Vec3A = Vec3A::new(-1.0, 0.0, 0.0);
pub const RIGHT: Vec3A = Vec3A::new(1.0, 0.0, 0.0);
pub const UP: Vec3A = Vec3A::new(0.0, 1.0, 0.0);
pub const DOWN: Vec3A = Vec3A::new(0.0, -1.0, 0.0);
pub const FORWARD: Vec3A = Vec3A::new(0.0, 0.0, 1.0);
pub const BACK: Vec3A = Vec3A::new(0.0, 0.0, -1.0);

#[derive(Debug, Clone)]
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
        Vec3A::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        Vec3A::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    );
    pub const UNIVERSE: AABB = AABB::new(
        Vec3A::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        Vec3A::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
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
}

impl Intersect for AABB {
    fn intersect(&self, ray: Ray) -> HitRecord {
        aabb_ray_interesct(
            self.min,
            self.max,
            ray.origin,
            *ray.get_direction(),
            *ray.get_inverse_direction(),
        )
    }
}

fn aabb_ray_interesct(
    aabb_min: Vec3A,
    aabb_max: Vec3A,
    ray_origin: Vec3A,
    ray_direction: Vec3A,
    inv_direction: Vec3A,
) -> HitRecord {
    let t0 = (aabb_min - ray_origin) * inv_direction;

    let t1 = (aabb_max - ray_origin) * inv_direction;

    let t_min_vec = t0.min(t1);
    let t_max_vec = t0.max(t1);

    let t_enter = t_min_vec.max_element();
    let t_exit = t_max_vec.min_element();

    if !(t_enter <= t_exit && t_exit > 0.0) {
        //println!("enter: {t_enter}, exit: {t_exit}");
        return HitRecord::MISS;
    }

    let normal = -ray_direction.signum()
        * step_vec3(t_min_vec.yzx(), t_min_vec.xyz())
        * step_vec3(t_min_vec.zxy(), t_min_vec.xyz());

    let mut face_id = normal.abs().dot(Vec3A::new(1.0, 2.0, 4.0)) as u32;
    face_id ^= normal.cmpgt(Vec3A::ZERO).any() as u32;

    let uv3 = ray_origin + t_enter * ray_direction;

    let mut uv2 = 0.5 + mix_vec2(uv3.xy(), uv3.zz(), normal.xy().abs());

    uv2 = mix_vec2(
        uv2,
        Vec2::new(1.0 - uv2.x, uv2.y),
        Vec2::splat(normal.x.max(normal.y.max(-normal.z))),
    );

    HitRecord::new(t_enter, t_exit, uv2, normal)
}

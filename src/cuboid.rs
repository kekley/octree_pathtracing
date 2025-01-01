use std::f32::INFINITY;

use crate::{aabb::AABB, ray::Ray};
use glam::Vec3A as Vec3;

pub enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}
#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    resource_idx: u32,
}
pub const EPSILON: f32 = 0.00000000001;

impl Cuboid {
    pub fn get_bbox(&self) -> AABB {
        self.bbox.clone()
    }
    pub fn new(bbox: AABB, material_idx: u32) -> Self {
        Self {
            bbox,
            resource_idx: material_idx,
        }
    }
    pub fn new_multi_texture(bbox: AABB, materials_idx: u32) -> Self {
        Self {
            bbox,
            resource_idx: materials_idx,
        }
    }

    pub fn hit(&self, ray: &mut Ray) -> bool {
        ray.hit.t = INFINITY;
        let t = self.bbox.intersects(ray);
        if t == INFINITY {
            return false;
        } else {
            true
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

        t = (self.bbox.min.x - ix) / ray.direction.x;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = iz + ray.direction.z * t;
            v = iy + ray.direction.y * t;
            if u >= self.bbox.min.z
                && u <= self.bbox.max.z
                && v >= self.bbox.min.y
                && v <= self.bbox.max.y
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(-1.0, 0.0, 0.0);
            }
        }

        t = (self.bbox.max.x - ix) / ray.direction.x;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = iz + ray.direction.z * t;
            v = iy + ray.direction.y * t;
            if u >= self.bbox.min.z
                && u <= self.bbox.max.z
                && v >= self.bbox.min.y
                && v <= self.bbox.max.y
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = 1.0 - u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(1.0, 0.0, 0.0);
            }
        }

        t = (self.bbox.min.y - iy) / ray.direction.y;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iz + ray.direction.z * t;
            if u >= self.bbox.min.x
                && u <= self.bbox.max.x
                && v >= self.bbox.min.z
                && v <= self.bbox.max.z
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, -1.0, 0.0);
            }
        }

        t = (self.bbox.max.y - iy) / ray.direction.y;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iz + ray.direction.z * t;
            if u >= self.bbox.min.x
                && u <= self.bbox.max.x
                && v >= self.bbox.min.z
                && v <= self.bbox.max.z
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 1.0, 0.0);
            }
        }

        t = (self.bbox.min.z - iz) / ray.direction.z;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iy + ray.direction.y * t;
            if u >= self.bbox.min.x
                && u <= self.bbox.max.x
                && v >= self.bbox.min.y
                && v <= self.bbox.max.y
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = 1.0 - u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 0.0, -1.0);
            }
        }

        t = (self.bbox.max.z - iz) / ray.direction.z;
        if t < ray.hit.t_next && t > -Ray::EPSILON {
            u = ix + ray.direction.x * t;
            v = iy + ray.direction.y * t;
            if u >= self.bbox.min.x
                && u <= self.bbox.max.x
                && v >= self.bbox.min.y
                && v <= self.bbox.max.y
            {
                hit = true;
                ray.hit.t_next = t;
                ray.hit.u = u;
                ray.hit.v = v;
                ray.hit.outward_normal = Vec3::new(0.0, 0.0, 1.0);
            }
        }

        hit
    }
}

use std::f32::INFINITY;

use crate::{aabb::AABB, ray::Ray, scene, Scene, Texture};
use glam::Vec3A as Vec3;

pub enum Face {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    pub textures: [u16; 6],
}
pub const EPSILON: f32 = 0.00000000001;

impl Cuboid {
    pub fn get_bbox(&self) -> AABB {
        self.bbox.clone()
    }
    pub fn new(bbox: AABB, material_idx: u32) -> Self {
        Self {
            bbox,
            textures: [material_idx as u16; 6],
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

    pub fn intersect(&self, ray: &mut Ray, scene: &Scene) -> bool {
        let mut hit = false;
        ray.hit.t = INFINITY;
        if self.bbox.intersect(ray) {
            if ray.hit.outward_normal.y > 0.0 || true {
                ray.hit.v = 1.0 - ray.hit.v;
                hit = Self::intersect_texture(ray, scene, self.textures[0]);
            }
        }
        hit
    }

    pub fn intersect_texture(ray: &mut Ray, scene: &Scene, material_idx: u16) -> bool {
        let color = scene.materials[material_idx as usize].albedo.value(
            ray.hit.u,
            ray.hit.v,
            &ray.at(ray.hit.t),
        );
        if color.w > Ray::EPSILON {
            ray.hit.color = color;
            println!("Color: {:?}", color);
            true
        } else {
            false
        }
    }
}

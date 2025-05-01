use core::f32;
use std::f32::INFINITY;

use crate::{ray_tracing::aabb::AABB, ray_tracing::ray::Ray};

use anyhow::Ok;
use glam::Vec3A;

use super::{material::Material, texture::Texture};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Face {
    West = 0,
    East = 1,
    Bottom = 2,
    Top = 3,
    South = 4,
    North = 5,
}

impl Face {
    pub fn to_normal(face: Face) -> Vec3A {
        match face {
            Face::West => Vec3A::NEG_X,
            Face::East => Vec3A::X,
            Face::Bottom => Vec3A::NEG_Y,
            Face::Top => Vec3A::Y,
            Face::North => Vec3A::NEG_Z,
            Face::South => Vec3A::Z,
        }
    }
}

impl TryFrom<u32> for Face {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Face::West),
            1 => Ok(Face::East),
            2 => Ok(Face::Bottom),
            3 => Ok(Face::Top),
            4 => Ok(Face::North),
            5 => Ok(Face::South),
            _ => Err(anyhow::Error::msg(value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    pub textures: [u16; 6],
}
pub const MY_EPSILON: f32 = 0.00000005;
pub const OFFSET: f32 = 0.000001;

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
        if !t {
            return false;
        } else {
            true
        }
    }

    pub fn intersect_texture(ray: &mut Ray, texture: &Texture) -> bool {
        //dbg!(material);
        let color = texture.value(ray.hit.u.abs(), ray.hit.v.abs(), &ray.at(ray.hit.t));
        //println!("u:{} ,v: {}", ray.hit.u, ray.hit.v);
        if color.w > Ray::EPSILON {
            ray.hit.color = color;
            /*             ray.hit.color = Vec4::new(
                ray.hit.outward_normal.x,
                ray.hit.outward_normal.y,
                ray.hit.outward_normal.z,
                1.0,
            ); */
            true
        } else {
            ray.hit.color = color;
            false
        }
    }

    pub fn intersect_texture_not_transparent(ray: &mut Ray, texture: &Texture) -> bool {
        //dbg!(material);
        let color = texture.value(ray.hit.u.abs(), ray.hit.v.abs(), &ray.at(ray.hit.t));
        //println!("u:{} ,v: {}", ray.hit.u, ray.hit.v);
        ray.hit.color = color;
        ray.hit.color.w = 1.0;
        true
    }
}

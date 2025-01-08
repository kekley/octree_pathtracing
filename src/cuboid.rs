use std::f32::INFINITY;

use crate::{
    aabb::AABB,
    axis::{Axis, AxisOps},
    ray::Ray,
    scene, Scene, Texture,
};

use glam::{BVec3, Vec3A as Vec3, Vec4};

pub enum Face {
    Top = 0,
    Bottom = 1,
    North = 2,
    South = 3,
    East = 4,
    West = 5,
}

#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    pub textures: [u16; 6],
}
pub const EPSILON: f32 = 0.00000005;
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

    pub fn intersect(&self, ray: &mut Ray, scene: &Scene) -> bool {
        let mut hit = false;
        ray.hit.t = INFINITY;
        let pot_t = self.get_bbox().intersects_new(ray);
        if pot_t != INFINITY {
            let point = ray.at(pot_t);
            let c = (self.bbox.min + self.bbox.max) * 0.5;
            let p = point - c;
            let d = (self.bbox.min - self.bbox.max) * 0.5;

            let bias = 1.000001;

            let normal = Vec3::new(
                (p.x / d.x.abs() * bias).trunc(),
                (p.y / d.y.abs() * bias).trunc(),
                (p.z / d.z.abs() * bias).trunc(),
            )
            .normalize();

            let size = self.bbox.max.abs() - self.bbox.min.abs();
            let inv_size = Vec3::new(1.0 / size.x, 1.0 / size.y, 1.0 / size.z);
            let uvw = (point - self.bbox.min) * inv_size;
            let mat;
            let normal_sign = normal.signum();
            if normal == Vec3::X {
                ray.hit.u = 1.0 - uvw.z;
                ray.hit.v = uvw.y;
                mat = self.textures[Face::East as usize];
            } else if normal == Vec3::Y {
                ray.hit.u = uvw.x;
                ray.hit.v = uvw.z;
                mat = self.textures[Face::Top as usize];
            } else if normal == Vec3::Z {
                ray.hit.u = uvw.x;
                ray.hit.v = uvw.y;
                mat = self.textures[Face::North as usize];
            } else if normal == Vec3::NEG_X {
                ray.hit.u = uvw.z;
                ray.hit.v = uvw.y;
                mat = self.textures[Face::West as usize];
            } else if normal == Vec3::NEG_Y {
                ray.hit.u = uvw.x;
                ray.hit.v = 1.0 - uvw.z;
                mat = self.textures[Face::Bottom as usize];
            } else if normal == Vec3::NEG_Z {
                ray.hit.u = uvw.x;
                ray.hit.v = uvw.y;
                mat = self.textures[Face::South as usize];
            } else {
                println!("pot_t: {:?}", pot_t);
                println!("point: {:?}", point);
                println!("normal: {:?}", normal);
                panic!("bad normal");
            }
            ray.hit.outward_normal = normal;

            hit = Cuboid::intersect_texture(ray, scene, mat);
            if hit {
                ray.hit.outward_normal = normal;

                ray.hit.t = ray.hit.t_next;
                ray.distance_travelled += ray.hit.t;
                ray.origin = ray.at(ray.hit.t);
            }
        }
        hit
    }

    pub fn intersect_texture(ray: &mut Ray, scene: &Scene, material_idx: u16) -> bool {
        let color = scene.materials[material_idx as usize].albedo.value(
            ray.hit.u.abs(),
            ray.hit.v.abs(),
            &ray.at(ray.hit.t),
        );
        //println!("u:{} ,v: {}", ray.hit.u, ray.hit.v);
        if color.w > Ray::EPSILON {
            assert!(color.w == 1.0);
            ray.hit.color = color;
            /*             ray.hit.color = Vec4::new(
                ray.hit.outward_normal.x,
                ray.hit.outward_normal.y,
                ray.hit.outward_normal.z,
                1.0,
            ); */
            true
        } else {
            println!("something went wrong");
            false
        }
    }
}

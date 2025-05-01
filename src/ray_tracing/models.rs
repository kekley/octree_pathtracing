use std::{f32::INFINITY, num::NonZeroU32, usize};

use glam::{Vec2, Vec3A, Vec4};

use crate::voxels::octree_traversal::OctreeIntersectResult;

use super::{
    aabb::AABB,
    cuboid::{Cuboid, Face},
    material::Material,
    quad::Quad,
    ray::Ray,
    resource_manager::QuadID,
    texture::Texture,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SingleBlockModel {
    pub first_quad_index: QuadID,
}

impl SingleBlockModel {
    pub fn get_id_for_face(&self, face: Face) -> QuadID {
        self.first_quad_index + face as u32
    }
}
impl SingleBlockModel {
    pub fn intersect(
        &self,
        ray: &mut Ray,
        t0: f32,
        face: Face,
        uv: &Vec2,
        quads: &[Quad],
        materials: &[Material],
        textures: &[Texture],
    ) -> bool {
        ray.hit.t = INFINITY;
        ray.hit.t_next = INFINITY;
        let normal = Face::to_normal(face);
        let quad = &quads[self.get_id_for_face(face) as usize];
        let material = &materials[quad.material_id as usize];
        let texture = &textures[material.texture as usize];
        ray.hit.previous_material = ray.hit.current_material.clone();
        ray.hit.current_material = quad.material_id;
        ray.origin = ray.at(t0);
        ray.hit.normal = normal;
        ray.hit.t = t0;
        ray.hit.u = uv.x;
        ray.hit.v = uv.y;
        Cuboid::intersect_texture(ray, texture);
        true
    }
    pub fn intersect_preview(
        &self,
        ray: &mut Ray,
        t0: f32,
        face: Face,
        uv: &Vec2,
        quads: &[Quad],
        materials: &[Material],
        textures: &[Texture],
    ) -> bool {
        ray.hit.t = INFINITY;
        ray.hit.t_next = INFINITY;
        let normal = Face::to_normal(face);
        let quad = &quads[self.get_id_for_face(face) as usize];
        let material = &materials[quad.material_id as usize];
        let texture = &textures[material.texture as usize];
        ray.hit.previous_material = ray.hit.current_material.clone();
        ray.hit.current_material = quad.material_id;
        ray.origin = ray.at(t0);
        ray.hit.normal = normal;
        ray.hit.t = t0;
        ray.hit.u = uv.x;
        ray.hit.v = uv.y;
        Cuboid::intersect_texture_not_transparent(ray, texture);
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuadModel {
    pub starting_quad_id: QuadID,
    len: NonZeroU32,
}

impl QuadModel {
    const E0: Vec3A = Vec3A::splat(-Ray::EPSILON);
    const E1: Vec3A = Vec3A::splat(1.0 + Ray::EPSILON);
    pub fn new(starting_quad_id: u32, len: u32) -> Self {
        assert!(len > 0);
        Self {
            starting_quad_id,
            len: NonZeroU32::new(len).unwrap(),
        }
    }
    pub fn intersect(
        &self,
        ray: &mut Ray,
        voxel_position: &Vec3A,
        t_enter: f32,
        quads: &[Quad],
        materials: &[Material],
        textures: &[Texture],
    ) -> bool {
        let mut hit_any = false;
        ray.hit.t = INFINITY;
        ray.hit.t_next = INFINITY;
        let mut color = Vec4::ZERO;
        let mut closest: Option<&Quad> = None;
        ray.origin = ray.at(t_enter);
        let quads = &quads
            [(self.starting_quad_id as usize)..(self.starting_quad_id + self.len.get()) as usize];
        quads.iter().for_each(|quad| {
            if quad.hit(ray, voxel_position) {
                let c = textures[materials[quad.material_id as usize].texture as usize].value(
                    ray.hit.u,
                    ray.hit.v,
                    &ray.at(ray.hit.t_next),
                );

                if c.w > Ray::EPSILON {
                    closest = Some(quad);
                    color = c;
                    ray.hit.t = ray.hit.t_next;
                    hit_any = true
                }
            }
        });

        if hit_any {
            /*             let p =
                ray.origin - (ray.at(Ray::OFFSET)).floor() + *ray.get_direction() * ray.hit.t_next;
            let gt = p.cmpgt(Self::E1);
            let lt = p.cmplt(Self::E0);

            if gt.any() || lt.any() {
                return false;
            } */
            ray.hit.color = color;
            ray.set_normal(closest.unwrap().normal);
            ray.distance_travelled += ray.hit.t;
            ray.origin = ray.at(ray.hit.t);
            ray.hit.previous_material = ray.hit.current_material.clone();
            ray.hit.current_material = closest.as_ref().unwrap().material_id.clone();
        }
        //dbg!(ray.hit.t);

        hit_any
    }
}

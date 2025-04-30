use std::{f32::INFINITY, usize};

use glam::{Vec2, Vec3A, Vec4};

use crate::voxels::octree_traversal::OctreeIntersectResult;

use super::{
    aabb::AABB,
    cuboid::{Cuboid, Face},
    material::Material,
    quad::Quad,
    ray::Ray,
};

#[derive(Debug, Clone)]
pub struct SingleBlockModel {
    pub materials: [Material; 6],
}
impl SingleBlockModel {
    pub fn intersect(
        &self,
        ray: &mut Ray,
        voxel_position: &Vec3A,
        t0: f32,
        face: Face,
        uv: &Vec2,
    ) -> bool {
        let bounds = AABB::from_points(*voxel_position, voxel_position + 1.0);
        let (t0, t1) = bounds.intersects_new(ray);
        let normal = Face::to_normal(face);

        let material = &self.materials[face as usize];

        ray.hit.previous_material = ray.hit.current_material.clone();
        ray.hit.current_material = self.materials[face as usize].clone();
        ray.origin = ray.at(t0);
        ray.hit.normal = normal;
        ray.hit.t = t0;
        ray.hit.u = uv.x;
        ray.hit.v = uv.y;

        Cuboid::intersect_texture(ray, material);
        true
    }
}

#[derive(Debug, Clone)]
pub struct QuadModel {
    pub quads: Vec<Quad>,
}
impl QuadModel {
    const E0: Vec3A = Vec3A::splat(-Ray::EPSILON);
    const E1: Vec3A = Vec3A::splat(1.0 + Ray::EPSILON);
    pub fn intersect(&self, ray: &mut Ray, voxel_position: &Vec3A) -> bool {
        let mut hit_any = false;
        ray.hit.t = INFINITY;
        let mut color = Vec4::ZERO;
        let mut closest: Option<&Quad> = None;
        self.quads.iter().for_each(|quad| {
            if quad.hit(ray, voxel_position) {
                let c = quad
                    .material
                    .albedo
                    .value(ray.hit.u, ray.hit.v, &ray.at(ray.hit.t_next));

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
            ray.orient_normal(closest.unwrap().normal);
            ray.distance_travelled += ray.hit.t;
            ray.origin = ray.at(ray.hit.t);
            ray.hit.previous_material = ray.hit.current_material.clone();
            ray.hit.current_material = closest.as_ref().unwrap().material.clone();
        }
        //dbg!(ray.hit.t);

        hit_any
    }
}

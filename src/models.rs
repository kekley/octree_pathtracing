use std::{array::from_fn, f32::INFINITY, usize};

use glam::{Vec3, Vec3A, Vec4};
use rayon::array;

use crate::{
    material, octree_traversal::OctreeIntersectResult, Cuboid, Face, Material, MaterialBuilder,
    Quad, RTWImage, Ray, Texture, AABB,
};

#[derive(Debug, Clone)]
pub struct SingleBlockModel {
    pub materials: [Material; 6],
}
impl SingleBlockModel {
    pub fn intersect(&self, octree_result: &OctreeIntersectResult<u32>, ray: &mut Ray) -> bool {
        let bounds = AABB::from_points(
            octree_result.voxel_position,
            octree_result.voxel_position + 1.0,
        );
        let (t0, t1) = bounds.intersects_new(ray);
        if !t0.is_finite() {
            println!("ray: {:?}", ray);
            println!("box: {:?}", bounds);
            println!("{:?}", (t0, t1));
            panic!();
        }
        let normal = Face::to_normal(octree_result.face);
        let material = &self.materials[octree_result.face as usize];
        ray.hit.previous_material = ray.hit.current_material.clone();
        ray.hit.current_material = self.materials[octree_result.face as usize].clone();
        ray.origin = ray.at(t0);
        ray.hit.normal = normal;
        ray.hit.t = t0;
        ray.hit.u = octree_result.uv.x;
        ray.hit.v = octree_result.uv.y;
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
    pub fn intersect(&self, ray: &mut Ray) -> bool {
        let mut hit = false;

        ray.hit.t = INFINITY;
        let mut color = Vec4::ZERO;
        self.quads.iter().for_each(|quad| {
            if quad.hit(ray) {
                let c = quad
                    .material
                    .albedo
                    .value(ray.hit.u, ray.hit.v, &Vec3A::ZERO);
                if c.w > Ray::EPSILON {
                    color = c;
                    ray.hit.t = ray.hit.t_next;
                    ray.orient_normal(quad.normal);
                    hit = true
                }
            }
        });

        if hit {
            let p = ray.origin - (ray.at(Ray::OFFSET)).floor() + ray.direction * ray.hit.t_next;
            let gt = p.cmpgt(Self::E1);
            let lt = p.cmplt(Self::E0);

            if gt.any() || lt.any() {
                return false;
            }

            ray.hit.color = color;
            ray.distance_travelled += ray.hit.t;
            ray.origin = ray.at(ray.hit.t);
        }
        hit
    }
}

use super::{material::Material, ray::Ray};
use crate::ray_tracing::cuboid::Face;
use crate::voxels::octree_traversal::OctreeIntersectResult;
use glam::{Mat3A, Vec3A, Vec4};
use std::cmp::PartialEq;

#[derive(Debug, Clone)]
pub struct Quad {
    pub origin: Vec3A,
    v: Vec3A,
    u: Vec3A,
    w: Vec3A,
    pub normal: Vec3A,
    pub material: Material,
    pub tint: Vec4,
}

impl Quad {
    //uv Minimum and maximum U/V texture coordinates (x0,y0 bottom left)
    pub fn new(origin: Vec3A, u: Vec3A, v: Vec3A, material: Material) -> Self {
        let n = u.cross(v);
        let normal = n.normalize();
        let w = n / n.dot(n);
        Quad {
            origin: origin,
            v: v,
            u: u,
            w: w,
            normal: normal,
            material: material,
            tint: Vec4::ONE,
        }
    }
    pub fn transform(&mut self, matrix: &Mat3A) {
        todo!();
    }
    /*    pub fn hit(&self, ray: &mut Ray, octree_intersect_result: &OctreeIntersectResult<u32>) -> bool {
        // ISSUE WHERE ray.at(Ray::OFFSET).floor() DOESN'T EQUAL VOXEL POS
        let test =
            if octree_intersect_result.face == Face::Top && self.material.name.contains("top") {
                true
            } else {
                false
            };
        let (u, v): (f32, f32);
        let mut i = ray.origin - ray.at(Ray::OFFSET).floor();
        let denominator = ray.get_direction().dot(self.normal);
        if denominator < -Ray::EPSILON || (denominator > Ray::EPSILON && true) {
            let t = -((i * self.normal).element_sum() + self.d) / denominator;
            if test {
                dbg!(i);
                dbg!(ray.at(Ray::OFFSET).floor());
                dbg!(octree_intersect_result);
            }
            if t > -Ray::EPSILON && t < ray.hit.t {
                //plane interesction confirmed
                i = i + ray.get_direction() * t - self.origin;
                u = i.dot(self.xv) * self.xvl;
                v = i.dot(self.yv) * self.yvl;
                if u >= 0.0 && u <= 1.0 && v >= 0.0 && v <= 1.0 {
                    ray.hit.u = self.uv.x + u * self.uv.y;
                    ray.hit.v = self.uv.z + v * self.uv.w;
                    ray.hit.t_next = t;

                    return true;
                }
            }
        }
        return false;
    } */
    pub fn hit(&self, ray: &mut Ray, octree_intersect_result: &OctreeIntersectResult<u32>) -> bool {
        let ray_origin_translated = ray.origin - octree_intersect_result.voxel_position;
        let denom = self.normal.dot(*ray.get_direction());

        // ray parallel to plane
        if f32::abs(denom) < 1e-8 {
            panic!();
            return false;
        }

        let d = self.normal.dot(self.origin);

        let t = (d - self.normal.dot(ray_origin_translated)) / denom;

        let intersection = ray_origin_translated + ray.get_direction() * t;
        let planar_hit_point = intersection - self.origin;
        let alpha = self.w.dot(planar_hit_point.cross(self.v));
        let beta = self.w.dot(self.u.cross(planar_hit_point));

        if alpha < 0.0 || alpha > 1.0 || beta < 0.0 || beta > 1.0 && ray.hit.t_next > t {
            return false;
        }

        ray.hit.t_next = t;
        ray.hit.normal = self.normal;
        ray.hit.u = alpha;
        ray.hit.v = beta;

        true
    }
}

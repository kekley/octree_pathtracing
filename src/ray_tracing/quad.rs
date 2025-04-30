use super::{material::Material, ray::Ray};
use crate::ray_tracing::cuboid::Face;
use crate::voxels::octree_traversal::OctreeIntersectResult;
use glam::{Affine3A, Mat3A, Mat4, Vec2, Vec3A, Vec4};
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
    texture_u_range: Vec2,
    texture_v_range: Vec2,
}

impl Quad {
    pub fn new(
        origin: Vec3A,
        u: Vec3A,
        v: Vec3A,
        texture_u_range: Vec2,
        texture_v_range: Vec2,
        material: Material,
    ) -> Self {
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
            texture_u_range,
            texture_v_range,
        }
    }
    pub fn transform_about_pivot(&mut self, matrix: &Affine3A, pivot: Vec3A) {
        self.origin -= pivot;
        self.origin = matrix.transform_point3a(self.origin);
        self.origin += pivot;
        self.u = matrix.transform_vector3a(self.u);
        self.v = matrix.transform_vector3a(self.v);
        let n = self.u.cross(self.v);

        self.normal = matrix.transform_vector3a(self.normal);
        self.w = n / n.dot(n);
    }

    pub fn transform(&mut self, matrix: &Affine3A) {
        dbg!("transform");
        self.origin = matrix.transform_point3a(self.origin);
        self.u = matrix.transform_vector3a(self.u);
        self.v = matrix.transform_vector3a(self.v);
        let n = self.u.cross(self.v);

        self.normal = n.normalize();
        self.w = n / n.dot(n);
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
    pub fn hit(&self, ray: &mut Ray, voxel_position: &Vec3A) -> bool {
        let translated_quad_origin = self.origin + voxel_position;
        let denom = self.normal.dot(*ray.get_direction());
        let d = self.normal.dot(translated_quad_origin);
        // ray parallel to plane
        if f32::abs(denom) < 1e-8 {
            return false;
        }

        let t = (d - self.normal.dot(ray.origin)) / denom;
        if t < 0.0 || !t.is_finite() || t > ray.hit.t_next {
            return false;
        }
        let intersection = ray.origin + ray.get_direction() * t;
        let planar_hit_point = intersection - translated_quad_origin;
        let alpha = self.w.dot(planar_hit_point.cross(self.v));
        let beta = self.w.dot(self.u.cross(planar_hit_point));

        if alpha < 0.0 || alpha > 1.0 || beta < 0.0 || beta > 1.0 {
            return false;
        }

        ray.hit.t_next = t;
        ray.hit.normal = self.normal;
        ray.hit.u =
            self.texture_u_range.x + alpha * (self.texture_u_range.y - self.texture_u_range.x);
        ray.hit.v =
            self.texture_v_range.x + beta * (self.texture_v_range.y - self.texture_v_range.x);

        true
    }
}

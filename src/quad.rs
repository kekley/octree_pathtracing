use glam::{Vec3, Vec3A, Vec4};

use crate::{Material, Ray};

pub struct Quad {
    pub origin: Vec3A,
    xv: Vec3A,
    yv: Vec3A,
    uv: Vec4,
    d: f32,
    xvl: f32,
    yvl: f32,
    pub normal: Vec3A,
    pub material: Material,
    pub tint: Vec4,
}

impl Quad {
    pub fn hit(&self, ray: &mut Ray) -> bool {
        let (u, v): (f32, f32);

        let mut i = ray.origin - ray.at(Ray::OFFSET).floor();

        let denominator = self.normal.dot(ray.direction);

        if denominator < -Ray::EPSILON || (denominator > Ray::EPSILON) {
            let t = -(i.dot(self.normal) + self.d) / denominator;

            if t > -Ray::EPSILON && t < ray.hit.t {
                i = i + ray.direction * t - self.origin;
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
    }
}

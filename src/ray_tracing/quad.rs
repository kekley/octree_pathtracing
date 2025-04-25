use glam::{Mat3A, Vec3A, Vec4};

use super::{material::Material, ray::Ray};

#[derive(Debug, Clone)]
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
    //uv Minimum and maximum U/V texture coordinates (x0,y0 bottom left)
    pub fn new(v0: Vec3A, v1: Vec3A, v2: Vec3A, uv: Vec4, material: Material) -> Self {
        let origin = v0;
        let xv = v1 - v0;
        let xvl = 1.0 / xv.length_squared();
        let yv = v2 - v0;
        let yvl = 1.0 / yv.length_squared();
        let n = xv.cross(yv).normalize();
        let d = -n.dot(origin);
        let mut uv = uv;
        uv.y -= uv.x;
        uv.w -= uv.z;
        Self {
            origin,
            xv,
            yv,
            uv,
            d,
            xvl,
            yvl,
            normal: n,
            material,
            tint: Vec4::ONE,
        }
    }
    pub fn transform(&mut self, matrix: &Mat3A) {
        self.origin -= 0.5;
        self.origin = *matrix * self.origin;
        self.origin += 0.5;
        self.xv = *matrix * self.xv;
        self.yv = *matrix * self.yv;
        self.normal = *matrix * self.normal;
        self.d = -self.normal.dot(self.origin);
    }
    pub fn hit(&self, ray: &mut Ray) -> bool {
        let (u, v): (f32, f32);

        let mut i = ray.origin - ray.at(Ray::OFFSET).floor();

        let denominator = self.normal.dot(*ray.get_direction());

        if denominator < -Ray::EPSILON || (denominator > Ray::EPSILON) {
            let t = -(i.dot(self.normal) + self.d) / denominator;

            if t > -Ray::EPSILON && t < ray.hit.t {
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
    }
}

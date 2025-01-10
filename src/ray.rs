use std::f32::{consts::PI, INFINITY};

use crate::HitRecord;
use glam::{Mat3A, Vec3A as Vec3, Vec4};
use rand::{rngs::StdRng, Rng};

#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub(crate) origin: Vec3,
    pub(crate) direction: Vec3,
    pub(crate) inv_dir: Vec3,
    pub(crate) distance_travelled: f32,
    pub(crate) hit: HitRecord<u16>,
}

impl Ray {
    pub const EPSILON: f32 = 0.00000005;
    pub const OFFSET: f32 = 0.000001;

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
    #[inline]
    pub fn new(point: Vec3, direction: Vec3) -> Self {
        Self {
            origin: point,
            direction,
            hit: HitRecord::default(),
            distance_travelled: 0.0,
            inv_dir: 1.0 / direction,
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            direction: self.direction,
            distance_travelled: self.distance_travelled,
            hit: self.hit.clone(),
            inv_dir: self.inv_dir,
        }
    }

    pub fn set_normals(&mut self, normal: Vec3) {
        self.hit.outward_normal = normal;
        //self.hit.geom_normal = normal;
    }

    pub fn orient_normal(&mut self, normal: Vec3) {
        if self.direction.dot(normal) > 0.0 {
            self.hit.outward_normal = -normal;
        } else {
            self.hit.outward_normal = normal;
        }
        //self.hit.geom_normal = normal;
    }

    pub fn specular_reflection(&self, roughness: f32, rng: &mut StdRng) -> Self {
        let mut tmp = Ray {
            origin: self.origin,
            direction: self.direction,
            distance_travelled: 0.0,
            hit: HitRecord {
                t: INFINITY,
                t_next: INFINITY,
                u: 0.0,
                v: 0.0,
                current_material: self.hit.current_material,
                outward_normal: self.hit.outward_normal,
                previous_material: self.hit.previous_material,
                color: Vec4::ZERO,
                depth: self.hit.depth,
                specular: self.hit.specular,
            },
            inv_dir: self.inv_dir,
        };
        tmp.hit.current_material = tmp.hit.previous_material;

        if roughness > Ray::EPSILON {
            let mut specular_dir = self.direction;
            let s = -2.0 * self.direction.dot(self.hit.outward_normal);
            let d = self.hit.outward_normal;
            let o = self.direction;

            specular_dir = s * d + o;

            let x1 = rng.gen::<f32>();
            let x2 = rng.gen::<f32>();
            let r = x1.sqrt();
            let theta = 2.0 * PI * x2;

            let tx = r * theta.cos();
            let ty = r * theta.sin();
            let tz = (1.0 - x1).sqrt();

            let tangent: Vec3;
            if tmp.hit.outward_normal.x.abs() > 0.1 {
                tangent = Vec3::new(0.0, 1.0, 0.0);
            } else {
                tangent = Vec3::new(1.0, 0.0, 0.0);
            }

            let u = tangent.cross(tmp.hit.outward_normal).normalize();
            let v = tmp.hit.outward_normal.cross(u);

            let rotation_matrix = Mat3A::from_cols(u, v, tmp.hit.outward_normal);

            let new_dir = rotation_matrix * Vec3::new(tx, ty, tz);

            tmp.direction = new_dir * roughness + specular_dir * (1.0 - roughness);
            tmp.direction = tmp.direction.normalize();
            tmp.inv_dir = 1.0 / tmp.direction;
            tmp.origin = tmp.at(Ray::OFFSET);
        } else {
            tmp.direction = self.direction
                - 2.0 * self.direction.dot(self.hit.outward_normal) * self.hit.outward_normal;
            tmp.inv_dir = 1.0 / tmp.direction;
            tmp.origin = tmp.at(Ray::OFFSET);
        }

        if tmp.hit.outward_normal.dot(tmp.direction).signum()
            == tmp.hit.outward_normal.dot(self.direction).signum()
        {
            let factor = tmp.hit.outward_normal.dot(self.direction) * -Ray::EPSILON
                - tmp.direction.dot(tmp.hit.outward_normal);
            tmp.direction += factor * tmp.hit.outward_normal;
            tmp.direction = tmp.direction.normalize();
            tmp.inv_dir = 1.0 / tmp.direction;
        }

        tmp
    }

    pub fn scatter_normal(&mut self, rng: &mut StdRng) {
        let x1 = rng.gen::<f32>();
        let x2 = rng.gen::<f32>();

        let r = x1.sqrt();
        let theta = 2.0 * PI * x2;

        let tangent = if self.hit.outward_normal.x.abs() > 0.1 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        let u = tangent.cross(self.hit.outward_normal).normalize();
        let v = self.hit.outward_normal.cross(u);

        let rotation_matrix = Mat3A::from_cols(u, v, self.hit.outward_normal);

        let new_dir =
            rotation_matrix * Vec3::new(r * theta.cos(), r * theta.sin(), (1.0 - x1).sqrt());

        self.direction = new_dir;
        self.inv_dir = 1.0 / self.direction;
        self.origin = self.at(Ray::OFFSET);
    }

    pub fn diffuse_reflection(&self, rng: &mut StdRng) -> Self {
        let mut tmp = self.clone();

        let x1 = rng.gen::<f32>();
        let x2 = rng.gen::<f32>();

        let r = x1.sqrt();
        let theta = 2.0 * PI * x2;

        let tx = r * theta.cos();
        let ty = r * theta.sin();
        let tz = (1.0 - tx * tx - ty * ty).sqrt();

        let tangent = if self.hit.outward_normal.x.abs() > 0.1 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        let u = tangent.cross(self.hit.outward_normal).normalize();
        let v = self.hit.outward_normal.cross(u);

        let rotation_matrix = Mat3A::from_cols(u, v, self.hit.outward_normal);
        let new_dir = rotation_matrix * Vec3::new(tx, ty, tz);

        tmp.direction = new_dir.normalize();
        tmp.inv_dir = 1.0 / tmp.direction;

        tmp.origin = tmp.at(Ray::OFFSET);
        //println!("new_dir: {:?}", new_dir);

        tmp.hit.current_material = tmp.hit.previous_material;
        tmp.hit.specular = false;

        if tmp.hit.outward_normal.dot(tmp.direction).signum()
            == tmp.hit.outward_normal.dot(self.direction).signum()
        {
            let factor = tmp.hit.outward_normal.dot(self.direction).signum() * -Ray::EPSILON
                - tmp.direction.dot(tmp.hit.outward_normal);
            tmp.direction += factor * tmp.hit.outward_normal;
            tmp.direction = tmp.direction.normalize();
            tmp.inv_dir = 1.0 / tmp.direction
        }
        //tmp.origin = self.at(self.hit.t);
        //tmp.origin = tmp.at(Ray::OFFSET);
        tmp
    }
}

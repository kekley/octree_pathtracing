use std::f32::{consts::PI, INFINITY};

use crate::HitRecord;
use glam::{Mat3A as Mat3, Vec3A as Vec3};
use rand::{rngs::StdRng, Rng};

#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_dir: Vec3,
    pub distance_travelled: f32,
    pub hit: HitRecord,
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
            inv_dir: 1.0 / direction,
            hit: HitRecord::default(),
            distance_travelled: 0.0,
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            direction: self.direction,
            inv_dir: self.inv_dir,
            distance_travelled: self.distance_travelled,
            hit: self.hit.clone(),
        }
    }

    pub fn specular_reflection(&self, roughness: f32, rng: &mut StdRng) -> Self {
        let mut tmp = self.clone();
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

            let xx: Vec3;
            if tmp.hit.outward_normal.x.abs() > 0.1 {
                xx = Vec3::new(0.0, 1.0, 0.0);
            } else {
                xx = Vec3::new(1.0, 0.0, 0.0);
            }

            let u = xx.cross(tmp.hit.outward_normal).normalize();
            let v = tmp.hit.outward_normal.cross(u);

            let rotation_matrix = Mat3::from_cols(u, v, tmp.hit.outward_normal);

            let new_dir = rotation_matrix * Vec3::new(tx, ty, tz);

            tmp.direction = new_dir * roughness + specular_dir * (1.0 - roughness);
            tmp.direction = tmp.direction.normalize();
            tmp.origin = tmp.at(Ray::EPSILON);
        } else {
            tmp.direction = self.direction
                - 2.0 * self.direction.dot(self.hit.outward_normal) * self.hit.outward_normal;
            tmp.origin = tmp.at(Ray::EPSILON);
        }

        if tmp.hit.geom_normal.dot(tmp.direction).signum()
            == tmp.hit.geom_normal.dot(self.direction).signum()
        {
            let factor = tmp.hit.geom_normal.dot(self.direction) * -Ray::EPSILON
                - tmp.direction.dot(tmp.hit.geom_normal);
            tmp.direction += factor * tmp.hit.geom_normal;
            tmp.direction = tmp.direction.normalize();
        }

        tmp
    }

    pub fn scatter_normal() {}
}

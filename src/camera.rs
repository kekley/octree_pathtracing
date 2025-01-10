use glam::Vec3A as Vec3;

use rand::{rngs::StdRng, Rng};
use rand_distr::UnitDisc;
use std::f32::{consts::FRAC_PI_6, INFINITY};

use crate::{axis::UP, ray::Ray, HitRecord};

#[derive(Debug, Clone)]
pub struct Camera {
    pub fov: f32,
    pub up: Vec3,
    pub aperture: f32,
    pub focus_dist: f32,
    direction: Vec3,
    position: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: FRAC_PI_6,
            up: UP,
            aperture: 0.0,
            focus_dist: 0.0,
            direction: Vec3::NEG_Z,
            position: Vec3::ZERO,
        }
    }
}

impl Camera {
    pub fn look_at(look_from: Vec3, look_at: Vec3, up: Vec3, fov_degrees: f32) -> Self {
        let direction = (look_at - look_from).normalize();
        let right = up.cross(direction).normalize();
        let up = direction.cross(right).normalize();
        let fov = fov_degrees.to_radians();
        Self {
            fov,
            up,
            aperture: 0.0,
            focus_dist: 0.0,
            direction,
            position: look_from,
        }
    }

    pub fn focus(mut self, focal_point: Vec3, aperture: f32) -> Self {
        self.focus_dist = (focal_point - self.position).dot(self.direction);
        self.aperture = aperture;
        self
    }

    // x and y normalized to [-1,1]
    pub fn get_ray(&self, rng: &mut StdRng, x: f32, y: f32) -> Ray {
        let distance_to_image_plane = (self.fov / 2.0).tan().recip();

        let right = self.direction.cross(self.up).normalize();

        let mut origin = self.position;
        let mut new_dir =
            (distance_to_image_plane * self.direction + x * right + y * self.up).normalize();
        //println!("new_dir: {}", new_dir);

        if self.aperture > 0.0 {
            let focal_point = origin + new_dir * self.focus_dist;
            let [x, y]: [f32; 2] = rng.sample(UnitDisc);
            origin += (x * right + y * self.up) * self.aperture;
            new_dir = (focal_point - origin);
        }
        //println!("new_dir: {:}", new_dir);

        Ray {
            origin,
            direction: new_dir,
            hit: HitRecord::default(),
            distance_travelled: 0.0,
            inv_dir: 1.0 / new_dir,
        }
    }
}

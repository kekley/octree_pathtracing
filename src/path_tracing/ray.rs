use std::f32::consts::PI;

use rand::rngs::ThreadRng;

use glam::Vec3A;

use crate::{geometry::axis::Axis, path_tracing::vector_extensions::VectorExtensions as _};

#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3A,
    direction: Vec3A,
    inv_dir: Vec3A,
}

impl Ray {
    pub const EPSILON: f32 = 1e-6;
    pub const OFFSET: f32 = 1e-8;

    pub fn at(&self, t: f32) -> Vec3A {
        self.origin + self.direction * t
    }
    pub fn new(origin: Vec3A, direction: Vec3A) -> Self {
        let mut inv_dir = Vec3A::splat(1.0 / Self::EPSILON) * direction.signum();
        let smaller_than_epsilon = direction.abs().cmplt(Vec3A::splat(Self::EPSILON));
        // Prevent generation of NANs in inv_dir
        for axis in Axis::iter() {
            let axis = axis.into();
            if !smaller_than_epsilon.test(axis) {
                inv_dir[axis] = 1.0 / direction[axis];
            }
        }

        Self {
            origin,
            direction,
            inv_dir,
        }
    }

    pub fn get_direction(&self) -> &Vec3A {
        &self.direction
    }
    pub fn get_inverse_direction(&self) -> &Vec3A {
        &self.inv_dir
    }
    pub fn set_direction(&mut self, direction: Vec3A) {
        let mut inv_dir = Vec3A::splat(1.0 / Self::EPSILON) * direction.signum();
        let smaller_than_epsilon = direction.abs().cmplt(Vec3A::splat(Self::EPSILON));
        for axis in Axis::iter() {
            let axis_index = axis.into();
            if !smaller_than_epsilon.test(axis_index) {
                inv_dir[axis_index] = 1.0 / direction[axis_index];
            }
        }

        self.direction = direction;
        self.inv_dir = inv_dir;
    }
    pub fn specular_reflection(&self, rng: &mut ThreadRng) -> Self {
        todo!();
    }

    pub fn scatter_normal(&mut self, rng: &mut ThreadRng) {
        todo!();
    }

    pub fn diffuse_reflection(&mut self, ray: &mut Ray, rng: &mut ThreadRng) {
        todo!();
    }
}

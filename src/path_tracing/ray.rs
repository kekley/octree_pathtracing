use std::f32::consts::PI;

use rand::rngs::ThreadRng;

use glam::Vec3A;


#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3A,
    direction: Vec3A,
    inv_dir: Vec3A,
}

impl Ray {
    pub const EPSILON: f32 = 0.00000005;
    pub const OFFSET: f32 = 0.000001;

    pub fn at(&self, t: f32) -> Vec3A {
        self.origin + self.direction * t
    }
    pub fn new(origin: Vec3A, direction: Vec3A) -> Self {
        const EPSILON: f32 = 1e-6;
        let b_vec = direction.abs().cmplt(Vec3A::splat(EPSILON));

        if direction.abs()
        let mut inv_dir = Vec3A::splat(1.0 / EPSILON);
        // Prevent generation of NANs in inv_dir
        (0..3).for_each(|i: usize| {
            if !b_vec.test(i) {
                inv_dir[i] = 1.0 / direction[i];
            }
        });

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
        const EPSILON: f32 = 1e-6;
        let inv_dir = Vec3A::new(
            if direction.x.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.x
            },
            if direction.y.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.y
            },
            if direction.z.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.z
            },
        );
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

use fastrand::Rng;

use crate::{BVHTree, Camera, Cuboid, Ray, Sphere};

pub struct Scene {
    spheres: Vec<Sphere>,
    cubes: Vec<Cuboid>,
    bvhs: Vec<BVHTree>,
    camera: Camera,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            spheres: Vec::new(),
            cubes: Vec::new(),
            bvhs: Vec::new(),
            camera: Camera::new(),
        }
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
    }

    pub fn add_cube(&mut self, cube: Cuboid) {
        self.cubes.push(cube);
    }

    pub fn add_bvh(&mut self, bvh: BVHTree) {
        self.bvhs.push(bvh);
    }
}

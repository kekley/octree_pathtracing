use crate::{BVHTree, Camera, Cuboid, Ray, Sphere};

use glam::Vec3A as Vec3;
pub struct Scene {
    spheres: Vec<Sphere>,
    cubes: Vec<Cuboid>,
    bvhs: Vec<BVHTree>,
    camera: Camera,
}

impl Scene {
    const SKY_COLOR: Vec3 = Vec3::new(0.5, 0.7, 1.0);

    pub fn new() -> Self {
        Self {
            spheres: Vec::new(),
            cubes: Vec::new(),
            bvhs: Vec::new(),
            camera: Camera::default(),
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

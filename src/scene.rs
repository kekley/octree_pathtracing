use crate::{BVHTree, Camera, Cuboid, Material, Ray, Sphere};

use glam::Vec3A as Vec3;
pub struct Scene {
    spheres: Vec<Sphere>,
    cubes: Vec<Cuboid>,
    bvhs: Vec<BVHTree>,
    pub materials: Vec<Material>,
    pub spp: u32,
    pub branch_count: u32,
    pub camera: Camera,
}

impl Scene {
    pub const SKY_COLOR: Vec3 = Vec3::new(0.5, 0.7, 1.0);

    pub fn new() -> Self {
        Self {
            spheres: Vec::new(),
            cubes: Vec::new(),
            bvhs: Vec::new(),
            camera: Camera::default(),
            materials: Vec::new(),
            spp: todo!(),
            branch_count: todo!(),
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

    pub fn hit(&self, ray: &mut Ray) -> bool {
        let mut hit = false;

        for sphere in &self.spheres {
            hit |= sphere.hit(ray);
        }

        true
    }

    pub fn get_current_branch_count(&self) -> u32 {
        if self.spp < self.branch_count {
            if self.spp as f32 <= (self.branch_count as f32).sqrt() {
                return 1;
            } else {
                return self.branch_count - self.spp;
            }
        } else {
            return self.branch_count;
        }
    }
}

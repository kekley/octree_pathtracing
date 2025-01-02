use crate::{path_trace, BVHTree, Camera, Cuboid, Material, Ray, Sphere};

use bitflags::bitflags;
use glam::{Vec3A as Vec3, Vec4};
use rand::rngs::StdRng;
pub struct Scene {
    spheres: Vec<Sphere>,
    cubes: Vec<Cuboid>,
    bvhs: Vec<BVHTree>,
    pub materials: Vec<Material>,
    pub spp: u32,
    pub branch_count: u32,
    pub camera: Camera,
}

pub struct SceneBuilder {
    pub spp: Option<u32>,
    pub branch_count: Option<u32>,
    pub camera: Option<Camera>,
}

impl SceneBuilder {
    pub fn build(self) -> Scene {
        Scene {
            spheres: Vec::new(),
            cubes: Vec::new(),
            bvhs: Vec::new(),
            materials: Vec::new(),
            spp: self.spp.unwrap_or(1),
            branch_count: self.branch_count.unwrap_or(1),
            camera: self.camera.unwrap_or(Camera::default()),
        }
    }

    pub fn spp(self, spp: u32) -> Self {
        Self {
            spp: Some(spp),
            ..self
        }
    }

    pub fn branch_count(self, branch_count: u32) -> Self {
        Self {
            branch_count: Some(branch_count),
            ..self
        }
    }

    pub fn camera(self, camera: Camera) -> Self {
        Self {
            camera: Some(camera),
            ..self
        }
    }
}

impl Scene {
    pub fn new() -> SceneBuilder {
        SceneBuilder {
            spp: None,
            branch_count: None,
            camera: None,
        }
    }
    pub const SKY_COLOR: Vec3 = Vec3::new(0.5, 0.7, 1.0);

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

        for cube in &self.cubes {
            if cube.intersect(ray, &self) {
                hit = true;
            }
        }
        if hit {
            ray.distance_travelled = ray.hit.t;
            ray.origin = ray.at(ray.hit.t);
            return true;
        }
        return false;
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

    pub fn trace_ray(&self, x: f32, y: f32, rng: &mut StdRng) -> Vec4 {
        let mut ray = self.camera.get_ray(rng, x, y);
        path_trace(&self, &mut ray, true);
        ray.hit.color
    }
}

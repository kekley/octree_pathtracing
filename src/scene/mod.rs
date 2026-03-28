pub mod resource_manager;
pub mod scene_config;

use rand::rngs::ThreadRng;

use glam::{Vec3, Vec4};

use crate::{
    geometry::{aabb::UP, quad::Quad},
    octree::world::WorldOctree,
    path_tracing::{
        path_tracer::{path_trace, preview_render},
        ray::Ray,
    },
    scene::{resource_manager::MaterialID, scene_config::SceneConfig},
    textures::material::Material,
};

pub struct Scene {
    config: SceneConfig,
    pub octree: WorldOctree,
    pub quads: Box<[Quad]>,
    pub materials: Box<[Material]>,
}

impl Scene {
    pub fn get_material(&self, material_id: MaterialID) -> &Material {
        &self.materials[material_id as usize]
    }
}

pub struct SceneBuilder {
    pub spp: Option<u32>,
    pub branch_count: Option<u32>,
}

impl Scene {
    pub const SKY_COLOR: Vec4 = Vec4::new(0.5, 0.7, 1.0, 1.0);

    pub fn hit(&self, ray: &mut Ray) -> bool {
        let hit = false;
        let direction = ray.get_direction();
        if direction.x == 0.0 && direction.y == 0.0 && direction.z == 0.0 || direction.is_nan() {
            println!("invalid ray direction");
            println!("ray dir: {}", direction);
            ray.set_direction(UP);
        }

        let max_dst = 1024.0;

        //TODO
        //self.octree
        // .intersect_octree_path_tracer(ray, max_dst, &self.materials, &self.quads);
        false
    }
    pub fn hit_preview(&self, ray: &mut Ray) -> bool {
        let hit = false;
        let direction = ray.get_direction();
        if direction.x == 0.0 && direction.y == 0.0 && direction.z == 0.0 || direction.is_nan() {
            println!("invalid ray direction");
            println!("ray dir: {}", direction);
            ray.set_direction(UP);
        }

        let max_dst = 1024.0;

        //self.octree
        //  .intersect_octree_preview(ray, max_dst, &self.materials, &self.quads);
        false
    }

    pub fn get_preview_color(&self, mut ray: Ray, x: f32, y: f32, rng: &mut ThreadRng) -> Vec3 {
        let mut attenuation = Vec4::ZERO;
        preview_render(rng, self, &mut ray, &mut attenuation);
        todo!();
    }
    pub fn get_color(&self, mut ray: Ray, rng: &mut ThreadRng, current_spp: u32) -> Vec3 {
        let mut attenuation = Vec4::ZERO;
        path_trace(rng, self, &mut ray, true, &mut attenuation, current_spp);
        todo!();
        //Vec3::new(ray.hit.normal.x, ray.hit.normal.y, ray.hit.normal.z)
    }

    pub fn get_sky_color(&self, ray: &mut Ray, draw_sun: bool) {
        self.get_sky_color_diffuse_inner(ray);
        //TODO: RAY COLOR TIMES SKY EXPOSURE AND SKY LIGHT MODIFIER
        if draw_sun {
            self.add_sun_color(ray);
        }
    }

    pub fn get_sky_color_diffuse_sun(&self, ray: &mut Ray, diffuse_sun: bool) {
        self.get_sky_color_diffuse_inner(ray);
        //TODO: RAY COLOR TIMES SKY EXPOSURE AND SKY LIGHT MODIFIER

        if diffuse_sun {
            self.add_sun_color_diffuse_sun(ray);
        }
    }

    pub fn get_sky_color_inner(&self, ray: &mut Ray) {
        todo!();
    }
    pub fn get_sky_color_interp(&self, ray: &mut Ray) {
        self.get_sky_color_diffuse_inner(ray);
        // ray color times sky exposure and skylightmodifier
        self.add_sun_color(ray);
        todo!();
    }
    pub fn add_sun_color(&self, ray: &mut Ray) {
        todo!();
    }

    pub fn add_sun_color_diffuse_sun(&self, ray: &mut Ray) {
        todo!();
    }
    pub fn get_sky_color_diffuse_inner(&self, ray: &mut Ray) {
        todo!();
    }
}

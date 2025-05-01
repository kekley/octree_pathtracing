use std::{f32::consts::PI, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitterSamplingStrategy {
    None {
        name: &'static str,
        description: &'static str,
    },
    One {
        name: &'static str,
        description: &'static str,
    },
    OneBlock {
        name: &'static str,
        description: &'static str,
    },
    All {
        name: &'static str,
        description: &'static str,
    },
}

impl Default for EmitterSamplingStrategy {
    fn default() -> Self {
        Self::NONE
    }
}

impl EmitterSamplingStrategy {
    pub fn get_description(&self) -> &str {
        match self {
            EmitterSamplingStrategy::None { description, .. } => description,
            EmitterSamplingStrategy::One { description, .. } => description,
            EmitterSamplingStrategy::OneBlock { description, .. } => description,
            EmitterSamplingStrategy::All { description, .. } => description,
        }
    }
    pub const NONE: EmitterSamplingStrategy = EmitterSamplingStrategy::None {
        name: "None",
        description: "No emitter sampling.",
    };

    pub const ONE: EmitterSamplingStrategy = EmitterSamplingStrategy::One {
        name: "One",
        description: "Sample a single face.",
    };

    pub const ONE_BLOCK: EmitterSamplingStrategy = EmitterSamplingStrategy::OneBlock {
        name: "One Block",
        description: "Sample all the faces on a single emitter block.",
    };

    pub const ALL: EmitterSamplingStrategy = EmitterSamplingStrategy::All {
        name: "All",
        description: "Sample all faces on all emitter blocks.",
    };
}

#[derive(Debug, Clone)]
pub struct SunSamplingStrategy {
    name: &'static str,
    description: &'static str,
    pub sun_sampling: bool,
    pub diffuse_sun: bool,
    pub strict_direct_light: bool,
    pub sun_luminosity: bool,
    pub importance_sampling: bool,
}

impl Default for SunSamplingStrategy {
    fn default() -> Self {
        Self::IMPORTANCE
    }
}

impl SunSamplingStrategy {
    pub const OFF: SunSamplingStrategy = SunSamplingStrategy {
        name: "Off",
        description: "Sun is not sampled with next event estimation.",
        sun_sampling: false,
        diffuse_sun: true,
        strict_direct_light: false,
        sun_luminosity: true,
        importance_sampling: false,
    };

    pub const NON_LUMINOUS: SunSamplingStrategy = SunSamplingStrategy {
        name: "Non-Luminous",
        description:
            "Sun is drawn on the skybox but it does not contribute to the lighting of the scene.",
        sun_sampling: false,
        diffuse_sun: false,
        strict_direct_light: false,
        sun_luminosity: false,
        importance_sampling: false,
    };

    pub const FAST: SunSamplingStrategy = SunSamplingStrategy {
    name: "Fast",
    description: "Fast sun sampling algorithm. Lower noise but does not correctly model some visual effects.",
    sun_sampling: true,
    diffuse_sun: false,
    strict_direct_light: false,
    sun_luminosity: false,
    importance_sampling: false,
};

    pub const IMPORTANCE: SunSamplingStrategy = SunSamplingStrategy {
    name: "Importance",
    description: "Sun is sampled on a certain percentage of diffuse reflections. Correctly models visual effects while reducing noise for direct and diffuse illumination.",
    sun_sampling: false,
    diffuse_sun: true,
    strict_direct_light: false,
    sun_luminosity: true,
    importance_sampling: true,
};

    pub const HIGH_QUALITY: SunSamplingStrategy = SunSamplingStrategy {
    name: "High Quality",
    description: "High quality sun sampling. More noise but correctly models visual effects such as caustics.",
    sun_sampling: true,
    diffuse_sun: true,
    strict_direct_light: true,
    sun_luminosity: true,
    importance_sampling: false,
};
}

use rand::rngs::StdRng;

use glam::{UVec3, Vec3, Vec3A, Vec3Swizzles, Vec4, Vec4Swizzles};
use spider_eye::{block, loaded_world::WorldCoords, MCResourceLoader};

use crate::{
    random_float,
    ray_tracing::axis::UP,
    voxels::octree::{self, Octree},
};

use super::{
    camera::Camera,
    material::Material,
    path_tracer::{path_trace, preview_render},
    quad::Quad,
    ray::Ray,
    resource_manager::{MaterialID, ModelManager, ResourceModel},
    texture::Texture,
    tile_renderer::U8Color,
};

pub struct Scene {
    pub sun: Sun,
    pub sun_sampling_strategy: SunSamplingStrategy,
    pub emitters_enabled: bool,
    pub emmitter_intensity: f32,
    pub emitter_sampling_strategy: EmitterSamplingStrategy,
    pub f_sub_surface: f32,
    pub octree: Octree<ResourceModel>,
    pub quads: Box<[Quad]>,
    pub textures: Box<[Texture]>,
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

impl ModelManager {
    pub fn build(&self, octree: Octree<ResourceModel>) -> Scene {
        let mut write_lock = self.quads.write();
        let quad_vec: &mut Vec<Quad> = write_lock.as_mut();
        let quads: Vec<Quad> = std::mem::take(quad_vec);
        let quad_box: Box<[Quad]> = Box::from(quads);

        let mut write_lock = self.materials.write();
        let materials_vec_ref: &mut Vec<Material> = write_lock.as_mut();
        let materials_vec: Vec<Material> = std::mem::take(materials_vec_ref);
        let materials_box: Box<[Material]> = Box::from(materials_vec);

        let mut write_lock = self.textures.write();
        let textures_ref: &mut Vec<Texture> = write_lock.as_mut();
        let textures_vec: Vec<Texture> = std::mem::take(textures_ref);
        let textures_box: Box<[Texture]> = Box::from(textures_vec);

        Scene {
            sun: Sun::new(
                PI / 2.5,
                PI / 3.0,
                0.03,
                Vec4::splat(1.0),
                Texture::Color(U8Color::new(25, 25, 25, 25)),
                true,
                false,
                Vec3A::splat(1.0),
            ),
            sun_sampling_strategy: SunSamplingStrategy::IMPORTANCE,
            emitter_sampling_strategy: EmitterSamplingStrategy::NONE,
            emitters_enabled: false,
            emmitter_intensity: 13.0,
            f_sub_surface: 0.3,
            quads: quad_box,
            textures: textures_box,
            materials: materials_box,
            octree: octree,
        }
    }
}

impl Scene {
    pub const SKY_COLOR: Vec4 = Vec4::new(0.5, 0.7, 1.0, 1.0);

    pub fn hit(&self, ray: &mut Ray) -> bool {
        let mut hit = false;
        let direction = ray.get_direction();
        if direction.x == 0.0 && direction.y == 0.0 && direction.z == 0.0 || direction.is_nan() {
            println!("invalid ray direction");
            println!("ray dir: {}", direction);
            ray.set_direction(UP);
        }

        let max_dst = 1024.0;

        let intersection = self.octree.intersect_octree_path_tracer(
            ray,
            max_dst,
            &self.materials,
            &self.textures,
            &self.quads,
        );
        intersection
    }
    pub fn hit_preview(&self, ray: &mut Ray) -> bool {
        let mut hit = false;
        let direction = ray.get_direction();
        if direction.x == 0.0 && direction.y == 0.0 && direction.z == 0.0 || direction.is_nan() {
            println!("invalid ray direction");
            println!("ray dir: {}", direction);
            ray.set_direction(UP);
        }

        let max_dst = 1024.0;

        let intersection = self.octree.intersect_octree_preview(
            ray,
            max_dst,
            &self.materials,
            &self.textures,
            &self.quads,
        );
        intersection
    }

    pub fn get_current_branch_count(scene_branch_count: u32, current_spp: u32) -> u32 {
        if current_spp < scene_branch_count {
            if current_spp <= (scene_branch_count as f32).sqrt() as u32 {
                return 1;
            } else {
                return scene_branch_count - current_spp;
            }
        } else {
            return scene_branch_count;
        }
    }
    pub fn get_preview_color(&self, mut ray: Ray, x: f32, y: f32, rng: &mut StdRng) -> Vec3 {
        let mut attenuation = Vec4::ZERO;
        preview_render(rng, &self, &mut ray, &mut attenuation);
        ray.hit.color.xyz().into()
    }
    pub fn get_color(&self, mut ray: Ray, rng: &mut StdRng, current_spp: u32) -> Vec3 {
        let mut attenuation = Vec4::ZERO;
        path_trace(rng, &self, &mut ray, true, &mut attenuation, current_spp);
        //Vec3::new(ray.hit.normal.x, ray.hit.normal.y, ray.hit.normal.z)
        ray.hit.color.xyz()
    }

    pub fn get_sky_color(&self, ray: &mut Ray, draw_sun: bool) {
        self.get_sky_color_diffuse_inner(ray);
        //TODO: RAY COLOR TIMES SKY EXPOSURE AND SKY LIGHT MODIFIER
        if draw_sun {
            self.add_sun_color(ray);
        }
        ray.hit.color.w = 1.0;
    }

    pub fn get_sky_color_diffuse_sun(&self, ray: &mut Ray, diffuse_sun: bool) {
        self.get_sky_color_diffuse_inner(ray);
        //TODO: RAY COLOR TIMES SKY EXPOSURE AND SKY LIGHT MODIFIER

        if diffuse_sun {
            self.add_sun_color_diffuse_sun(ray);
        }
        ray.hit.color.w = 1.0
    }

    pub fn get_sky_color_inner(&self, ray: &mut Ray) {
        ray.hit.color = Scene::SKY_COLOR;
    }
    pub fn get_sky_color_interp(&self, ray: &mut Ray) {
        self.get_sky_color_diffuse_inner(ray);
        // ray color times sky exposure and skylightmodifier
        self.add_sun_color(ray);
        ray.hit.color.w = 1.0;
    }
    pub fn add_sun_color(&self, ray: &mut Ray) {
        let r = ray.hit.color.x;
        let g = ray.hit.color.y;
        let b = ray.hit.color.z;
        if self.sun.intersect(ray) {
            ray.hit.color.x = ray.hit.color.x + r;
            ray.hit.color.y = ray.hit.color.y + g;
            ray.hit.color.z = ray.hit.color.z + b;
        }
    }

    pub fn add_sun_color_diffuse_sun(&self, ray: &mut Ray) {
        let r = ray.hit.color.x;
        let g = ray.hit.color.y;
        let b = ray.hit.color.z;
        if self.sun.intersect_diffuse(ray) {
            let mult = self.sun.luminosity;
            ray.hit.color.x = ray.hit.color.x * mult + r;
            ray.hit.color.y = ray.hit.color.y * mult + g;
            ray.hit.color.z = ray.hit.color.z * mult + b;
        }
    }
    pub fn get_sky_color_diffuse_inner(&self, ray: &mut Ray) {
        ray.hit.color = Scene::SKY_COLOR;
    }
}

#[derive(Debug, Clone)]
pub struct Sun {
    pub luminosity: f32,
    pub luminosity_pdf: f32,
    pub importance_sample_chance: f32,
    pub importance_sample_radius: f32,
    draw_texture: bool,
    texture_modification: bool,
    apparent_brightness: f32,
    apparent_texture_brightness: Vec3A,
    texture: Texture,
    color: Vec4,
    sw: Vec3A,
    pub radius: f32,
    pub azimuth: f32,
    pub altitude: f32,
    pub su: Vec3A,
    sv: Vec3A,
    radius_cos: f32,
    radius_sin: f32,
    pub emmittance: Vec4,
}

impl Default for Sun {
    fn default() -> Self {
        Sun::new(
            PI / 2.5,
            PI / 3.0,
            0.03,
            Vec4::splat(1.0),
            Texture::Color(U8Color::new(255, 255, 255, 255)),
            true,
            false,
            Vec3A::splat(1.0),
        )
    }
}

impl Sun {
    pub const DEFAULT_AZIMUTH: f32 = PI / 2.5;
    pub const DEFAULT_ALTITUDE: f32 = PI / 3.0;
    pub const DEFAULT_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.1;
    pub const MAX_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.9;
    pub const MIN_IMPORTANCE_SAMPLE_CHANCE: f32 = 0.001;
    pub const MAX_IMPORTANCE_SAMPLE_RADIUS: f32 = 5.0;
    pub const DEFAULT_IMPORTANCE_SAMPLE_RADIUS: f32 = 1.2;
    pub const MIN_IMPORTANCE_SAMPLE_RADIUS: f32 = 0.1;
    const AMBIENT: f32 = 0.3;
    const INTENSITY: f32 = 1.25;
    const GAMMA: f32 = 2.2;
    pub fn new(
        azimuth: f32,
        altitude: f32,
        radius: f32,
        color: Vec4,
        texture: Texture,
        draw_texture: bool,
        texture_modification: bool,
        apparent_color: Vec3A,
    ) -> Self {
        let azimuth = azimuth;
        let altitude = altitude;
        let radius_cos = radius.cos();
        let radius_sin = radius.sin();

        let theta = azimuth;
        let phi = altitude;

        let r = phi.cos().abs();

        let sw = Vec3A::new(theta.cos() * r, phi.sin(), theta.sin() * r);

        let mut su = if sw.x.abs() > 0.1 {
            Vec3A::new(0.0, 1.0, 0.0)
        } else {
            Vec3A::new(1.0, 0.0, 0.0)
        };

        let mut sv = sw.cross(su);
        sv = sv.normalize();
        su = sv.cross(sw);

        let mut emittance = color;
        emittance *= Sun::INTENSITY.powf(Sun::GAMMA);
        let apparent_brightness = Sun::INTENSITY;
        let mut apparent_texture_brightness = if texture_modification {
            apparent_color
        } else {
            Vec3A::splat(1.0)
        };

        apparent_texture_brightness *= apparent_brightness.powf(Sun::GAMMA);

        let sun = Sun {
            draw_texture,
            texture,
            color,
            sw,
            radius,
            azimuth,
            altitude,
            su,
            sv,
            emmittance: emittance,
            radius_cos,
            radius_sin,
            luminosity: 100.0,
            luminosity_pdf: 1.0 / 100.0,
            importance_sample_chance: Sun::DEFAULT_IMPORTANCE_SAMPLE_CHANCE,
            importance_sample_radius: Sun::DEFAULT_IMPORTANCE_SAMPLE_RADIUS,
            texture_modification,
            apparent_texture_brightness: apparent_texture_brightness,
            apparent_brightness,
        };

        sun
    }
    pub fn intersect(&self, ray: &mut Ray) -> bool {
        let direction = ray.get_direction();
        if !self.draw_texture || direction.dot(self.sw) < 0.5 {
            return false;
        }

        let width = self.radius * 4.0;
        let width2 = width * 2.0;
        let a = PI / 2.0 - direction.dot(self.su).acos() + width;
        if a >= 0.0 && a < width2 {
            let b = PI / 2.0 - direction.dot(self.sv).acos() + width;
            if b >= 0.0 && b < width2 {
                ray.hit.color = self.texture.value(a / width2, b / width2, &Vec3A::ZERO);
                ray.hit.color.x *= self.apparent_texture_brightness.x * 10.0;
                ray.hit.color.y *= self.apparent_texture_brightness.y * 10.0;
                ray.hit.color.z *= self.apparent_texture_brightness.z * 10.0;
                return true;
            }
        }

        return false;
    }
    pub fn intersect_diffuse(&self, ray: &mut Ray) -> bool {
        let direction = ray.get_direction();
        if direction.dot(self.sw) < 0.5 {
            return false;
        }
        let width = self.radius * 4.0;
        let width2 = width * 2.0;

        let a = PI / 2.0 - direction.dot(self.su).acos() + width;
        if a >= 0.0 && a < width2 {
            let b = PI / 2.0 - direction.dot(self.sv).acos() + width;
            if b >= 0.0 && b < width2 {
                ray.hit.color = self.texture.value(a / width2, b / width2, &Vec3A::ZERO);
                ray.hit.color.x *= self.color.x * 10.0;
                ray.hit.color.y *= self.color.y * 10.0;
                ray.hit.color.z *= self.color.z * 10.0;
                return true;
            }
        }
        return false;
    }
    pub fn get_random_sun_direction(&self, reflected: &mut Ray, rng: &mut StdRng) {
        let x1 = random_float(rng);
        let x2 = random_float(rng);
        let cos_a = 1.0 - x1 + x1 * self.radius_cos;
        let sin_a = (1.0 - cos_a * cos_a).sqrt();
        let phi = 2.0 * PI * x2;

        let mut u = self.su.clone();
        let mut v = self.sv.clone();
        let mut w = self.sw.clone();

        u *= phi.cos() * sin_a;
        v *= phi.sin() * sin_a;
        w *= cos_a;

        let mut reflected_dir = u + v;
        reflected_dir += w.normalize();
        reflected.set_direction(reflected_dir);
    }

    pub fn flat_shading(&self, ray: &mut Ray) {
        let n = ray.hit.normal;
        let mut shading = n.x * self.sw.x + n.y * self.sw.y + n.z * self.sw.z;
        shading = Sun::AMBIENT.max(shading);
        ray.hit.color *= self.emmittance * shading;
    }
}

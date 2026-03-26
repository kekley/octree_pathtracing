use rand::rngs::ThreadRng;

use glam::Vec4;

use crate::{path_tracing::ray::Ray, scene::Scene, textures::material::Material};

pub fn path_trace(
    rng: &mut ThreadRng,
    scene: &Scene,
    ray: &mut Ray,
    first_reflection: bool,
    attenuation: &mut Vec4,
    branch_count: u32,
) -> bool {
    todo!();
}

pub fn preview_render(rng: &mut ThreadRng, scene: &Scene, ray: &mut Ray, attenuation: &mut Vec4) {
    todo!();
}

pub fn do_specular_reflection(
    ray: &Ray,
    cumulative_color: &mut Vec4,
    do_metal: bool,
    rng: &mut ThreadRng,
    attenuation: &mut Vec4,
    current_spp: u32,
) -> Ray {
    todo!();
}

pub fn do_diffuse_reflection(
    ray: &Ray,
    cumulative_color: &mut Vec4,
    material: &Material,
    rng: &mut ThreadRng,
    attenuation: &mut Vec4,
    branch_count: u32,
) -> Ray {
    todo!()
}

pub fn do_refraction(
    ray: &Ray,
    current_material: &Material,
    prev_material: &Material,
    rng: &mut ThreadRng,
    attenuation: &mut Vec4,
    branch_count: u32,
) -> Ray {
    todo!();
}

pub fn do_transmission(
    ray: &Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    absorption: f32,
    scene: &Scene,
    attenuation: &mut Vec4,
    rng: &mut ThreadRng,
    branch_count: u32,
) -> bool {
    let mut hit = false;
    *next = ray.clone();
    next.origin = next.at(Ray::OFFSET);

    if path_trace(rng, scene, next, false, attenuation, branch_count) {
        translucent_ray_color(scene, ray, next, cumulative_color, absorption);
        hit = true;
    }
    hit
}

pub fn translucent_ray_color(
    scene: &Scene,
    ray: &Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    absorption: f32,
) {
    todo!();
}
pub fn next_intersection(scene: &Scene, ray: &mut Ray) -> bool {
    todo!();
}

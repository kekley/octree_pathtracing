use core::f32;
use std::f32::INFINITY;

use rand::rngs::StdRng;

use glam::{Vec3A, Vec4, Vec4Swizzles};

use crate::random_float;

use super::{
    material::{self, Material, MaterialFlags},
    ray::Ray,
    scene::{EmitterSamplingStrategy, Scene},
};
pub fn path_trace(
    rng: &mut StdRng,
    scene: &Scene,
    ray: &mut Ray,
    first_reflection: bool,
    attenuation: &mut Vec4,
    current_spp: u32,
) -> bool {
    let mut hit: bool = false;

    loop {
        if !next_intersection(scene, ray) {
            if ray.hit.depth == 0 {
                //direct sky hit
                scene.get_sky_color_interp(ray);
                hit = true;
            } else if ray.hit.specular {
                scene.get_sky_color(ray, true);
                hit = true;
            } else {
                scene.get_sky_color_diffuse_sun(ray, scene.sun_sampling_strategy.diffuse_sun);
                hit = true;
            }
            break;
        }
        //println!("hit!");

        let current_material = ray.hit.current_material.clone();
        if current_material.name.contains("cobblestone") && ray.hit.depth == 0 {
            let a = 4;
        }
        let prev_material = ray.hit.previous_material.clone();

        let specular = current_material.specular;
        let diffuse = ray.hit.color.w;
        let absorb = ray.hit.color.w;

        let ior1 = current_material.index_of_refraction;
        let ior2 = prev_material.index_of_refraction;

        if ray.hit.color.w + specular < Ray::EPSILON && ior1 == ior2 {
            continue;
        }

        if ray.hit.depth + 1 >= 5 {
            break;
        }
        ray.hit.depth += 1;

        let mut cumm_color = Vec4::splat(0.0);
        let mut next = Ray::default();

        let metal = current_material.metalness;

        let count = if first_reflection {
            Scene::get_current_branch_count(scene.branch_count, current_spp)
        } else {
            1
        };

        for _ in 0..count {
            let do_metal = metal > Ray::EPSILON && random_float(rng) < metal;
            if do_metal || (specular > Ray::EPSILON && random_float(rng) < specular) {
                hit |= do_specular_reflection(
                    ray,
                    &mut next,
                    &mut cumm_color,
                    do_metal,
                    rng,
                    scene,
                    attenuation,
                    current_spp,
                );
            } else if random_float(rng) < diffuse {
                //println!("diffuse");
                hit |= do_diffuse_reflection(
                    ray,
                    &mut next,
                    &mut cumm_color,
                    &current_material,
                    rng,
                    scene,
                    attenuation,
                    current_spp,
                );
            } else if (ior1 - ior2).abs() >= Ray::EPSILON {
                hit |= do_refraction(
                    ray,
                    &mut next,
                    &current_material,
                    &prev_material,
                    &mut cumm_color,
                    ior1,
                    ior2,
                    absorb,
                    rng,
                    scene,
                    attenuation,
                    current_spp,
                );
            } else {
                hit |= do_transmission(
                    ray,
                    &mut next,
                    &mut cumm_color,
                    absorb,
                    scene,
                    attenuation,
                    rng,
                    current_spp,
                )
            }
        }

        ray.hit.color = cumm_color * (1.0 / count as f32);

        break;
    }

    if !hit {
        ray.hit.color = Vec4::new(0.0, 0.0, 0.0, 1.0);
        if first_reflection {
            let air_distance = ray.distance_travelled;
        }
    }

    hit
}

pub fn preview_render(rng: &mut StdRng, scene: &Scene, ray: &mut Ray, attenuation: &mut Vec4) {
    loop {
        if !next_intersection(scene, ray) {
            break;
        } else if !(ray.hit.current_material.name == "air") && ray.hit.color.w > 0.0 {
            break;
        } else {
            ray.origin = ray.at(Ray::OFFSET);
        }
    }

    if ray.hit.current_material.name == "air" {
        scene.get_sky_color_inner(ray);
        scene.add_sun_color(ray);
    } else {
        scene.sun.flat_shading(ray);
    }
}

pub fn do_specular_reflection(
    ray: &Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    do_metal: bool,
    rng: &mut StdRng,
    scene: &Scene,
    attenuation: &mut Vec4,
    current_spp: u32,
) -> bool {
    println!("specular");

    let mut hit = false;
    *next = ray.specular_reflection(ray.hit.current_material.roughness, rng);

    if path_trace(rng, scene, next, false, attenuation, current_spp) {
        if do_metal {
            cumulative_color.x += ray.hit.color.x * next.hit.color.x;
            cumulative_color.y += ray.hit.color.y * next.hit.color.y;
            cumulative_color.z += ray.hit.color.z * next.hit.color.z;
        } else {
            cumulative_color.x += next.hit.color.x;
            cumulative_color.y += next.hit.color.y;
            cumulative_color.z += next.hit.color.z;
        }
        hit = true;
    }

    hit
}

pub fn do_diffuse_reflection(
    ray: &mut Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    material: &material::Material,
    rng: &mut StdRng,
    scene: &Scene,
    attenuation: &mut Vec4,
    current_spp: u32,
) -> bool {
    let mut hit = false;
    let mut emmitance = Vec3A::splat(0.0);
    let indirect_emmitter_color = Vec4::splat(0.0);

    if scene.emitters_enabled
        && (scene.emitter_sampling_strategy == EmitterSamplingStrategy::NONE || ray.hit.depth == 1)
        && material.emittance > Ray::EPSILON
    {
        emmitance = Vec3A::new(
            ray.hit.color.x * ray.hit.color.x,
            ray.hit.color.y * ray.hit.color.y,
            ray.hit.color.z * ray.hit.color.z,
        );
        emmitance *= material.emittance;
        hit = true
    } else if scene.emitters_enabled
        && scene.emitter_sampling_strategy != EmitterSamplingStrategy::NONE
    {
        match scene.emitter_sampling_strategy {
            EmitterSamplingStrategy::None { name, description } => {}
            EmitterSamplingStrategy::All { name, description } => todo!(),
            EmitterSamplingStrategy::One { name, description } => todo!(),
            EmitterSamplingStrategy::OneBlock { name, description } => todo!(),
        }
    }

    if scene.sun_sampling_strategy.sun_sampling {
        dbg!("huh?");
        *next = ray.clone();
        scene.sun.get_random_sun_direction(next, rng);

        let mut direct_light_r = 0.0;
        let mut direct_light_g = 0.0;
        let mut direct_light_b = 0.0;

        let front_light = next.get_direction().dot(ray.hit.normal) > 0.0;

        if front_light
            || (material
                .material_flags
                .contains(MaterialFlags::SUBSURFACE_SCATTER)
                && random_float(rng) < scene.f_sub_surface)
        {
            if !front_light {
                next.origin += -Ray::OFFSET * ray.hit.normal;
            }

            next.hit.current_material = next.hit.previous_material.clone();

            get_direct_light_attenuation(scene, next, attenuation);

            let a = if scene.sun_sampling_strategy.sun_luminosity {
                scene.sun.luminosity_pdf
            } else {
                1.0
            };
            if attenuation.w > 0.0 {
                let mult = next.get_direction().dot(ray.hit.normal).abs() * a;
                direct_light_r = attenuation.x * attenuation.w * mult;
                direct_light_g = attenuation.y * attenuation.w * mult;
                direct_light_b = attenuation.z * attenuation.w * mult;
                hit = true;
            }
        }
        next.diffuse_reflection(ray, rng, scene);
        hit = path_trace(rng, scene, next, false, attenuation, current_spp) || hit;

        if hit {
            let sun_emittance = scene.sun.emmittance;
            cumulative_color.x += emmitance.x
                + ray.hit.color.x
                    * (direct_light_r * sun_emittance.x
                        + next.hit.color.x
                        + indirect_emmitter_color.x);
            cumulative_color.y += emmitance.y
                + ray.hit.color.y
                    * (direct_light_g * sun_emittance.y
                        + next.hit.color.y
                        + indirect_emmitter_color.y);
            cumulative_color.z += emmitance.z
                + ray.hit.color.z
                    * (direct_light_b * sun_emittance.z
                        + next.hit.color.z
                        + indirect_emmitter_color.z);
        } else if indirect_emmitter_color.x > Ray::EPSILON
            || indirect_emmitter_color.y > Ray::EPSILON
            || indirect_emmitter_color.z > Ray::EPSILON
        {
            hit = true;
            cumulative_color.x += ray.hit.color.x * indirect_emmitter_color.x;
            cumulative_color.y += ray.hit.color.y * indirect_emmitter_color.y;
            cumulative_color.z += ray.hit.color.z * indirect_emmitter_color.z;
        }
    } else {
        let ray_color = ray.hit.color.clone();
        next.diffuse_reflection(ray, rng, scene);
        hit = path_trace(rng, scene, next, false, attenuation, current_spp) || hit;

        if hit {
            cumulative_color.x +=
                emmitance.x + ray_color.x * (next.hit.color.x + indirect_emmitter_color.x);
            cumulative_color.y +=
                emmitance.y + ray_color.y * (next.hit.color.y + indirect_emmitter_color.y);
            cumulative_color.z +=
                emmitance.z + ray_color.z * (next.hit.color.z + indirect_emmitter_color.z);
        } else if indirect_emmitter_color.x > Ray::EPSILON
            || indirect_emmitter_color.y > Ray::EPSILON
            || indirect_emmitter_color.z > Ray::EPSILON
        {
            hit = true;
            cumulative_color.x += ray_color.x * indirect_emmitter_color.x;
            cumulative_color.y += ray_color.y * indirect_emmitter_color.y;
            cumulative_color.z += ray_color.z * indirect_emmitter_color.z;
        }
        ray.hit.color = ray_color;
    }
    hit
}

pub fn do_refraction(
    ray: &Ray,
    next: &mut Ray,
    current_material: &Material,
    prev_material: &Material,
    cumulative_color: &mut Vec4,
    ior1: f32,
    ior2: f32,
    absorption: f32,
    rng: &mut StdRng,
    scene: &Scene,
    attenuation: &mut Vec4,
    current_spp: u32,
) -> bool {
    print!("refraction");

    let mut hit = false;
    let do_refraction = current_material
        .material_flags
        .contains(MaterialFlags::REFRACTIVE)
        || current_material
            .material_flags
            .contains(MaterialFlags::REFRACTIVE);

    let ior1overior2 = ior1 / ior2;
    let cos_theta = -ray.get_direction().dot(ray.hit.normal);
    let radicand = 1.0 - ior1overior2.powi(2) * (1.0 - cos_theta.powi(2));

    if do_refraction && radicand < Ray::EPSILON {
        *next = ray.specular_reflection(current_material.roughness, rng);
        if path_trace(rng, scene, next, false, attenuation, current_spp) {
            hit = true;
            cumulative_color.x += next.hit.color.x;
            cumulative_color.y += next.hit.color.y;
            cumulative_color.z += next.hit.color.z;
        }
    } else {
        *next = ray.clone();

        let a = ior1overior2 - 1.0;
        let b = ior1overior2 + 1.0;

        let r0 = a * a / (b * b);
        let c: f32 = 1.0 - cos_theta;
        let rtheta = r0 + (1.0 - r0) * c.powi(5);

        if random_float(rng) < rtheta {
            *next = ray.specular_reflection(current_material.roughness, rng);
            if path_trace(rng, scene, next, false, attenuation, current_spp) {
                hit = true;
                cumulative_color.x += next.hit.color.x;
                cumulative_color.y += next.hit.color.y;
                cumulative_color.z += next.hit.color.z;
            }
        } else if do_refraction {
            let t2 = radicand.sqrt();
            let n = ray.hit.normal;
            if cos_theta > 0.0 {
                let refracted_direction =
                    ior1overior2 * *ray.get_direction() + (ior1overior2 * cos_theta - t2) * n;
                next.set_direction(refracted_direction);
            } else {
                let refracted_direction =
                    ior1overior2 * *ray.get_direction() - (-ior1overior2 * cos_theta - t2) * n;
                next.set_direction(refracted_direction);
            }
            next.set_direction(next.get_direction().normalize());

            if next.hit.normal.dot(*next.get_direction()).signum()
                != next.hit.normal.dot(*ray.get_direction()).signum()
            {
                let factor = next.hit.normal.dot(*ray.get_direction()).signum() * -Ray::EPSILON
                    - next.get_direction().dot(next.hit.normal);
                next.set_direction(*next.get_direction() + factor * next.hit.normal);
                next.set_direction(next.get_direction().normalize());
            }
            next.origin = next.at(Ray::OFFSET);
        }
        if path_trace(rng, scene, next, false, attenuation, current_spp) {
            hit = true;
            translucent_ray_color(scene, ray, next, cumulative_color, absorption);
        }
    }
    hit
}

pub fn do_transmission(
    ray: &Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    absorption: f32,
    scene: &Scene,
    attenuation: &mut Vec4,
    rng: &mut StdRng,
    current_spp: u32,
) -> bool {
    println!("transmission");
    let mut hit = false;
    *next = ray.clone();
    next.origin = next.at(Ray::OFFSET);

    if path_trace(rng, scene, next, false, attenuation, current_spp) {
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
    let rgb_trans;
    //todo: implement fancy translucent ray color
    rgb_trans = Vec3A::from(ray.hit.color.xyz()) * absorption;

    let output_color;
    output_color = Vec4::new(rgb_trans.x, rgb_trans.y, rgb_trans.z, 1.0) * next.hit.color;

    *cumulative_color += output_color;
}
pub fn next_intersection(scene: &Scene, ray: &mut Ray) -> bool {
    ray.hit.previous_material = ray.hit.current_material.clone();
    ray.hit.t = INFINITY;
    if scene.hit(ray) {
        return true;
    }

    return false;
}

pub fn get_direct_light_attenuation(scene: &Scene, ray: &mut Ray, attenuation: &mut Vec4) {
    *attenuation = Vec4::splat(1.0);
    while attenuation.w > 0.0 {
        ray.origin = ray.at(Ray::OFFSET);
        if !next_intersection(scene, ray) {
            break;
        }
        let mult = 1.0 - ray.hit.color.w;
        attenuation.x *= ray.hit.color.x * ray.hit.color.w + mult;
        attenuation.y *= ray.hit.color.y * ray.hit.color.w + mult;
        attenuation.z *= ray.hit.color.z * ray.hit.color.w + mult;
        attenuation.w *= mult;

        if scene.sun_sampling_strategy.strict_direct_light
            && ray.hit.previous_material.index_of_refraction
                != ray.hit.current_material.index_of_refraction
        {
            attenuation.w = 0.0;
            println!("umm");
        }
    }
}

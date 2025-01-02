use core::f32;
use std::{f32::INFINITY, path};

use crate::{material, Material, MaterialFlags, Ray, Scene};
use glam::{Vec3A as Vec3, Vec4, Vec4Swizzles};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub fn path_trace(scene: &Scene, ray: &mut Ray, first_reflection: bool) -> bool {
    let mut hit: bool = false;
    let mut rng = StdRng::from_entropy();
    let ray_origin = ray.origin.clone();
    let ray_direction = ray.direction.clone();

    let mut air_distance: f32 = 0.0;

    loop {
        if !next_intersection(scene, ray) {
            if ray.hit.specular {
                ray.hit.color = Vec4::new(
                    Scene::SKY_COLOR.x,
                    Scene::SKY_COLOR.y,
                    Scene::SKY_COLOR.z,
                    1.0,
                );
                hit = true;
            } else {
                ray.hit.color = Vec4::new(
                    Scene::SKY_COLOR.x,
                    Scene::SKY_COLOR.y,
                    Scene::SKY_COLOR.z,
                    1.0,
                );
                hit = true;
            }
            break;
        }
        let current_material = ray.hit.current_material;
        let prev_material = ray.hit.previous_material;

        let specular = scene.materials[current_material as usize].specular;

        let diffuse = ray.hit.color.w;
        let absorb = ray.hit.color.w;

        let ior1 = scene.materials[current_material as usize].index_of_refraction;
        let ior2 = scene.materials[prev_material as usize].index_of_refraction;

        if ray.hit.color.w + specular < Ray::EPSILON && ior1 == ior2 {
            continue;
        }

        if ray.hit.depth + 1 >= 5 {
            break;
        }
        ray.hit.depth += 1;

        let mut cumm_color = Vec4::splat(0.0);
        let mut next = Ray::default();

        let metal = scene.materials[current_material as usize].metalness;

        let count = if first_reflection {
            scene.get_current_branch_count()
        } else {
            1
        };

        (0..count).for_each(|_| {
            let do_metal = metal > Ray::EPSILON && rng.gen::<f32>() < metal;
            if do_metal || (specular > Ray::EPSILON && rng.gen::<f32>() < specular) {
                hit |= do_specular_reflection(
                    ray,
                    &mut next,
                    &mut cumm_color,
                    do_metal,
                    &mut rng,
                    scene,
                );
            } else if rng.gen::<f32>() < diffuse {
                hit |= do_diffuse_reflection(
                    ray,
                    &mut next,
                    &mut cumm_color,
                    &scene.materials[current_material as usize],
                    &mut rng,
                    scene,
                );
            } else if ior1 != ior2 {
                hit |= do_refraction(
                    ray,
                    &mut next,
                    &scene.materials[current_material as usize],
                    &scene.materials[prev_material as usize],
                    &mut cumm_color,
                    ior1,
                    ior2,
                    absorb,
                    &mut rng,
                    scene,
                );
            } else {
                hit |= do_transmission(ray, &mut next, &mut cumm_color, absorb, scene)
            }
        });

        ray.hit.color = cumm_color / count as f32;

        break;
    }
    if !hit {
        ray.hit.color = Vec4::new(0.0, 0.0, 0.0, 1.0);
        if first_reflection {
            air_distance = ray.distance_travelled;
        }
    }
    hit
}

pub fn do_specular_reflection(
    ray: &Ray,
    next: &mut Ray,
    cumulative_color: &mut Vec4,
    do_metal: bool,
    rng: &mut StdRng,
    scene: &Scene,
) -> bool {
    let mut hit = false;
    *next = ray.specular_reflection(
        scene.materials[ray.hit.current_material as usize].roughness,
        rng,
    );

    if path_trace(scene, next, false) {
        if do_metal {
            cumulative_color.x = ray.hit.color.x * next.hit.color.x;
            cumulative_color.y = ray.hit.color.y * next.hit.color.y;
            cumulative_color.z = ray.hit.color.z * next.hit.color.z;
        } else {
            cumulative_color.x = next.hit.color.x;
            cumulative_color.y = next.hit.color.y;
            cumulative_color.z = next.hit.color.z;
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
) -> bool {
    let mut hit = false;
    let emmitance = Vec3::splat(0.0);
    let indirect_emmitter_color = Vec4::splat(0.0);

    //(scene.emittersEnabled && (!scene.isPreventNormalEmitterWithSampling() || scene.getEmitterSamplingStrategy() == EmitterSamplingStrategy.NONE || ray.depth == 1) && currentMat.emittance > Ray.EPSILON)
    let ray_color = ray.hit.color.clone();

    *next = ray.diffuse_reflection(rng);

    hit = path_trace(scene, next, false) || hit;

    if hit {
        cumulative_color.x = ray_color.x * next.hit.color.x;
        cumulative_color.y = ray_color.y * next.hit.color.y;
        cumulative_color.z = ray_color.z * next.hit.color.z;
    }

    ray.hit.color = ray_color;

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
) -> bool {
    let mut hit = false;
    let do_refraction = current_material
        .material_flags
        .contains(MaterialFlags::REFRACTIVE)
        || current_material
            .material_flags
            .contains(MaterialFlags::REFRACTIVE);

    let ior1overior2 = ior1 / ior2;
    let cos_theta = -ray.direction.dot(ray.hit.outward_normal);
    let radicand = 1.0 - ior1overior2.powi(2) * (1.0 - cos_theta.powi(2));

    if do_refraction && radicand < Ray::EPSILON {
        *next = ray.specular_reflection(current_material.roughness, rng);
        if path_trace(scene, next, false) {
            hit = true;
            cumulative_color.x = next.hit.color.x;
            cumulative_color.y = next.hit.color.y;
            cumulative_color.z = next.hit.color.z;
        }
    } else {
        *next = ray.clone();

        let a = ior1overior2 - 1.0;
        let b = ior1overior2 + 1.0;

        let r0 = a * a / (b * b);
        let c: f32 = 1.0 - cos_theta;
        let rtheta = r0 + (1.0 - r0) * c.powi(5);

        if rng.gen::<f32>() < rtheta {
            *next = ray.specular_reflection(current_material.roughness, rng);
            if path_trace(scene, next, false) {
                hit = true;
                cumulative_color.x = next.hit.color.x;
                cumulative_color.y = next.hit.color.y;
                cumulative_color.z = next.hit.color.z;
            }
        } else if do_refraction {
            let t2 = radicand.sqrt();
            let n = ray.hit.outward_normal;
            if cos_theta > 0.0 {
                let refracted_direction =
                    ior1overior2 * ray.direction + (ior1overior2 * cos_theta - t2) * n;
                next.direction = refracted_direction;
            } else {
                let refracted_direction =
                    ior1overior2 * ray.direction - (-ior1overior2 * cos_theta - t2) * n;
                next.direction = refracted_direction;
            }
            next.direction = next.direction.normalize();

            if next.hit.geom_normal.dot(next.direction).signum()
                != next.hit.geom_normal.dot(ray.direction).signum()
            {
                let factor = next.hit.geom_normal.dot(ray.direction).signum() * -Ray::EPSILON
                    - next.direction.dot(next.hit.geom_normal);
                next.direction += factor * next.hit.geom_normal;
                next.direction = next.direction.normalize();
            }
            next.origin = next.at(Ray::OFFSET);
        }
        if path_trace(scene, next, false) {
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
) -> bool {
    let mut hit = false;
    *next = ray.clone();
    next.origin = next.at(Ray::OFFSET);

    if path_trace(scene, next, false) {
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
    let mut rgb_trans = Vec3::splat(1.0) - absorption;
    //todo: implement fancy translucent ray color
    rgb_trans = Vec3::from(ray.hit.color.xyz()) * absorption;

    let mut output_color = Vec4::splat(0.0);
    output_color = Vec4::new(rgb_trans.x, rgb_trans.y, rgb_trans.z, 1.0) * next.hit.color;

    *cumulative_color += output_color;
}

pub fn next_intersection(scene: &Scene, ray: &mut Ray) -> bool {
    ray.hit.previous_material = ray.hit.current_material;
    ray.hit.t = INFINITY;
    let mut hit = false;
    if scene.hit(ray) {
        return true;
    }
    if hit {
        ray.distance_travelled += ray.hit.t;
        ray.origin = ray.at(ray.hit.t);
        return true;
    } else {
        return false;
    }
}

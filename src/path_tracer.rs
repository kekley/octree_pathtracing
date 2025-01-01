use core::f32;
use std::f32::INFINITY;

use crate::{Ray, Scene};
use glam::{Vec3A as Vec3, Vec4};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub fn path_trace(scene: &Scene, ray: &mut Ray, first_reflection: bool) -> bool {
    let mut hit: bool = false;
    let mut rng = StdRng::from_entropy();
    let ray_origin = ray.origin;
    let ray_direction = ray.direction;

    let mut air_distance: f32 = 0.0;

    loop {
        if !next_intersection(scene, ray) {
            // If no intersection, return sky color
            ray.hit.color = Vec4::new(
                Scene::SKY_COLOR.x,
                Scene::SKY_COLOR.y,
                Scene::SKY_COLOR.z,
                1.0,
            );
            break;
        }
        let current_material = ray.hit.current_material;
        let prev_material = ray.hit.previous_material;

        let specular = scene.materials[current_material as usize].specular;

        let diffuse = ray.hit.color.w;

        let ior1 = scene.materials[current_material as usize].index_of_refraction;
        let ior2 = scene.materials[prev_material as usize].index_of_refraction;

        if ray.hit.color.w + specular < Ray::EPSILON && ior1 == ior2 {
            continue;
        }

        if ray.hit.depth + 1 >= 5 {
            break;
        }
        ray.hit.depth += 1;

        let cumm_color = Vec4::splat(0.0);
        let mut next = Ray::default();

        let metal = scene.materials[current_material as usize].metalness;

        // Reusing first rays - a simplified form of "branched path tracing" (what Blender used to call it before they implemented something fancier)
        // The initial rays cast into the scene are very similar between each sample, since they are almost entirely a function of the pixel coordinates
        // Because of that, casting those initial rays on every sample is redundant and can be skipped
        // If the ray depth is high, this doesn't help much (just a few percent), but in some outdoor/low depth scenes, this can improve performance by >40%
        // The main caveat is that antialiasing is achieved by varying the starting rays at the subpixel level (see PathTracingRenderer.java)
        // Therefore, it's still necessary to have a decent amount (20 is ok, 50 is better) of distinct starting rays for each pixel
        // scene.branchCount is the number of times we use the same first ray before casting a new one

        let count = if first_reflection {
            scene.get_current_branch_count()
        } else {
            1
        };

        (0..count).for_each(|_| {
            let do_metal = metal > Ray::EPSILON && rng.gen::<f32>() < metal;
            if do_metal || (specular > Ray::EPSILON && rng.gen::<f32>() < specular) {
                hit |= true; //FIXME: do specular reflection func
            } else if rng.gen::<f32>() < diffuse {
                hit |= true; //FIXME: do diffuse reflection func
            } else if ior1 != ior2 {
                hit |= true; //FIXME: do refraction func
            } else {
                hit |= true; //FIXME: do default transmission func
            }
        });

        ray.hit.color = cumm_color;
        ray.hit.color *= 1.0 / count as f32;

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

pub fn do_specular_reflection(ray: &Ray, )->bool{
    let mut hit = false;

}


pub fn next_intersection(scene: &Scene, ray: &mut Ray) -> bool {
    ray.hit.previous_material = ray.hit.current_material;
    ray.hit.t = INFINITY;
    let mut hit = false;
    if scene.hit(ray) {
        return true;
    }
    todo!()
}

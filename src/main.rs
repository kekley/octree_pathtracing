extern crate ray_tracing;
use std::error::Error;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::{fs::File, io::Write, time::Instant};

use anyhow::Ok;
use glam::Vec3A as Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use ray_tracing::{BVHTree, Scene};
use ray_tracing::{Camera, HittableBVH};
use ray_tracing::{Cuboid, Material};
use ray_tracing::{HitList, Hittable};
use ray_tracing::{TileRenderer, AABB};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use spider_eye::{ChunkCoords, World, WorldCoords};
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    let start = Instant::now();

    blocks()?;

    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
    Ok(())
}
/*
fn checkered_spheres() {
    let mut world = HitList::new();

    let texture_black = Texture::Color(Vec3::splat(0.6));
    let texture_white = Texture::Color(Vec3::splat(1.0));
    let checker_texture = Texture::CheckerBoard {
        inv_scale: 1.0 / 0.32,
        a: Box::new(texture_black),
        b: Box::new(texture_white),
    };

    let ground_material = Material::Lambertian {
        texture: &checker_texture,
    };
    let earth_texture = Texture::Image(RTWImage::load("./assets/greasy.png"));

    let earth_surface = Material::Lambertian {
        texture: &earth_texture,
    };

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0.0, -1000.0, 0.0),
        1000.0,
        &ground_material,
    )));

    let mut materials = Vec::new();
    let mut rng = Rng::new();
    (-11..11).for_each(|a| {
        for b in (-11..11) {
            let choose_mat = random_float(&mut rng);
            let center = Vec3::new(
                a as f32 + 0.9 * random_float(&mut rng),
                0.2,
                b as f32 + 0.9 * random_float(&mut rng),
            );
            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                let sphere_material: Material;
                match choose_mat {
                    //diffuse
                    _mat if choose_mat < 0.8 => {
                        let albedo = random_vec(&mut rng);
                        sphere_material = Material::Lambertian {
                            texture: Box::leak(Box::new(Texture::Color(albedo))),
                        };
                    }
                    _mat if choose_mat < 0.95 => {
                        let fuzz = random_float_in_range(&mut rng, 0.0, 0.5);
                        sphere_material = Material::Metal {
                            texture: &earth_texture,
                            fuzz: fuzz,
                        };
                    }
                    _ => {
                        sphere_material = Material::Dielectric {
                            refraction_index: 1.5,
                        };
                    }
                }
                materials.push((sphere_material.clone(), center));
            }
        }
    });

    materials.iter().for_each(|sphere_data| {
        world.add(Hittable::Sphere(Sphere::new(
            sphere_data.1,
            0.2,
            &sphere_data.0,
        )));
    });

    let material1 = Material::Dielectric {
        refraction_index: 1.5,
    };
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0.0, 1.0, 0.0),
        1.0,
        &material1,
    )));

    let yellow_tex = Texture::Color(Vec3::new(0.4, 0.2, 0.1));

    let material2 = Material::Lambertian {
        texture: &yellow_tex,
    };

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(-4.0, 1.0, 0.0),
        1.0,
        &earth_surface,
    )));

    let gray = Vec3::new(0.7, 0.6, 0.5);

    let material3 = Material::Metal {
        texture: &checker_texture,
        fuzz: 0.1,
    };
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(4.0, 1.0, 0.0),
        1.0,
        &material3,
    )));

    let bvh_world = BVHTree::from_hit_list(&world);

    let mut camera = Camera::new();
    camera.aspect_ratio = ASPECT_RATIO;
    camera.image_width = 1000;
    camera.samples_per_pixel = 200;
    camera.max_depth = 50;
    camera.v_fov = 20.0;
    camera.look_from = Vec3::new(13.0, 2.0, 3.0);
    camera.look_at = Vec3::new(0.0, 0.0, 0.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);
    camera.defocus_angle = 0.1;
    camera.focus_dist = 10.0;

    let buf = camera.multi_threaded_render(&Hittable::BVH(bvh_world));

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
}

fn earth() {
    let earth_texture = Texture::Image(RTWImage::load("./assets/earthmap.jpg"));

    let earth_surface = Material::Lambertian {
        texture: &earth_texture,
    };

    let globe = Hittable::Sphere(Sphere::new(Vec3::splat(0.0), 2.0, &earth_surface));

    let mut camera = Camera::new();

    camera.aspect_ratio = 16.0 / 9.0;
    camera.image_width = 1200;
    camera.samples_per_pixel = 500;
    camera.max_depth = 50;
    camera.v_fov = 20.0;
    camera.look_from = Vec3::new(0.0, 0.0, 12.0);
    camera.look_at = Vec3::splat(0.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);

    camera.defocus_angle = 0.0;

    let buf = camera.multi_threaded_render(&globe);

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
}

fn cube() {
    let greasy_texture = Texture::Image(RTWImage::load("./assets/greasy.jpg"));

    let cube_surface = Material::Lambertian {
        texture: &greasy_texture,
    };

    let bounds = AABB::from_points(Vec3::splat(-1.0), Vec3::splat(1.0));
    let cube = Hittable::Box(Cuboid::new(bounds, &cube_surface));

    let mut camera = Camera::new();

    camera.aspect_ratio = 16.0 / 9.0;
    camera.image_width = 1200;
    camera.samples_per_pixel = 500;
    camera.max_depth = 50;
    camera.v_fov = 20.0;
    camera.look_from = Vec3::new(-10.0, 10.0, 10.0);
    camera.look_at = Vec3::splat(0.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);

    camera.defocus_angle = 0.0;

    let buf = camera.multi_threaded_render(&cube);

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
}
 */

fn blocks() -> Result<(), anyhow::Error> {
    let mut hitlist = HitList::new();
    let mut world = World::new("./hous");

    let chunk_view_distance: i32 = 500;
    let camera = Camera::look_at(
        Vec3::splat(0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
        70.0,
    );
    let mut scene = Scene::new().branch_count(1).camera(camera).spp(1).build();
    let materials = vec![Material::default()];
    scene.add_cube(Cuboid::new(
        AABB::from_points(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        0,
    ));
    scene.materials = materials;
    let mut rng = StdRng::from_entropy();
    let a = TileRenderer::new((500, 500), 8, scene);
    a.render();
    Ok(())
}

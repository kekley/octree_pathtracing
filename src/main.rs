extern crate ray_tracing;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Read};
use std::{fs::File, io::Write, time::Instant};

use fastrand::Rng;
use ray_tracing::Camera;
use ray_tracing::Cuboid;
use ray_tracing::Material;
use ray_tracing::RTWImage;
use ray_tracing::Sphere;
use ray_tracing::Texture;
use ray_tracing::Vec3;
use ray_tracing::AABB;
use ray_tracing::{random_float, random_float_in_range, random_vec};
use ray_tracing::{BVHTree, TextureManager};
use ray_tracing::{HitList, Hittable};
use spider_eye::{ChunkData, Region, SpiderEyeError};

pub const ASPECT_RATIO: f64 = 1.5;

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    chunk();

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
                a as f64 + 0.9 * random_float(&mut rng),
                0.2,
                b as f64 + 0.9 * random_float(&mut rng),
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
fn chunk() -> Result<(), Box<dyn Error>> {
    let mut region_file = File::open("r.0.0.mca").unwrap();

    let mut file_data = Vec::with_capacity(region_file.metadata().unwrap().len() as usize);

    region_file.read_to_end(&mut file_data).unwrap();

    let file_data = Cursor::new(file_data);

    let mut region = Region::from_stream(file_data)?;

    let chunk = region.get_chunk(0, 0).ok_or("Error getting chunk")?;

    let data = ChunkData::from_compound(chunk.data);

    let mut hitlist = HitList::new();
    let mut material_manager = TextureManager::new();

    data.sections
        .iter()
        .enumerate()
        .for_each(|(section_idx, section)| {
            (0..16).for_each(|y| {
                (0..16).for_each(|z| {
                    (0..16).for_each(|x| {
                        let block = section.block_states.get_block(x, y, z);
                        if !(block.0 == "minecraft:air") && !(block.0 == "minecraft:grass_block") {
                            println!("{}", block.0);
                            let start_pos = Vec3::new(
                                x as f64,
                                (y as u32 + section_idx as u32 * 16) as f64,
                                z as f64,
                            );
                            println!("{:?}", start_pos);

                            let mat = material_manager.get_or_make_material_idx(&block.0);
                            let end_pos = start_pos + 1.0;

                            let block = Cuboid::new(AABB::from_points(start_pos, end_pos), mat);
                            hitlist.add(Hittable::Box(block));
                        }
                    });
                });
            });
        });

    println!("{:?}", hitlist.objects.len());

    let tree = BVHTree::from_hit_list(&hitlist);

    let mut camera = Camera::new();

    camera.aspect_ratio = 16.0 / 9.0;
    camera.image_width = 2560;
    camera.samples_per_pixel = 500;
    camera.max_depth = 50;
    camera.v_fov = 20.0;
    camera.look_from = Vec3::new(-20.0, 7.0, -20.0);
    camera.look_at = Vec3::new(0.0, 4.5, 0.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);

    camera.defocus_angle = 0.0;

    let buf = camera.multi_threaded_render(&Hittable::BVH(tree), material_manager.clone());

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();

    Ok(())
}

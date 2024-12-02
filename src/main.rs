extern crate ray_tracing;
use core::str;
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
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::vec;
use spider_eye::{Chunk, ChunkData, Region, SpiderEyeError, World};

pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    world()?;

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
fn chunk() -> Result<(), Box<dyn Error>> {
    let mut region_file = File::open("r.0.0.mca").unwrap();

    let mut file_data = Vec::with_capacity(region_file.metadata().unwrap().len() as usize);

    region_file.read_to_end(&mut file_data).unwrap();

    let file_data = Cursor::new(file_data);

    let mut region = Region::from_stream(file_data)?;

    let mut hitlist = HitList::new();
    let mut material_manager = TextureManager::new();
    let mut camera = Camera::new();

    camera.aspect_ratio = 16.0 / 9.0;
    camera.image_width = 400;
    camera.samples_per_pixel = 4;
    camera.max_depth = 10;
    camera.v_fov = 70.0;
    camera.look_from = Vec3::new(0.0, -57.0, 0.0);
    camera.look_at = Vec3::new(10.0, -59.0, 10.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);

    camera.defocus_angle = 0.0;

    for chunk in region.chunk_segments {
        if let Some(segment) = chunk {
            let chunk_data = region.read_chunk_from_segment(segment);

            let chunk = Chunk::from_slice(chunk_data).unwrap();
            let pos = Vec3::new(chunk.xpos as f32, 0.0 as f32, chunk.zpos as f32);
            let distance = ((pos * 16.0) - camera.look_from).length();
            if distance >= 128.0 {
                continue;
            }
            let data = chunk.get_data();

            (-64..320).for_each(|y: i16| {
                (0..16).for_each(|x: i16| {
                    (0..16).for_each(|z: i16| {
                        let chunk_x = chunk.xpos * 16;
                        let chunk_z = chunk.zpos * 16;
                        let block = data.get_block(x, y, z);
                        if !(block.0 == "minecraft:air") {
                            let start_pos = Vec3::new(
                                chunk_x as f32 + x as f32,
                                y as f32,
                                chunk_z as f32 + z as f32,
                            );

                            let mat = if (block.0 == "minecraft:grass_block") {
                                material_manager.get_or_make_material_idx("minecraft:grass_block")
                            } else {
                                material_manager
                                    .get_or_make_material_idx(str::from_utf8(block.0).unwrap())
                            };
                            let end_pos = start_pos + 1.0;

                            let block = Cuboid::new(AABB::from_points(start_pos, end_pos), mat);
                            hitlist.add(Hittable::Box(block));
                        }
                    });
                });
            });
        }
    }

    println!("{:?}", hitlist.objects.len());

    let tree = BVHTree::from_hit_list(&hitlist);

    let buf = camera.multi_threaded_render(&Hittable::BVH(tree), material_manager.clone());

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();

    Ok(())
}

fn world() -> Result<(), Box<dyn Error>> {
    let mut hitlist = HitList::new();
    let mut material_manager = TextureManager::new();
    let mut camera = Camera::new();

    camera.aspect_ratio = 16.0 / 9.0;
    camera.image_width = 400;
    camera.samples_per_pixel = 4;
    camera.max_depth = 10;
    camera.v_fov = 70.0;
    camera.look_from = Vec3::new(0.0, -57.0, 0.0);
    camera.look_at = Vec3::new(10.0, -59.0, 10.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);

    camera.defocus_angle = 0.0;

    //world stuff here
    let world = World::new("./world");

    let chunk_view_distance: i32 = 16;
    let starting_chunk_x = (camera.look_from.x as i32) >> 4;
    let starting_chunk_z = (camera.look_from.z as i32) >> 4;
    // Estimate the number of regions based on chunk view distance
    let estimated_regions = (chunk_view_distance * chunk_view_distance) as usize;

    let mut regions: Vec<Region> = Vec::with_capacity(estimated_regions);
    for x in starting_chunk_x..starting_chunk_x + chunk_view_distance {
        for z in starting_chunk_z..starting_chunk_z + chunk_view_distance {
            if let Some(region) = world.get_region_containing_chunk(x, z) {
                regions.push(region.clone());
            }
        }
    }

    let a = regions
        .into_iter()
        .filter(|region| {
            (((region.x * 32).pow(2) + (region.z * 32).pow(2)) as f32).sqrt()
                <= chunk_view_distance as f32
        })
        .map(|region| {
            let segments = region.chunk_segments;
            let chunks = segments
                .into_par_iter()
                .filter_map(|opt| {
                    if let Some(segment) = opt {
                        let chunk = region.read_chunk_from_segment(segment);
                        let chunk = Chunk::from_slice(chunk).unwrap();
                        let dist = (((chunk.xpos * 16) as f32 - camera.look_from.x).powi(2)
                            + ((chunk.zpos * 16) as f32 - camera.look_from.z).powi(2))
                        .sqrt();
                        if dist <= chunk_view_distance as f32 {
                            return Some(chunk);
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                })
                .collect::<Vec<_>>();
            chunks
        })
        .collect::<Vec<_>>();

    println!("{:?}", a.len());
    //let tree = BVHTree::from_hit_list(&hitlist);

    //let buf = camera.multi_threaded_render(&Hittable::BVH(tree), material_manager.clone());

    //file to write to
    //let mut file = File::create("./output.ppm").unwrap();

    //file.write(&buf[..]).unwrap();

    Ok(())
}

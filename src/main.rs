use std::{
    fs::File,
    io::Write,
    sync::Arc,
    time::{self, Duration, Instant},
};

use camera::Camera;
use hittable::{HitList, Hittable};
use interval::Interval;
use material::Material;
use ray::Ray;
use sphere::Sphere;
use util::INFINITY;
use vec3::Vec3;

pub const ASPECT_RATIO: f64 = 16f64 / 9f64;

mod camera;
mod hittable;
mod interval;
mod material;
mod ray;
mod sphere;
mod util;
mod vec3;
#[tokio::main]
async fn main() {
    let start = time::Instant::now();
    let mut world = HitList::new();

    let ground_material = Arc::new(Material::Lambertian {
        albedo: Vec3::new(0.5, 0.5, 0.5),
    });

    let center_material = Arc::new(Material::Lambertian {
        albedo: Vec3::new(0.1, 0.2, 0.5),
    });

    let left_material = Arc::new(Material::Dielectric {
        refraction_index: 1.5,
    });

    let bubble_material = Arc::new(Material::Dielectric {
        refraction_index: 1.00 / 1.50,
    });

    let right_material = Arc::new(Material::Metal {
        albedo: Vec3::new(0.8, 0.6, 0.2),
        fuzz: 1.0,
    });

    let metal_material = Arc::new(Material::Metal {
        albedo: Vec3::splat(0.8),
        fuzz: 0.0,
    });

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0f64, -100.5f64, -1f64),
        100f64,
        ground_material.clone(),
    )));

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0.0, 0.0, -1.2),
        0.5,
        center_material.clone(),
    )));

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(-1.0, 0.0, -1.0),
        0.5,
        left_material.clone(),
    )));

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(-1.0, 0.0, -1.0),
        0.4,
        bubble_material.clone(),
    )));

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(1.0, 0.0, -1.0),
        0.5,
        right_material.clone(),
    )));

    let mut camera = Camera::new();
    camera.aspect_ratio = ASPECT_RATIO;
    camera.image_width = 400;
    camera.samples_per_pixel = 100;
    camera.max_depth = 50;

    let buf = camera.render(&world);

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
}

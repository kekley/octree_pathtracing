use std::{fs::File, io::Write};

use camera::Camera;
use hittable::{HitList, Hittable};
use interval::Interval;
use ray::Ray;
use sphere::Sphere;
use util::INFINITY;
use vec3::Vec3;

pub const ASPECT_RATIO: f64 = 16f64 / 9f64;

mod camera;
mod hittable;
mod interval;
mod ray;
mod sphere;
mod util;
mod vec3;
fn main() {
    let mut world = HitList::new();
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0f64, 0f64, -1f64),
        0.5f64,
    )));
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0f64, -100.5f64, -1f64),
        100f64,
    )));

    let mut camera = Camera::new();
    camera.aspect_ratio = ASPECT_RATIO;
    camera.image_width = 1600;

    let buf = camera.render(&world);

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
}

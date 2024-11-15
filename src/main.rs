use std::{fs::File, io::Write, time::Instant};

use bvh::BVHTree;
use camera::Camera;
use fastrand::Rng;
use hittable::{HitList, Hittable};
use material::Material;
use sphere::Sphere;
use util::{random_float, random_float_in_range, random_vec};
use vec3::Vec3;

pub const ASPECT_RATIO: f64 = 1.5;

mod aabb;
mod bvh;
mod camera;
mod hittable;
mod interval;
mod material;
mod ray;
mod sphere;
mod util;
mod vec3;
fn main() {
    let start = Instant::now();
    let mut world = HitList::new();

    let ground_material = Material::Lambertian {
        albedo: Vec3::new(0.5, 0.5, 0.5),
    };

    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0.0, -1000.0, 0.0),
        1000.0,
        ground_material,
    )));

    let mut materials: Vec<(Material, Vec3)> = vec![];
    let mut rng = Rng::new();
    for a in 0..1 {
        for b in 0..1 {
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
                        sphere_material = Material::Lambertian { albedo: albedo };
                    }
                    _mat if choose_mat < 0.95 => {
                        let albedo = random_vec(&mut rng);
                        let fuzz = random_float_in_range(&mut rng, 0.0, 0.5);
                        sphere_material = Material::Metal {
                            albedo: albedo,
                            fuzz: fuzz,
                        };
                    }
                    _ => {
                        sphere_material = Material::Dielectric {
                            refraction_index: 1.5,
                        };
                    }
                }
                materials.push((sphere_material, center));
            }
        }
    }

    materials.iter().for_each(|sphere_data| {
        world.add(Hittable::Sphere(Sphere::new(
            sphere_data.1,
            0.2,
            sphere_data.0.clone(),
        )));
    });

    let material1 = Material::Dielectric {
        refraction_index: 1.5,
    };
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(0.0, 1.0, 0.0),
        1.0,
        material1,
    )));

    let material2 = Material::Lambertian {
        albedo: Vec3::new(0.4, 0.2, 0.1),
    };
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(-4.0, 1.0, 0.0),
        1.0,
        material2,
    )));

    let material3 = Material::Metal {
        albedo: Vec3::new(0.7, 0.6, 0.5),
        fuzz: 0.0,
    };
    world.add(Hittable::Sphere(Sphere::new(
        Vec3::new(4.0, 1.0, 0.0),
        1.0,
        material3,
    )));

    let world = BVHTree::from_hit_list(&world);

    let mut camera = Camera::new();
    camera.aspect_ratio = ASPECT_RATIO;
    camera.image_width = 1200;
    camera.samples_per_pixel = 100;
    camera.max_depth = 50;
    camera.v_fov = 30.0;
    camera.look_from = Vec3::new(13.0, 2.0, 3.0);
    camera.look_at = Vec3::new(0.0, 0.0, 0.0);
    camera.v_up = Vec3::new(0.0, 1.0, 0.0);
    camera.defocus_angle = 0.1;
    camera.focus_dist = 10.0;

    let buf = camera.multi_threaded_render(Box::new(Hittable::BVH(world)));

    //file to write to
    let mut file = File::create("./output.ppm").unwrap();

    file.write(&buf[..]).unwrap();
    let finish = Instant::now();
    let duration = finish - start;

    println!("time elapsed: {}", duration.as_millis());
}

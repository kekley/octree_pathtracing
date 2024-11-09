use std::{fs::File, io::Write};

use ray::Ray;
use vec3::Vec3;

pub const IMAGE_WIDTH: u64 = 400;
pub const ASPECT_RATIO: f64 = 16f64 / 9f64;
pub const IMAGE_HEIGHT: u64 = (IMAGE_WIDTH as f64 / ASPECT_RATIO) as u64;

mod hittable;
mod ray;
mod sphere;
mod vec3;
fn main() {
    assert!(IMAGE_HEIGHT >= 1);

    let focal_length: f64 = 1.0;

    let viewport_height: f64 = 2.0;
    let viewport_width: f64 = viewport_height * IMAGE_WIDTH as f64 / IMAGE_HEIGHT as f64;
    let camera_center = Vec3::splat(0f64);

    let viewport_u = Vec3::new(viewport_width, 0f64, 0f64);
    let viewport_v = Vec3::new(0f64, -viewport_height, 0f64);

    let pixel_delta_u = viewport_u / IMAGE_WIDTH as f64;
    let pixel_delta_v = viewport_v / IMAGE_HEIGHT as f64;

    let viewport_upper_left =
        camera_center - Vec3::new(0f64, 0f64, focal_length) - viewport_u / 2f64 - viewport_v / 2f64;

    let pixel00_loc = viewport_upper_left + 0.5f64 * (pixel_delta_u + pixel_delta_v);
    let mut file = File::create("./output.ppm").unwrap();
    let mut buf = Vec::with_capacity(600 * 1024);
    buf.write_fmt(format_args!("P3\n{} {}\n255\n", IMAGE_WIDTH, IMAGE_HEIGHT))
        .unwrap();
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let pixel_center =
                pixel00_loc + (x as f64 * pixel_delta_u) + (y as f64 * pixel_delta_v);
            let ray_direction = pixel_center - camera_center;

            let ray = Ray::new(camera_center, ray_direction);

            let color = ray_color(&ray);
            Vec3::write_rgb8_color_as_text_to_stream(&color, &mut buf);
        }
    }
    file.write(&buf[..]).unwrap();
}

fn ray_color(ray: &Ray) -> Vec3 {
    let t = hit_sphere(Vec3::new(0f64, 0f64, -1f64), 0.5f64, &ray);

    if t > 0f64 {
        let normal = (ray.at(t) - Vec3::new(0f64, 0f64, -1f64)).normalize();

        return 0.5f64 * Vec3::new(normal.x + 1f64, normal.y + 1f64, normal.z + 1f64);
    }

    let unit_direction = ray.direction.normalize();

    let a = 0.5f64 * (unit_direction.y + 1f64);

    let final_color = (1f64 - a) * Vec3::ONE + a * Vec3::new(0.5f64, 0.7f64, 1.0f64);

    final_color
}

fn hit_sphere(center: Vec3, radius: f64, ray: &Ray) -> f64 {
    let origin_to_center = center - ray.origin;
    let a = ray.direction.length_squared();
    let h = ray.direction.dot(origin_to_center);
    let c = origin_to_center.length_squared() - radius * radius;

    let discriminant = h * h - a * c;

    if discriminant < 0f64 {
        -1f64
    } else {
        (h - f64::sqrt(discriminant)) / a
    }
}

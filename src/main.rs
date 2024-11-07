use std::{fs::File, io::Write};

use vec3::Vec3;

pub const IMAGE_WIDTH: u64 = 400;
pub const ASPECT_RATIO: f64 = 16f64 / 9f64;
pub const IMAGE_HEIGHT: u64 = (IMAGE_WIDTH as f64 / ASPECT_RATIO) as u64;
pub const VIEWPORT_HEIGHT: f64 = 2.0;
pub const VIEWPORT_WIDTH: f64 = VIEWPORT_HEIGHT * (IMAGE_WIDTH / IMAGE_HEIGHT) as f64;

mod ray;
mod vec3;
fn main() {
    assert!(IMAGE_HEIGHT >= 1);
    let mut file = File::create("./output.ppm").unwrap();
    let mut buf = Vec::with_capacity(600 * 1024);
    buf.write_fmt(format_args!("P3\n{} {}\n255\n", IMAGE_WIDTH, IMAGE_HEIGHT))
        .unwrap();
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let red: f64 = x as f64 / (IMAGE_WIDTH as f64 - 1f64);
            let green: f64 = y as f64 / (IMAGE_WIDTH as f64 - 1f64);
            let blue: f64 = 0f64;

            let color = Vec3::new(red * 255.999, green * 255.999, blue * 255.999);

            Vec3::write_as_text_to_stream(&color, &mut buf);
        }
    }
    file.write(&mut buf[..]).unwrap();
}

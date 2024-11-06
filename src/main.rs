use std::{fs::File, io::Write};

pub const IMAGE_WIDTH: u16 = 256;
pub const IMAGE_HEIGHT: u16 = 256;

fn main() {
    let mut file = File::create("./output.ppm").unwrap();
    let mut buf = Vec::with_capacity(600 * 1024);
    buf.write_fmt(format_args!("P3\n{} {}\n255\n", IMAGE_WIDTH, IMAGE_HEIGHT))
        .unwrap();
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let red: f32 = x as f32 / (IMAGE_WIDTH as f32 - 1f32);
            let green: f32 = y as f32 / (IMAGE_WIDTH as f32 - 1f32);
            let blue: f32 = 0f32;

            let int_red: u8 = (255.999f32 * red) as u8;
            let int_green: u8 = (255.999f32 * green) as u8;
            let int_blue: u8 = (255.999f32 * blue) as u8;
            buf.write_fmt(format_args!("{} {} {}", int_red, int_green, int_blue))
                .unwrap();
        }
    }
    file.write(&mut buf[..]).unwrap();
}

use crate::{interval::Interval, rtw_image::RTWImage, vec3::Vec3};

#[derive(Debug)]
pub enum Texture {
    Color(Vec3),
    Image(RTWImage),
    CheckerBoard {
        inv_scale: f64,
        a: Box<Texture>,
        b: Box<Texture>,
    },
}

impl Texture {
    pub fn value(&self, u: f64, v: f64, point: &Vec3) -> Vec3 {
        match self {
            Texture::Color(color) => return *color,
            Texture::Image(image) => {
                if image.image_height <= 0 {
                    return Vec3::new(0.0, 1.0, 1.0);
                }

                let u = Interval::new(0.0, 1.0).clamp(u);
                let v = 1.0 - Interval::new(0.0, 1.0).clamp(v);

                let i = (u * (image.image_width as f64)) as u32;
                let j =
                    (v * (image.image_height as f64)).min(image.image_height as f64 - 1.0) as u32;

                let color = image.pixel_data(i, j);

                let val = Vec3::new(
                    RTWImage::byte_to_float(color[0]) as f64,
                    RTWImage::byte_to_float(color[1]) as f64,
                    RTWImage::byte_to_float(color[2]) as f64,
                );
                val
            }

            Texture::CheckerBoard { inv_scale, a, b } => {
                let x_int = (point.x * inv_scale).floor() as i64;
                let y_int = (point.y * inv_scale).floor() as i64;
                let z_int = (point.z * inv_scale).floor() as i64;

                let is_even = (x_int + y_int + z_int) % 2 == 0;

                if is_even {
                    a.value(u, v, point)
                } else {
                    b.value(u, v, point)
                }
            }
        }
    }
}

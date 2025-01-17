use crate::rtw_image::RTWImage;
use glam::{Vec3A, Vec4};

#[derive(Debug, Clone)]
pub enum Texture {
    Color(Vec4),
    Image(RTWImage),
    CheckerBoard {
        inv_scale: f32,
        a: Box<Texture>,
        b: Box<Texture>,
    },
}

impl Texture {
    pub const DEFAULT_TEXTURE: Self = Texture::Color(Vec4::new(1.0, 0.0, 1.0, 1.0));

    pub fn value(&self, u: f32, v: f32, point: &Vec3A) -> Vec4 {
        match self {
            Texture::Color(color) => return *color,
            Texture::Image(image) => {
                if image.image_height <= 0 {
                    return Vec4::new(1.0, 1.0, 1.0, 1.0);
                }

                let u = u % 1.0;
                let u = if u < 0.0 { u + 1.0 } else { u };

                let v = v % 1.0;
                let v = if v < 0.0 { v + 1.0 } else { v };

                let v = 1.0 - v; // Flip v coordinate

                let i = (u * (image.image_width as f32)) as u32;
                let j =
                    (v * (image.image_height as f32)).min(image.image_height as f32 - 1.0) as u32;

                let color = image.pixel_data(i, j);

                let val = Vec4::new(
                    RTWImage::byte_to_float(color[0]) as f32,
                    RTWImage::byte_to_float(color[1]) as f32,
                    RTWImage::byte_to_float(color[2]) as f32,
                    RTWImage::byte_to_float(color[3]) as f32,
                );
                val
            }

            Texture::CheckerBoard { inv_scale, a, b } => {
                let x_int = (point.x as f32 * inv_scale).floor() as i64;
                let y_int = (point.y as f32 * inv_scale).floor() as i64;
                let z_int = (point.z as f32 * inv_scale).floor() as i64;

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

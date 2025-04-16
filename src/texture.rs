use std::{
    array::{self},
    rc::Rc,
};

use crate::rtw_image::RTWImage;
use glam::{Vec3A, Vec4};
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub enum Texture {
    Color(Vec4),
    Image(RTWImage),
    CheckerBoard {
        inv_scale: f32,
        a: Rc<Texture>,
        b: Rc<Texture>,
    },
}
lazy_static! {
    static ref lut_table: [f32; 256] = Texture::linear_lut();
}
impl Texture {
    pub const DEFAULT_TEXTURE: Self = Texture::Color(Vec4::new(1.0, 0.0, 1.0, 1.0));
    #[inline]
    fn linear_lut() -> [f32; 256] {
        let result: [f32; 256] = array::from_fn(|i| f32::powf(i as f32 / 255.0, 2.2));
        result
    }

    pub fn value(&self, u: f32, v: f32, point: &Vec3A) -> Vec4 {
        match self {
            Texture::Color(color) => {
                let mut color = *color;
                color.x = color.x.powf(1.0 / 2.2);
                color.y = color.y.powf(1.0 / 2.2);
                color.z = color.z.powf(1.0 / 2.2);
                color
            }
            Texture::Image(image) => {
                if image.image_height <= 0 {
                    return Vec4::new(1.0, 1.0, 1.0, 1.0);
                }

                let u = u.clamp(0.0, 1.0);
                let v = v.clamp(0.0, 1.0);

                let i = (u * image.image_width as f32) as u32;
                let j = (v * image.image_height as f32) as u32;

                let color = image.pixel_data(i, j);

                let mut val = Vec4::splat(0.0);

                val[0] = lut_table[(0xFF & color[0]) as usize];
                val[1] = lut_table[(0xFF & color[1]) as usize];
                val[2] = lut_table[(0xFF & color[2]) as usize];
                val[3] = (color[3] & 0xFF) as f32 / 255.0;

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

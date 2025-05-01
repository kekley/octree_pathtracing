use std::{
    array::{self},
    rc::Rc,
    sync::Arc,
};

use crate::rtw_image::RTWImage;
use glam::{Vec3A, Vec4};
use lazy_static::lazy_static;

use super::tile_renderer::U8Color;

#[derive(Debug, Clone)]
pub enum Texture {
    Color(U8Color),
    Image(RTWImage),
}
lazy_static! {
    pub static ref LUT_TABLE_FLOAT: [f32; 256] = Texture::linear_lut();
}
lazy_static! {
    pub static ref LUT_TABLE_BYTE: [u8; 256] = Texture::generate_gamma_lut();
}
impl Texture {
    pub const DEFAULT_TEXTURE: Self = Texture::Color(U8Color::new(255, 0, 255, 255));
    #[inline]
    fn linear_lut() -> [f32; 256] {
        let result: [f32; 256] = array::from_fn(|i| f32::powf(i as f32 / 255.0, 2.2));
        result
    }
    fn generate_gamma_lut() -> [u8; 256] {
        let mut lut = [0u8; 256];
        for i in 0..256 {
            lut[i] = (((i as f32 / 255.0).powf(1.0 / 2.2)) * 255.0) as u8;
        }
        lut
    }

    pub fn value(&self, u: f32, v: f32, point: &Vec3A) -> Vec4 {
        match self {
            Texture::Color(color) => {
                let mut color_vec = Vec4::ZERO;
                color_vec.w = color.a as f32 / 255.0;
                color_vec.x = LUT_TABLE_FLOAT[color.r as usize];
                color_vec.y = LUT_TABLE_FLOAT[color.g as usize];
                color_vec.z = LUT_TABLE_FLOAT[color.b as usize];
                color_vec
            }
            Texture::Image(image) => {
                if image.image_height <= 0 {
                    return Vec4::new(1.0, 1.0, 1.0, 1.0);
                }

                let u = u.clamp(0.0, 1.0);
                let v = 1.0 - v.clamp(0.0, 1.0);

                let i = (u * image.image_width as f32) as u32;
                let j = (v * image.image_height as f32) as u32;

                let color = image.pixel_data(i, j);

                let mut val = Vec4::splat(0.0);

                val[0] = LUT_TABLE_FLOAT[(color[0]) as usize];
                val[1] = LUT_TABLE_FLOAT[(color[1]) as usize];
                val[2] = LUT_TABLE_FLOAT[(color[2]) as usize];
                val[3] = color[3] as f32 / 255.0;

                val
            }
        }
    }
}

use std::hash::Hash;
use std::{
    array::{self},
    sync::Arc,
};

use glam::{Vec3A, Vec4};
use lazy_static::lazy_static;

use crate::colors::{F32Color, U8Color};

use super::rtw_image::RTWImage;

#[derive(Debug, Clone)]
pub enum Texture {
    Color(U8Color),
    Image(Arc<RTWImage>),
}

impl std::hash::Hash for Texture {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Texture::Color(u8_color) => {
                u8_color.hash(state);
            }
            Texture::Image(rtwimage) => Arc::as_ptr(rtwimage).hash(state),
        }
    }
}

impl Eq for Texture {}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Color(l0), Self::Color(r0)) => l0 == r0,
            (Self::Image(l0), Self::Image(r0)) => Arc::ptr_eq(l0, r0),
            _ => false,
        }
    }
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
        let tmp = 0..256;
        for i in tmp {
            lut[i] = (((i as f32 / 255.0).powf(1.0 / 2.2)) * 255.0) as u8;
        }
        lut
    }

    pub fn value(&self, u: f32, v: f32, point: &Vec3A) -> Vec4 {
        match self {
            Texture::Color(color) => {
                let color = F32Color::from(color);
                Vec4::from_array(color.into_array())
            }
            Texture::Image(image) => {
                if image.image_height == 0 {
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

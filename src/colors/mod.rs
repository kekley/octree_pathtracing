use std::{mem::transmute, ops::Mul};

use crate::textures::texture::{LUT_TABLE_BYTE, LUT_TABLE_FLOAT};

pub trait PixelColor<T> {
    fn r(&self) -> T;
    fn g(&self) -> T;
    fn b(&self) -> T;
    fn a(&self) -> T;
    fn r_mut(&mut self) -> &mut T;
    fn g_mut(&mut self) -> &mut T;
    fn b_mut(&mut self) -> &mut T;
    fn a_mut(&mut self) -> &mut T;
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct F32Color {
    data: [f32; 4],
}

impl PixelColor<f32> for F32Color {
    fn r(&self) -> f32 {
        self.data[0]
    }
    fn g(&self) -> f32 {
        self.data[1]
    }
    fn b(&self) -> f32 {
        self.data[2]
    }
    fn a(&self) -> f32 {
        self.data[3]
    }
    fn r_mut(&mut self) -> &mut f32 {
        &mut self.data[0]
    }
    fn g_mut(&mut self) -> &mut f32 {
        &mut self.data[1]
    }
    fn b_mut(&mut self) -> &mut f32 {
        &mut self.data[2]
    }
    fn a_mut(&mut self) -> &mut f32 {
        &mut self.data[3]
    }
}

impl F32Color {
    pub const BLACK: F32Color = F32Color {
        data: [0.0, 0.0, 0.0, 1.0],
    };
}

impl From<&U8Color> for F32Color {
    fn from(value: &U8Color) -> Self {
        let mut data = [0f32; 4];
        data[3] = value.a() as f32 / 255.0;
        data[0] = LUT_TABLE_FLOAT[value.r() as usize];
        data[1] = LUT_TABLE_FLOAT[value.g() as usize];
        data[2] = LUT_TABLE_FLOAT[value.b() as usize];
        Self { data }
    }
}
impl From<U8Color> for F32Color {
    fn from(value: U8Color) -> Self {
        todo!()
    }
}

impl Mul<f32> for F32Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> F32Color {
        let mut data = self.data;
        data.iter_mut().for_each(|val| *val *= rhs);
        F32Color { data }
    }
}
impl Mul<f32> for &F32Color {
    type Output = F32Color;

    fn mul(self, rhs: f32) -> F32Color {
        let mut data = self.data;
        data.iter_mut().for_each(|val| *val *= rhs);
        F32Color { data }
    }
}

impl F32Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { data: [r, g, b, a] }
    }
    pub fn from_array(data: [f32; 4]) -> Self {
        Self { data }
    }
    pub fn splat(value: f32) -> Self {
        Self { data: [value; 4] }
    }
    pub fn into_array(self) -> [f32; 4] {
        self.data
    }
    pub fn min_element(&self) -> f32 {
        *self
            .data
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(&self.data[0])
    }
    pub fn max_element(&self) -> f32 {
        *self
            .data
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(&self.data[0])
    }
    //Returns a color containing each minumum component of the two inputs
    pub fn min_color(&self, other: &F32Color) -> F32Color {
        let mut data: [f32; 4] = [0f32; 4];
        data.iter_mut()
            .zip(self.data.iter().zip(other.data))
            .for_each(|(out, (a, b))| *out = a.min(b));
        F32Color { data }
    }
    //Returns a color containing each minumum component of the two inputs
    pub fn max_color(&self, other: &F32Color) -> F32Color {
        let mut data: [f32; 4] = [0f32; 4];
        data.iter_mut()
            .zip(self.data.iter().zip(other.data))
            .for_each(|(out, (a, b))| *out = a.max(b));
        F32Color { data }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct U8Color {
    data: [u8; 4],
}

impl U8Color {
    pub const BLACK: U8Color = U8Color {
        data: [0, 0, 0, 255],
    };
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { data: [r, g, b, a] }
    }
}

impl PixelColor<u8> for U8Color {
    fn r(&self) -> u8 {
        self.data[0]
    }
    fn g(&self) -> u8 {
        self.data[1]
    }
    fn b(&self) -> u8 {
        self.data[2]
    }
    fn a(&self) -> u8 {
        self.data[3]
    }
    fn r_mut(&mut self) -> &mut u8 {
        &mut self.data[0]
    }
    fn g_mut(&mut self) -> &mut u8 {
        &mut self.data[1]
    }
    fn b_mut(&mut self) -> &mut u8 {
        &mut self.data[2]
    }
    fn a_mut(&mut self) -> &mut u8 {
        &mut self.data[3]
    }
}

impl From<U8Color> for [u8; 4] {
    fn from(val: U8Color) -> Self {
        unsafe { transmute::<U8Color, [u8; 4]>(val) }
    }
}

impl From<&F32Color> for U8Color {
    fn from(value: &F32Color) -> Self {
        let res: [f32; 4] = (value * 255.0)
            .min_color(&F32Color::splat(255.0))
            .into_array();
        let r = LUT_TABLE_BYTE[res[0] as usize];
        let g = LUT_TABLE_BYTE[res[1] as usize];
        let b = LUT_TABLE_BYTE[res[2] as usize];
        U8Color {
            data: [r, g, b, res[3] as u8],
        }
    }
}
impl From<F32Color> for U8Color {
    fn from(value: F32Color) -> Self {
        let res: [f32; 4] = (value * 255.0)
            .min_color(&F32Color::splat(255.0))
            .into_array();
        let r = LUT_TABLE_BYTE[res[0] as usize];
        let g = LUT_TABLE_BYTE[res[1] as usize];
        let b = LUT_TABLE_BYTE[res[2] as usize];
        U8Color {
            data: [r, g, b, res[3] as u8],
        }
    }
}

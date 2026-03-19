use std::fmt::Debug;

use image::GenericImageView;

#[derive(Clone)]
pub struct RgbaImage {
    pub raw_data: Box<[u8]>,
    pub image_width: u32,
    pub stride: u32,
    pub image_height: u32,
}

impl Debug for RgbaImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RgbaImage")
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .finish()
    }
}
impl RgbaImage {
    const BYTES_PER_PIXEL: u32 = 4;
    pub fn load_from_memory(data: &[u8]) -> Result<Self, String> {
        match image::load_from_memory(data) {
            Ok(image) => {
                let width = image.width();

                let height = image.width();
                let data = image.into_rgba8();

                Ok(Self {
                    raw_data: data.to_vec().into_boxed_slice(),
                    image_width: width,
                    image_height: height,
                    stride: width * Self::BYTES_PER_PIXEL,
                })
            }
            Err(err) => Err(err.to_string()),
        }
    }
    pub fn load(file_path: &str) -> Result<Self, String> {
        //println!("{}", file_path);
        let Ok(file) = std::fs::read(file_path) else {
            return Err(format!("failed to load: {file_path}"));
        };
        match image::load_from_memory(&file) {
            Ok(image) => {
                let width = image.width();

                let height = image.width();
                let data = image.into_rgba8();

                Ok(Self {
                    raw_data: data.to_vec().into_boxed_slice(),
                    image_width: width,
                    image_height: height,
                    stride: width * Self::BYTES_PER_PIXEL,
                })
            }
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn float_to_byte(value: f32) -> u8 {
        if value <= 0.0 {
            return 0;
        }
        if value >= 1.0 {
            return 255;
        }
        (256.0 * value) as u8
    }

    #[inline]
    pub fn byte_to_float(value: u8) -> f32 {
        value as f32 / 255.0
    }

    pub fn convert_to_bytes(floats: &[f32]) -> Vec<u8> {
        let total_bytes = floats.len();
        let mut bytes = Vec::with_capacity(total_bytes);
        floats.iter().for_each(|f| {
            bytes.push(Self::float_to_byte(*f));
        });
        bytes
    }
    pub fn convert_to_floats(bytes: &[u8]) -> Vec<f32> {
        let total_bytes = bytes.len();
        let mut floats = Vec::with_capacity(total_bytes);
        bytes.iter().for_each(|f| {
            floats.push(Self::byte_to_float(*f));
        });
        floats
    }

    pub fn pixel_data(&self, mut x: u32, mut y: u32) -> [u8; 4] {
        x = x.clamp(0, self.image_width - 1);
        y = y.clamp(0, self.image_height - 1);
        let index = (x * 4 + (y * self.stride)) as usize;

        let mut ret_val: [u8; 4] = [255, 0, 255, 255];
        let r = *self.raw_data.get(index).unwrap();
        let g = *self.raw_data.get(index + 1).unwrap();
        let b = *self.raw_data.get(index + 2).unwrap();
        let a = *self.raw_data.get(index + 3).unwrap();
        ret_val = [r, g, b, a];

        ret_val
    }
}

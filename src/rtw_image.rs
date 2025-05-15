use std::fmt::Debug;

use stb_image::image::load;

#[derive(Debug, Clone, Copy, Default)]
pub enum BytesPerPixel {
    #[default]
    INVALID = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

impl TryFrom<u32> for BytesPerPixel {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(BytesPerPixel::One),
            2 => Ok(BytesPerPixel::Two),
            3 => Ok(BytesPerPixel::Three),
            4 => Ok(BytesPerPixel::Four),
            _ => Err(format!("Invalid bytes per pixel: {}", value)),
        }
    }
}

#[derive(Clone)]
pub struct RTWImage {
    bytes_per_pixel: BytesPerPixel,
    bdata: Box<[u8]>,
    pub image_width: u32,
    pub image_height: u32,
    bytes_per_scanline: u32,
}

impl Debug for RTWImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTWImage")
            .field("bytes_per_pixel", &self.bytes_per_pixel)
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .field("bytes_per_scanline", &self.bytes_per_scanline)
            .finish()
    }
}
impl RTWImage {
    pub fn load(file_path: &str) -> Result<Self, String> {
        //println!("{}", file_path);
        let load_result = load(file_path);
        match load_result {
            stb_image::image::LoadResult::Error(e) => {
                //println!("{}", file_path);
                return Err(file_path.to_string());
            }
            stb_image::image::LoadResult::ImageU8(image) => {
                let bdata = image.data;
                let image_width = image.width as u32;
                let image_height = image.height as u32;
                let bytes_per_pixel = BytesPerPixel::try_from(image.depth as u32)?;
                let bytes_per_scanline = image_width * image.depth as u32;
                let fdata = Self::convert_to_floats(&bdata);
                let bdata: Box<[u8]> = Box::from(bdata);
                return Ok(RTWImage {
                    bytes_per_pixel,
                    bdata,
                    image_width,
                    image_height,
                    bytes_per_scanline,
                });
            }
            stb_image::image::LoadResult::ImageF32(image) => {
                let fdata = image.data;
                let image_width = image.width as u32;
                let image_height = image.height as u32;
                let bytes_per_pixel = BytesPerPixel::try_from(image.depth as u32)?;
                let bytes_per_scanline = image_width * image.depth as u32;
                let bdata = Self::convert_to_bytes(&fdata);
                let bdata: Box<[u8]> = Box::from(bdata);
                return Ok(RTWImage {
                    bytes_per_pixel,
                    bdata,
                    image_width,
                    image_height,
                    bytes_per_scanline,
                });
            }
        }
    }

    pub fn float_to_byte(value: f32) -> u8 {
        if value <= 0.0 {
            return 0;
        }
        if value >= 1.0 {
            return 255;
        }
        return (256.0 * value) as u8;
    }

    #[inline]
    pub fn byte_to_float(value: u8) -> f32 {
        value as f32 / 255.0
    }

    pub fn convert_to_bytes(floats: &Vec<f32>) -> Vec<u8> {
        let total_bytes = floats.len();
        let mut bytes = Vec::with_capacity(total_bytes);
        floats.iter().for_each(|f| {
            bytes.push(Self::float_to_byte(*f));
        });
        bytes
    }
    pub fn convert_to_floats(bytes: &Vec<u8>) -> Vec<f32> {
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
        let index: usize = (y * self.bytes_per_scanline + x * self.bytes_per_pixel as u32) as usize;

        let mut ret_val: [u8; 4] = [255, 0, 255, 255];
        match self.bytes_per_pixel {
            BytesPerPixel::One => {
                let col = *self.bdata.get(index).unwrap();
                ret_val = [col, col, col, 255];
            }
            BytesPerPixel::Two => {
                let col = [
                    *self.bdata.get(index).unwrap(),
                    *self.bdata.get(index + 1).unwrap(),
                ];
                let gray = (u16::from_be_bytes(col) >> 8) as u8;
                ret_val = [gray, gray, gray, 255]
            }
            BytesPerPixel::Three => {
                let r = *self.bdata.get(index).unwrap();
                let g = *self.bdata.get(index + 1).unwrap();
                let b = *self.bdata.get(index + 2).unwrap();
                ret_val = [r, g, b, 255];
            }
            BytesPerPixel::Four => {
                let r = *self.bdata.get(index).unwrap();
                let g = *self.bdata.get(index + 1).unwrap();
                let b = *self.bdata.get(index + 2).unwrap();
                let a = *self.bdata.get(index + 3).unwrap();
                ret_val = [r, g, b, a];
            }
            BytesPerPixel::INVALID => {}
        }
        ret_val
    }
}

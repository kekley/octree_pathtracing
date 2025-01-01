use std::default;

use stb_image::image::load;

#[derive(Debug, Clone, Copy, Default)]
pub enum BytesPerPixel {
    #[default]
    INVALID = 0,
    One = 1,
    Three = 3,
    Four = 4,
}

impl TryFrom<u32> for BytesPerPixel {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(BytesPerPixel::One),
            3 => Ok(BytesPerPixel::Three),
            4 => Ok(BytesPerPixel::Four),
            _ => Err(format!("Invalid bytes per pixel: {}", value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RTWImage {
    bytes_per_pixel: BytesPerPixel,
    fdata: Vec<f32>,
    bdata: Vec<u8>,
    pub image_width: u32,
    pub image_height: u32,
    bytes_per_scanline: u32,
}

impl RTWImage {
    pub fn load(file_path: &str) -> Result<Self, String> {
        let mut tmp = Self {
            bytes_per_pixel: BytesPerPixel::default(),
            fdata: vec![],
            bdata: vec![],
            image_width: 0,
            image_height: 0,
            bytes_per_scanline: 0,
        };
        let load_result = load(file_path);
        match load_result {
            stb_image::image::LoadResult::Error(e) => {
                //println!("{}", file_path);
                return Err(file_path.to_string());
            }
            stb_image::image::LoadResult::ImageU8(image) => {
                tmp.bdata = image.data;
                tmp.image_width = image.width as u32;
                tmp.image_height = image.height as u32;
                tmp.bytes_per_pixel = BytesPerPixel::try_from(image.depth as u32)?;
                tmp.bytes_per_scanline = tmp.image_width * image.depth as u32;
                tmp.convert_to_floats();
            }
            stb_image::image::LoadResult::ImageF32(image) => {
                tmp.fdata = image.data;
                tmp.image_width = image.width as u32;
                tmp.image_height = image.height as u32;
                tmp.bytes_per_pixel = BytesPerPixel::try_from(image.depth as u32)?;
                tmp.bytes_per_scanline = tmp.image_width * image.depth as u32;
                tmp.convert_to_bytes();
            }
        }
        Ok(tmp)
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

    pub fn convert_to_bytes(&mut self) {
        self.bdata.clear();

        let total_bytes = self.image_height * self.image_width * self.bytes_per_pixel as u32;
        self.bdata.reserve(total_bytes.try_into().unwrap());
        self.fdata.iter().for_each(|f| {
            self.bdata.push(Self::float_to_byte(*f));
        });
    }
    pub fn convert_to_floats(&mut self) {
        self.fdata.clear();

        let total_bytes = self.image_height * self.image_width * self.bytes_per_pixel as u32;
        self.fdata.reserve(total_bytes.try_into().unwrap());
        self.bdata.iter().for_each(|f| {
            self.fdata.push(Self::byte_to_float(*f));
        });
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
            _ => {}
        }
        ret_val
    }
}

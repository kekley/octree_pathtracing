use stb_image::image::load;
#[derive(Debug, Clone)]
pub struct RTWImage {
    bytes_per_pixel: u32,
    fdata: Vec<f32>,
    bdata: Vec<u8>,
    pub image_width: u32,
    pub image_height: u32,
    bytes_per_scanline: u32,
}

impl RTWImage {
    pub fn load(file_path: &str) -> Self {
        let mut tmp = Self {
            bytes_per_pixel: 0,
            fdata: vec![],
            bdata: vec![],
            image_width: 0,
            image_height: 0,
            bytes_per_scanline: 0,
        };
        let load_result = load(file_path);
        match load_result {
            stb_image::image::LoadResult::Error(e) => panic!("Error: {}, path: {}", e, file_path),
            stb_image::image::LoadResult::ImageU8(image) => {
                tmp.bdata = image.data;
                tmp.image_width = image.width as u32;
                tmp.image_height = image.height as u32;
                tmp.bytes_per_pixel = image.depth as u32;
                tmp.bytes_per_scanline = tmp.image_width * tmp.bytes_per_pixel;
                tmp.convert_to_floats();
            }
            stb_image::image::LoadResult::ImageF32(image) => {
                tmp.fdata = image.data;
                tmp.image_width = image.width as u32;
                tmp.image_height = image.height as u32;
                tmp.bytes_per_pixel = image.depth as u32;
                tmp.bytes_per_scanline = tmp.image_width * tmp.bytes_per_pixel;
                tmp.convert_to_bytes();
            }
        }
        tmp
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

    pub fn byte_to_float(value: u8) -> f32 {
        value as f32 / 255.0
    }

    pub fn convert_to_bytes(&mut self) {
        self.bdata.clear();

        let total_bytes = self.image_height * self.image_width * self.bytes_per_pixel;
        self.bdata.reserve(total_bytes.try_into().unwrap());
        self.fdata.iter().for_each(|f| {
            self.bdata.push(Self::float_to_byte(*f));
        });
    }
    pub fn convert_to_floats(&mut self) {
        self.fdata.clear();

        let total_bytes = self.image_height * self.image_width * self.bytes_per_pixel;
        self.fdata.reserve(total_bytes.try_into().unwrap());
        self.bdata.iter().for_each(|f| {
            self.fdata.push(Self::byte_to_float(*f));
        });
    }

    pub fn pixel_data(&self, mut x: u32, mut y: u32) -> [u8; 3] {
        x = x.clamp(0, self.image_width - 1);
        y = y.clamp(0, self.image_height - 1);

        let index = (y * self.bytes_per_scanline + x * self.bytes_per_pixel) as usize;
        let mut ret_val: [u8; 3] = [255, 0, 255];
        match self.bytes_per_pixel {
            1 => {
                let col = *self.bdata.get(index).unwrap();
                ret_val = [col, col, col];
            }
            3 => ret_val = self.bdata[index..index + 3].try_into().unwrap(),
            _ => {}
        }
        ret_val
    }
}

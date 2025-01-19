use std::sync::Mutex;
use std::{fs::File, io::Write, sync::Arc, thread, vec};

use rand::rngs::StdRng;

use glam::Vec2;
use image::{
    EncodableLayout, ExtendedColorType, GenericImage, ImageBuffer, Rgb, Rgb32FImage, RgbImage,
};
use rand::{Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{linear_to_gamma, random_float_in_range, sample_square, util, Scene};
pub struct Tile {
    bytes_per_pixel: usize,
    stride: usize,
    frame_buffer_stride: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    local_buffer: Vec<f32>,
    frame_buffer_resolution: (usize, usize),
    dim: f32,
    frame_buffer: Arc<Mutex<Vec<f32>>>,
}

pub struct TileRenderer {
    pub thread_count: usize,
    pub resolution: (usize, usize),
    bytes_per_pixel: usize,
    frame_buffer: Arc<Mutex<Vec<f32>>>,
    scene: Arc<Scene>,
}

impl Tile {
    pub fn new(
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        bytes_per_pixel: usize,
        frame_buffer: Arc<Mutex<Vec<f32>>>,
        frame_buffer_resolution: (usize, usize),
    ) -> Self {
        let x1 = x1.min(frame_buffer_resolution.0);
        let y1 = y1.min(frame_buffer_resolution.1);
        let dim = frame_buffer_resolution.0.max(frame_buffer_resolution.1) as f32;
        Self {
            bytes_per_pixel: bytes_per_pixel,
            stride: (x1 - x0),
            frame_buffer_stride: frame_buffer_resolution.0,
            x0,
            y0,
            x1,
            y1,
            local_buffer: vec![0.0; (y1 - y0) * (x1 - x0) * bytes_per_pixel],
            frame_buffer: frame_buffer,
            frame_buffer_resolution,
            dim,
        }
    }
}

impl TileRenderer {
    pub fn new(
        render_resolution: (usize, usize),
        bytes_per_pixel: usize,
        threads: usize,
        scene: Scene,
    ) -> Self {
        Self {
            thread_count: threads,
            resolution: render_resolution,
            frame_buffer: Arc::new(Mutex::new(vec![
                0.0;
                render_resolution.0
                    * render_resolution.1
                    * bytes_per_pixel
            ])),
            scene: Arc::new(scene),
            bytes_per_pixel: bytes_per_pixel,
        }
    }

    pub fn render(&mut self, file_name: &str) {
        let tile_width = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
        let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;

        (0..self.thread_count).into_iter().for_each(|y| {
            (0..self.thread_count).into_iter().for_each(|x| {
                let tile = Tile::new(
                    x * tile_width,
                    y * tile_height,
                    (x + 1) * tile_width,
                    (y + 1) * tile_height,
                    self.bytes_per_pixel,
                    self.frame_buffer.clone(),
                    self.resolution,
                );
                let scene = self.scene.clone();
                TileRenderer::render_tile(tile, scene);
            })
        });

        let u8_frame_buffer = self
            .frame_buffer
            .lock()
            .unwrap()
            .iter()
            .map(|&channel| {
                let corrected = channel.powf(1.0 / 2.2);
                (corrected * 255.0) as u8
            })
            .collect::<Vec<u8>>();

        let image = RgbImage::from_vec(
            self.resolution.0 as u32,
            self.resolution.1 as u32,
            u8_frame_buffer,
        )
        .unwrap();

        image.save(file_name).unwrap();

        /*         let mut ppm_buffer: Vec<u8> =
                   Vec::with_capacity(self.resolution.0 * self.resolution.1 * self.bytes_per_pixel * 3);

               ppm_buffer.write(header.as_bytes()).unwrap();
               let frame_buffer = self.frame_buffer.lock().unwrap().to_owned();
               for (i, byte) in frame_buffer.iter().enumerate() {
                   let byte_as_str = byte.to_string();
                   ppm_buffer.write(byte_as_str.as_bytes()).unwrap();
                   if (i + 1) % (self.bytes_per_pixel * 3) == 0 {
                       ppm_buffer.write("\n".as_bytes()).unwrap();
                   } else {
                       ppm_buffer.write(" ".as_bytes()).unwrap();
                   }
               }
        */
        //let mut file_stream = File::create(file_name).unwrap();

        //file_stream.write_all(&self.frame_buffer).unwrap();
    }

    pub fn render_tile(mut tile: Tile, scene: Arc<Scene>) {
        let mut rng = StdRng::from_entropy();

        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let x_normalized =
                    ((2 * x + 1) as f32 - tile.frame_buffer_resolution.0 as f32) / tile.dim;
                let y_normalized = ((2 * (tile.frame_buffer_resolution.1 - y) - 1) as f32
                    - tile.frame_buffer_resolution.1 as f32)
                    / tile.dim;
                let mut current_spp = 0;
                while current_spp < scene.target_spp {
                    let branch_count = scene.get_current_branch_count(current_spp);
                    let sin_v = 1.0 / (branch_count + current_spp) as f32;

                    let dx = rng.gen_range((-1.0 / tile.dim)..(1.0 / tile.dim));
                    let dy = rng.gen_range((-1.0 / tile.dim)..(1.0 / tile.dim));
                    let color = scene.get_color(
                        x_normalized + dx,
                        y_normalized + dy,
                        &mut rng,
                        current_spp,
                    );
                    let local_buffer_idx = Self::get_pixel_index(
                        x - tile.x0,
                        y - tile.y0,
                        tile.bytes_per_pixel,
                        tile.stride,
                    );
                    let r = color.x * branch_count as f32;
                    let g = color.y * branch_count as f32;
                    let b = color.z * branch_count as f32;
                    tile.local_buffer[local_buffer_idx] =
                        (tile.local_buffer[local_buffer_idx] * current_spp as f32 + r) * sin_v;
                    tile.local_buffer[local_buffer_idx + 1] =
                        (tile.local_buffer[local_buffer_idx + 1] * current_spp as f32 + g) * sin_v;
                    tile.local_buffer[local_buffer_idx + 2] =
                        (tile.local_buffer[local_buffer_idx + 2] * current_spp as f32 + b) * sin_v;
                    current_spp += branch_count;
                }
            }
        }

        let mut frame_buffer = tile.frame_buffer.lock().unwrap();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let frame_buffer_idx =
                    Self::get_pixel_index(x, y, tile.bytes_per_pixel, tile.frame_buffer_stride);
                let local_buffer_idx = Self::get_pixel_index(
                    x - tile.x0,
                    y - tile.y0,
                    tile.bytes_per_pixel,
                    tile.stride,
                );
                let r = tile.local_buffer[local_buffer_idx];
                let g = tile.local_buffer[local_buffer_idx + 1];
                let b = tile.local_buffer[local_buffer_idx + 2];

                frame_buffer[frame_buffer_idx] = r;
                frame_buffer[frame_buffer_idx + 1] = g;
                frame_buffer[frame_buffer_idx + 2] = b;
            }
        }
    }
    #[inline]
    fn get_pixel_index(x: usize, y: usize, bytes_per_pixel: usize, stride: usize) -> usize {
        let val = ((y * stride) + x) * bytes_per_pixel;
        //println!("stride:{}", stride);
        //println!("x:{}, y:{}", x, y);
        //println!("val: {}", val);
        val
    }

    fn normalize_coordinates(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
        let dim = width.max(height);
        let xn = ((2.0 * x + 1.0) - width) / dim;
        let yn = ((2.0 * (height - y) - 1.0) - height) / dim;

        (xn, yn)
    }
}

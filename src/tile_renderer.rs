use std::sync::Mutex;
use std::{
    fs::File,
    io::{Write},
    sync::Arc,
    thread, vec,
};

use rand::{rngs::StdRng, SeedableRng};

use crate::{util, Scene};
pub struct Tile {
    bytes_per_pixel: usize,
    stride: usize,
    frame_buffer_stride: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    local_buffer: Vec<u8>,
    frame_buffer_resolution: (usize, usize),
    frame_buffer: Arc<Mutex<Vec<u8>>>,
}

pub struct TileRenderer {
    pub thread_count: usize,
    pub resolution: (usize, usize),
    bytes_per_pixel: usize,
    stride: usize,
    frame_buffer: Arc<Mutex<Vec<u8>>>,
    scene: Arc<Scene>,
}

impl Tile {
    pub fn new(
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        bytes_per_pixel: usize,
        frame_buffer: Arc<Mutex<Vec<u8>>>,
        frame_buffer_resolution: (usize, usize),
    ) -> Self {
        let x1 = x1.min(frame_buffer_resolution.0);
        let y1 = y1.min(frame_buffer_resolution.1);
        Self {
            bytes_per_pixel: bytes_per_pixel,
            stride: (x1 - x0),
            frame_buffer_stride: frame_buffer_resolution.0,
            x0,
            y0,
            x1,
            y1,
            local_buffer: Vec::with_capacity((y1 - y0) * (x1 - x0) * bytes_per_pixel),
            frame_buffer: frame_buffer,
            frame_buffer_resolution,
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
                0u8;
                render_resolution.0
                    * render_resolution.1
                    * bytes_per_pixel
            ])),
            scene: Arc::new(scene),
            bytes_per_pixel: bytes_per_pixel,
            stride: render_resolution.0 * bytes_per_pixel,
        }
    }

    pub fn render(&self) {
        let mut threads = vec![];
        let tile_width = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
        let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;
        let header = format!("P3\n{} {}\n255\n", self.resolution.0, self.resolution.1);

        for y in 0..self.thread_count {
            for x in 0..self.thread_count {
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
                threads.push(thread::spawn(move || {
                    TileRenderer::render_tile(tile, &scene);
                }));
            }
        }

        for t in threads {
            t.join().unwrap();
        }

        let mut ppm_buffer: Vec<u8> =
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

        let mut file_stream = File::create("render.ppm").unwrap();

        file_stream.write_all(&ppm_buffer).unwrap();
    }

    pub fn render_tile(mut tile: Tile, scene: &Scene) {
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let (norm_x, norm_y) = TileRenderer::normalize_coordinates(
                    x,
                    y,
                    tile.frame_buffer_resolution.0,
                    tile.frame_buffer_resolution.1,
                );
                let color = scene.get_pixel_color(norm_x, norm_y, &mut rng);
                util::write_rgb8_color_to_stream(&color, &mut tile.local_buffer);
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
                frame_buffer[frame_buffer_idx] = tile.local_buffer[local_buffer_idx];
                frame_buffer[frame_buffer_idx + 1] = tile.local_buffer[local_buffer_idx + 1];
                frame_buffer[frame_buffer_idx + 2] = tile.local_buffer[local_buffer_idx + 2];
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

    fn normalize_coordinates(x: usize, y: usize, width: usize, height: usize) -> (f32, f32) {
        let dim = width.max(height) as f32;
        let xn = ((2 * x + 1) as f32 - width as f32) / dim;
        let yn = ((2 * (height - y) - 1) as f32 - height as f32) / dim;

        (xn, yn)
    }
}

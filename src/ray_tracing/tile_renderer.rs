use std::mem::transmute;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

use glam::Vec3;
use rand::rngs::StdRng;

use image::{Pixel, RgbImage, Rgba32FImage, RgbaImage};
use rand::{Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::scene::Scene;

pub struct Tile {
    stride: usize,
    frame_buffer_stride: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    local_buffer: Vec<F32Pixel>,
    frame_buffer_resolution: (usize, usize),
    dim: f32,
    frame_buffer: Arc<Mutex<Vec<F32Pixel>>>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct F32Pixel {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}
impl Into<[f32; 4]> for &F32Pixel {
    fn into(self) -> [f32; 4] {
        let a: [f32; 4] = [self.r, self.g, self.b, self.a];
        a
    }
}

impl F32Pixel {
    pub const BLACK: F32Pixel = F32Pixel {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
}

#[repr(C)]
struct U8Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Into<[u8; 4]> for U8Pixel {
    fn into(self) -> [u8; 4] {
        unsafe { transmute::<U8Pixel, [u8; 4]>(self) }
    }
}

impl From<&F32Pixel> for U8Pixel {
    fn from(value: &F32Pixel) -> Self {
        let r = (value.r.powf(1.0 / 2.2) * 255.0) as u8;
        let g = (value.g.powf(1.0 / 2.2) * 255.0) as u8;
        let b = (value.b.powf(1.0 / 2.2) * 255.0) as u8;
        let a = (value.a * 255.0) as u8;
        U8Pixel { r, g, b, a }
    }
}
pub struct TileRenderer {
    pub worker_thread: Option<JoinHandle<()>>,
    pub current_spp: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    pub thread_count: usize,
    pub resolution: (usize, usize),
    frame_buffer: Arc<Mutex<Vec<F32Pixel>>>,
    pub scene: Arc<Scene>,
}

impl Default for TileRenderer {
    fn default() -> Self {
        Self {
            current_spp: Default::default(),
            paused: Arc::new(AtomicBool::new(true)),
            thread_count: Default::default(),
            resolution: Default::default(),
            frame_buffer: Default::default(),
            scene: Default::default(),
            stopped: Arc::new(AtomicBool::new(true)),
            worker_thread: None,
        }
    }
}

impl Tile {
    pub fn new(
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        frame_buffer: Arc<Mutex<Vec<F32Pixel>>>,
        frame_buffer_resolution: (usize, usize),
    ) -> Self {
        let x1 = x1.min(frame_buffer_resolution.0);
        let y1 = y1.min(frame_buffer_resolution.1);
        let dim = frame_buffer_resolution.0.max(frame_buffer_resolution.1) as f32;
        Self {
            stride: (x1 - x0),
            frame_buffer_stride: frame_buffer_resolution.0,
            x0,
            y0,
            x1,
            y1,
            local_buffer: vec![F32Pixel::BLACK; (y1 - y0) * (x1 - x0)],
            frame_buffer: frame_buffer,
            frame_buffer_resolution,
            dim,
        }
    }
}

impl TileRenderer {
    pub fn new(render_resolution: (usize, usize), threads: usize, scene: Scene) -> Self {
        Self {
            thread_count: threads,
            resolution: render_resolution,
            frame_buffer: Arc::new(Mutex::new(
                (0..render_resolution.0 * render_resolution.1)
                    .into_iter()
                    .map(|_| F32Pixel::BLACK)
                    .collect(),
            )),
            scene: Arc::new(scene),
            paused: Arc::new(AtomicBool::new(true)),
            stopped: Arc::new(AtomicBool::new(true)),
            current_spp: Arc::new(AtomicU32::new(0)),
            worker_thread: None,
        }
    }
    pub fn get_frame_buffer_data(&self) -> Vec<u8> {
        let float_data = self.frame_buffer.lock().unwrap().clone();
        //save image
        let u8_data = float_data
            .iter()
            .map(|pixel| {
                let bytes: [u8; 4] = U8Pixel::from(pixel).into();
                bytes
            })
            .flat_map(|bytes| bytes.into_iter())
            .collect::<Vec<_>>();
        u8_data
    }
    pub fn current_spp(&self) -> u32 {
        self.current_spp.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn spp_add(&self, amount: u32) {
        self.current_spp
            .fetch_add(amount, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn is_idle(&self) -> bool {
        self.worker_thread.is_none()
            || self.stopped.load(std::sync::atomic::Ordering::SeqCst)
                && self.worker_thread.is_some()
    }
    pub fn send_pause_signal(&self) {
        self.paused.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn resume(&self) {
        if let Some(worker) = self.worker_thread.as_ref() {
            self.paused
                .store(false, std::sync::atomic::Ordering::SeqCst);
            self.stopped
                .store(false, std::sync::atomic::Ordering::SeqCst);
            worker.thread().unpark();
        }
    }

    pub fn collect_samples(&mut self) {
        let spp_clone = self.current_spp.clone();
        let frame_buffer = self.frame_buffer.clone();
        let scene = self.scene.clone();
        let branch_count = self.scene.get_current_branch_count(self.current_spp());
        let thread_count = self.thread_count;
        let resolution = self.resolution;
        let pause_bool = self.paused.clone();
        let stop_bool = self.stopped.clone();
        pause_bool.store(false, std::sync::atomic::Ordering::SeqCst);
        stop_bool.store(false, std::sync::atomic::Ordering::SeqCst);
        let f = move || loop {
            if pause_bool.load(std::sync::atomic::Ordering::SeqCst) {
                stop_bool.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::park();
            }
            let tile_width = (resolution.0 + thread_count - 1) / thread_count;
            let tile_height = (resolution.1 + thread_count - 1) / thread_count;

            let current_spp = spp_clone.load(std::sync::atomic::Ordering::SeqCst);
            (0..thread_count).into_iter().for_each(|y| {
                (0..thread_count).into_iter().for_each(|x| {
                    let tile = Tile::new(
                        x * tile_width,
                        y * tile_height,
                        (x + 1) * tile_width,
                        (y + 1) * tile_height,
                        frame_buffer.clone(),
                        resolution,
                    );
                    let scene = scene.clone();
                    TileRenderer::render_tile(tile, scene, current_spp, branch_count);
                    spp_clone.fetch_add(branch_count, std::sync::atomic::Ordering::SeqCst);
                })
            });
        };
        self.worker_thread = Some(spawn(f));
    }

    pub fn collect_sample(&self) {
        let tile_width = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
        let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;

        let current_spp = self.current_spp();
        let branch_count = self.scene.get_current_branch_count(current_spp);
        (0..self.thread_count).into_iter().for_each(|y| {
            (0..self.thread_count).into_iter().for_each(|x| {
                let tile = Tile::new(
                    x * tile_width,
                    y * tile_height,
                    (x + 1) * tile_width,
                    (y + 1) * tile_height,
                    self.frame_buffer.clone(),
                    self.resolution,
                );
                let scene = self.scene.clone();

                TileRenderer::render_tile(tile, scene, current_spp, branch_count);
                self.spp_add(branch_count);
                dbg!(current_spp);
            })
        });
    }

    pub fn render_to_image(&mut self, file_name: &str) {
        let tile_width = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
        let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;
        let mut current_spp = self.current_spp.load(std::sync::atomic::Ordering::SeqCst);
        while current_spp < self.scene.target_spp {
            let branch_count = self.scene.get_current_branch_count(current_spp);
            (0..self.thread_count).into_iter().for_each(|y| {
                (0..self.thread_count).into_iter().for_each(|x| {
                    let tile = Tile::new(
                        x * tile_width,
                        y * tile_height,
                        (x + 1) * tile_width,
                        (y + 1) * tile_height,
                        self.frame_buffer.clone(),
                        self.resolution,
                    );
                    let scene = self.scene.clone();

                    TileRenderer::render_tile(tile, scene, current_spp, branch_count);
                    current_spp += branch_count;
                    self.current_spp
                        .fetch_add(branch_count, std::sync::atomic::Ordering::SeqCst);
                })
            });
        }

        //save image
        let image_buffer = self.frame_buffer.lock().unwrap();
        let float_copy: Vec<f32> = image_buffer
            .as_slice()
            .into_iter()
            .map(|pixel| {
                let a: [f32; 4] = pixel.into();
                a
            })
            .flat_map(|floats| floats.into_iter())
            .collect();

        let image = Rgba32FImage::from_vec(
            self.resolution.0 as u32,
            self.resolution.1 as u32,
            float_copy,
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

    pub fn render_tile(mut tile: Tile, scene: Arc<Scene>, current_spp: u32, branch_count: u32) {
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let x_normalized =
                    ((2 * x + 1) as f32 - tile.frame_buffer_resolution.0 as f32) / tile.dim;
                let y_normalized = ((2 * (tile.frame_buffer_resolution.1 - y) - 1) as f32
                    - tile.frame_buffer_resolution.1 as f32)
                    / tile.dim;

                let dx = rng.gen_range((-1.0 / tile.dim)..(1.0 / tile.dim));
                let dy = rng.gen_range((-1.0 / tile.dim)..(1.0 / tile.dim));
                let color = Vec3::new(
                    x_normalized + dx,
                    y_normalized + dy,
                    current_spp as f32 / 1000.0,
                );
                //scene.get_color(x_normalized + dx, y_normalized + dy, &mut rng, current_spp);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = color.x * branch_count as f32;
                let g = color.y * branch_count as f32;
                let b = color.z * branch_count as f32;
                tile.local_buffer[local_buffer_idx].r = r;
                tile.local_buffer[local_buffer_idx].g = g;
                tile.local_buffer[local_buffer_idx].b = b;
            }
        }

        let mut frame_buffer = tile.frame_buffer.lock().unwrap();
        let sin_v = 1.0 / (branch_count + current_spp) as f32;

        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let frame_buffer_idx = Self::get_pixel_index(x, y, tile.frame_buffer_stride);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = tile.local_buffer[local_buffer_idx].r;
                let g = tile.local_buffer[local_buffer_idx].g;
                let b = tile.local_buffer[local_buffer_idx].b;

                frame_buffer[frame_buffer_idx].r =
                    (frame_buffer[frame_buffer_idx].r * current_spp as f32 + r) * sin_v;
                frame_buffer[frame_buffer_idx].g =
                    (frame_buffer[frame_buffer_idx].g * current_spp as f32 + g) * sin_v;
                frame_buffer[frame_buffer_idx].b =
                    (frame_buffer[frame_buffer_idx].b * current_spp as f32 + b) * sin_v;
            }
        }
    }
    #[inline]
    fn get_pixel_index(x: usize, y: usize, stride: usize) -> usize {
        let val = (y * stride) + x;
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

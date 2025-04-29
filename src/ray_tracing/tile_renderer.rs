use std::cell::Cell;
use std::future::Future;
use std::mem::transmute;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{self, Arc, Mutex, MutexGuard, RwLock};
use std::thread::{park, spawn, JoinHandle};
use std::time::Instant;

use glam::{Vec2, Vec3, Vec4};
use rand::rngs::StdRng;

use image::{Pixel, RgbImage, Rgba32FImage, RgbaImage};
use rand::{Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use rayon::result;

use crate::mandelbrot::mandelbrot;
use crate::ourple;

use super::scene::Scene;
use super::texture::LUT_TABLE_BYTE;

pub struct Tile {
    stride: usize,
    frame_buffer_stride: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    local_buffer: Vec<F32Color>,
    frame_buffer_resolution: (usize, usize),
    dim: f32,
    frame_buffer: Arc<Mutex<Vec<F32Color>>>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct F32Color {
    data: Vec4,
}

impl F32Color {
    pub fn r(&self) -> f32 {
        self.data[0]
    }
    pub fn g(&self) -> f32 {
        self.data[1]
    }
    pub fn b(&self) -> f32 {
        self.data[2]
    }
    pub fn a(&self) -> f32 {
        self.data[3]
    }
    pub fn r_mut(&mut self) -> &mut f32 {
        &mut self.data[0]
    }
    pub fn g_mut(&mut self) -> &mut f32 {
        &mut self.data[1]
    }
    pub fn b_mut(&mut self) -> &mut f32 {
        &mut self.data[2]
    }
    pub fn a_mut(&mut self) -> &mut f32 {
        &mut self.data[3]
    }
}

impl F32Color {
    pub const BLACK: F32Color = F32Color {
        data: Vec4::new(0.0, 0.0, 0.0, 1.0),
    };
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct U8Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl U8Color {
    pub const BLACK: U8Color = U8Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl Into<[u8; 4]> for U8Color {
    fn into(self) -> [u8; 4] {
        unsafe { transmute::<U8Color, [u8; 4]>(self) }
    }
}

impl From<&F32Color> for U8Color {
    fn from(value: &F32Color) -> Self {
        let res = (value.data * 255.0).min(Vec4::splat(255.0));
        let r = LUT_TABLE_BYTE[res[0] as usize];
        let g = LUT_TABLE_BYTE[res[1] as usize];
        let b = LUT_TABLE_BYTE[res[2] as usize];
        U8Color {
            r,
            g,
            b,
            a: res[3] as u8,
        }
    }
}

enum RendererMessage {
    GetImage,
    Pause,
    Stop,
    Resume,
}
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum RendererStatus {
    Busy = 0,
    Paused = 1,
    Stopped = 2,
}
impl RendererStatus {
    pub fn from_usize(value: usize) -> RendererStatus {
        match value {
            0 => RendererStatus::Busy,
            1 => RendererStatus::Paused,
            2 => RendererStatus::Stopped,
            _ => panic!(),
        }
    }
}

pub struct TileRenderer {
    status: Arc<AtomicUsize>,
    render_thread: Option<JoinHandle<Vec<Tile>>>,
    msg_channel: Option<Sender<RendererMessage>>,
    output_image_receiver: Option<Receiver<Vec<U8Color>>>,
    current_spp: Arc<AtomicU32>,
    pub thread_count: usize,
    resolution: (usize, usize),
    scene: Arc<RwLock<Scene>>,
}

impl Default for TileRenderer {
    fn default() -> Self {
        Self {
            current_spp: Default::default(),
            thread_count: Default::default(),
            resolution: Default::default(),
            scene: Default::default(),
            render_thread: None,
            msg_channel: None,
            status: Arc::new(AtomicUsize::new(RendererStatus::Stopped as usize)),
            output_image_receiver: None,
        }
    }
}

impl Tile {
    pub fn new(
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        frame_buffer: Arc<Mutex<Vec<F32Color>>>,
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
            local_buffer: vec![F32Color::BLACK; (y1 - y0) * (x1 - x0)],
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
            scene: Arc::new(RwLock::new(scene)),
            current_spp: Arc::new(AtomicU32::new(0)),
            render_thread: None,
            msg_channel: None,
            status: Arc::new(AtomicUsize::new(RendererStatus::Stopped as usize)),
            output_image_receiver: None,
        }
    }

    pub fn get_resolution(&self) -> (usize, usize) {
        self.resolution.clone()
    }

    pub fn set_resolution(&mut self, resolution: (usize, usize)) {
        self.resolution = resolution;
    }
    pub fn get_image(&mut self) -> Option<&[U8Color]> {}
    pub fn get_current_spp(&self) -> u32 {
        self.current_spp.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn spp_add(&self, amount: u32) {
        self.current_spp
            .fetch_add(amount, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn pause(&self) {
        self.msg_channel
            .as_ref()
            .unwrap()
            .send(RendererMessage::Pause)
            .unwrap();
    }
    pub fn stop(&self) {
        self.msg_channel
            .as_ref()
            .unwrap()
            .send(RendererMessage::Stop)
            .unwrap();
    }
    pub fn get_renderer_status(&self) -> RendererStatus {
        RendererStatus::from_usize(self.status.load(sync::atomic::Ordering::SeqCst))
    }
    pub fn resume(&self) {
        match &self.render_thread {
            Some(thread) => thread.thread().unpark(),
            None => {}
        }
    }
    pub fn collect_samples(&mut self) {
        if self.render_thread.is_some() {
            return;
        }

        let (msg_sender, msg_receiver) = channel::<RendererMessage>();
        let (img_sender, img_receiver) = channel::<Vec<U8Color>>();

        self.msg_channel = Some(msg_sender);

        let spp_arc = self.current_spp.clone();
        let scene_arc = self.scene.clone();

        let resolution = self.resolution;
        let thread_count = self.thread_count;
        let status_arc = self.status.clone();

        Some(spawn(move || {
            Self::thread_task(
                spp_arc,
                status_arc,
                scene_arc,
                msg_receiver,
                img_sender,
                resolution,
                thread_count,
            )
        }));
    }

    fn thread_task(
        spp_arc: Arc<AtomicU32>,
        status_arc: Arc<AtomicUsize>,
        scene_arc: Arc<RwLock<Scene>>,
        msg_receiver: Receiver<RendererMessage>,
        output_image_sender: Sender<Vec<U8Color>>,
        resolution: (usize, usize),
        rayon_thread_count: usize,
    ) {
        let frame_buffer = Arc::new(Mutex::new(
            (0..resolution.0 * resolution.1)
                .into_iter()
                .map(|_| F32Color::BLACK)
                .collect::<Vec<_>>(),
        ));

        let tile_width = (resolution.0 + rayon_thread_count - 1) / rayon_thread_count;
        let tile_height = (resolution.1 + rayon_thread_count - 1) / rayon_thread_count;
        let mut tiles = Vec::with_capacity(rayon_thread_count * rayon_thread_count);
        (0..rayon_thread_count).for_each(|y| {
            (0..rayon_thread_count).for_each(|x| {
                let tile = Tile::new(
                    x * tile_width,
                    y * tile_height,
                    (x + 1) * tile_width,
                    (y + 1) * tile_height,
                    frame_buffer.clone(),
                    resolution,
                );
                tiles.push(tile);
            });
        });
        loop {
            let current_spp = spp_arc.load(std::sync::atomic::Ordering::SeqCst);
            let scene = scene_arc.read().unwrap();
            let branch_count = Scene::get_current_branch_count(scene.branch_count, current_spp);

            let mut should_pause = match msg_receiver.try_recv() {
                Ok(RendererMessage::Pause) => {
                    status_arc.store(
                        RendererStatus::Paused as usize,
                        sync::atomic::Ordering::SeqCst,
                    );
                    true
                }
                Ok(RendererMessage::Stop) => break,
                Err(_) => false,
                _ => false,
            };

            while should_pause {
                match msg_receiver.recv() {
                    Ok(RendererMessage::Resume) => should_pause = false,
                    Ok(RendererMessage::GetImage) => {}
                    Err(_) => break,
                    _ => {}
                }
            }

            if scene.target_spp <= current_spp {
                dbg!("Reached target SPP!");
                break;
            }
            dbg!(current_spp);
            dbg!(branch_count);
            tiles.par_iter_mut().for_each(|tile| {
                TileRenderer::render_tile_average(tile, &scene, current_spp, branch_count);
            });

            spp_arc.fetch_add(branch_count, std::sync::atomic::Ordering::SeqCst);
        }
    }
    /*
       pub fn collect_sample(&mut self) {
           let tile_width: usize = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
           let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;

           let current_spp = self.current_spp();

           let branch_count = Scene::get_current_branch_count(self.scene.branch_count, current_spp);
           let mut tiles = self.tiles.lock().unwrap();
           tiles.par_iter_mut().for_each(|tile| {
               let scene = self.scene.clone();

               TileRenderer::render_tile_average(tile, scene, current_spp, branch_count);
           });

           self.spp_add(branch_count);
           dbg!(current_spp);
       }

       pub fn render_to_image(&mut self, file_name: &str) {
           let tile_width = (self.resolution.0 + self.thread_count - 1) / self.thread_count;
           let tile_height = (self.resolution.1 + self.thread_count - 1) / self.thread_count;
           let mut current_spp = self.current_spp.load(std::sync::atomic::Ordering::SeqCst);
           let mut tiles = self.tiles.lock().unwrap();

           while current_spp < self.scene.target_spp {
               let branch_count =
                   Scene::get_current_branch_count(self.scene.branch_count, current_spp);
               tiles.par_iter_mut().for_each(|tile| {
                   let scene = self.scene.clone();

                   TileRenderer::render_tile_average(tile, scene, current_spp, branch_count);
               });
               current_spp += branch_count;
               self.current_spp
                   .fetch_add(branch_count, std::sync::atomic::Ordering::SeqCst);
           }

           //save image
           let image_buffer = self.frame_buffer.lock().unwrap();
           let float_copy: Vec<f32> = image_buffer
               .as_slice()
               .into_iter()
               .map(|pixel| {
                   let a: [f32; 4] = pixel.data.to_array();
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
    */
    pub fn render_tile_replace(mut tile: Tile, scene: Arc<Scene>, current_spp: u32) {
        let time = (current_spp as f32 / 100.0);
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let x_normalized =
                    ((2 * x + 1) as f32 - tile.frame_buffer_resolution.0 as f32) / tile.dim;
                let y_normalized = ((2 * (tile.frame_buffer_resolution.1 - y) - 1) as f32
                    - tile.frame_buffer_resolution.1 as f32)
                    / tile.dim;
                let color = ourple::main_image(
                    x as f32,
                    y as f32,
                    Vec2::new(
                        tile.frame_buffer_resolution.0 as f32,
                        tile.frame_buffer_resolution.1 as f32,
                    ),
                    time,
                );
                //scene.get_color(x_normalized + dx, y_normalized + dy, &mut rng, current_spp);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = color.x;
                let g = color.y;
                let b = color.z;
                *tile.local_buffer[local_buffer_idx].r_mut() = r;
                *tile.local_buffer[local_buffer_idx].g_mut() = g;
                *tile.local_buffer[local_buffer_idx].b_mut() = b;
            }
        }

        let mut frame_buffer = tile.frame_buffer.lock().unwrap();

        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let frame_buffer_idx = Self::get_pixel_index(x, y, tile.frame_buffer_stride);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = tile.local_buffer[local_buffer_idx].r();
                let g = tile.local_buffer[local_buffer_idx].g();
                let b = tile.local_buffer[local_buffer_idx].b();
                *frame_buffer[frame_buffer_idx].r_mut() = r;
                *frame_buffer[frame_buffer_idx].g_mut() = g;
                *frame_buffer[frame_buffer_idx].b_mut() = b;
            }
        }
    }
    pub fn render_tile_average(
        tile: &mut Tile,
        scene: &Scene,
        current_spp: u32,
        branch_count: u32,
    ) {
        let time = (current_spp as f32 / 100.0);
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
                let color =
                    scene.get_color(x_normalized + dx, y_normalized + dy, &mut rng, current_spp);
                //scene.get_color(x_normalized + dx, y_normalized + dy, &mut rng, current_spp);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = color.x * branch_count as f32;
                let g = color.y * branch_count as f32;
                let b = color.z * branch_count as f32;
                *tile.local_buffer[local_buffer_idx].r_mut() = r;
                *tile.local_buffer[local_buffer_idx].g_mut() = g;
                *tile.local_buffer[local_buffer_idx].b_mut() = b;
            }
        }

        let mut frame_buffer = tile.frame_buffer.lock().unwrap();
        let s_inv = 1.0 / (branch_count + current_spp) as f32;

        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let frame_buffer_idx = Self::get_pixel_index(x, y, tile.frame_buffer_stride);
                let local_buffer_idx = Self::get_pixel_index(x - tile.x0, y - tile.y0, tile.stride);
                let r = tile.local_buffer[local_buffer_idx].r();
                let g = tile.local_buffer[local_buffer_idx].g();
                let b = tile.local_buffer[local_buffer_idx].b();
                *frame_buffer[frame_buffer_idx].r_mut() =
                    (frame_buffer[frame_buffer_idx].r() * current_spp as f32 + r) * s_inv;
                *frame_buffer[frame_buffer_idx].g_mut() =
                    (frame_buffer[frame_buffer_idx].g() * current_spp as f32 + g) * s_inv;
                *frame_buffer[frame_buffer_idx].b_mut() =
                    (frame_buffer[frame_buffer_idx].b() * current_spp as f32 + b) * s_inv;
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

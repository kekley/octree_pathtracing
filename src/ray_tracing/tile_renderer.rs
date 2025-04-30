use core::error;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::future::Future;
use std::mem::transmute;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::sync::{self, Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
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
    GetImage(Vec<U8Color>),
    Pause,
    Stop,
    Resume,
    ChangeSpp(u32),
}
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum RendererStatus {
    Running = 0,
    Paused = 1,
    Stopped = 2,
}
impl RendererStatus {
    pub fn from_usize(value: usize) -> RendererStatus {
        match value {
            0 => RendererStatus::Running,
            1 => RendererStatus::Paused,
            2 => RendererStatus::Stopped,
            _ => panic!(),
        }
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            RendererStatus::Running => "Running",
            RendererStatus::Paused => "Paused",
            RendererStatus::Stopped => "Stopped",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererMode {
    Preview,
    PathTraced,
}

impl RendererMode {
    pub fn to_str(&self) -> &'static str {
        match self {
            RendererMode::Preview => "Preview",
            RendererMode::PathTraced => "Path Traced",
        }
    }
}

pub struct TileRenderer {
    mode: RendererMode,
    status: Arc<AtomicUsize>,
    render_thread: Option<JoinHandle<()>>,
    msg_channel: Option<Sender<RendererMessage>>,
    output_image_buffer: Option<Vec<U8Color>>,
    output_image_receiver: Option<Receiver<Vec<U8Color>>>,
    current_spp: Arc<AtomicU32>,
    branch_count: u32,
    target_spp: u32,
    thread_count: usize,
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
            output_image_buffer: None,
            target_spp: 1,
            branch_count: 10,
            mode: RendererMode::Preview,
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
    pub fn new(
        render_resolution: (usize, usize),
        samples_per_pixel: u32,
        threads: usize,
        scene: Scene,
    ) -> Self {
        Self {
            thread_count: threads,
            resolution: render_resolution,
            scene: Arc::new(RwLock::new(scene)),
            current_spp: Arc::new(AtomicU32::new(0)),
            render_thread: None,
            msg_channel: None,
            status: Arc::new(AtomicUsize::new(RendererStatus::Stopped as usize)),
            output_image_receiver: None,
            output_image_buffer: None,
            target_spp: samples_per_pixel,
            branch_count: 10,
            mode: RendererMode::Preview,
        }
    }
    pub fn edit_scene_with<F: Fn(&mut Scene)>(&mut self, f: F) {
        self.reset_render();
        let mut write_lock = self.scene.write().unwrap();
        f(&mut write_lock);
        drop(write_lock);
        self.collect_samples();
    }

    pub fn reset_render(&mut self) {
        self.stop();

        self.current_spp.store(0, sync::atomic::Ordering::SeqCst);
        self.msg_channel = None;
        self.output_image_buffer = None;
        self.output_image_receiver = None;
    }
    pub fn get_resolution(&self) -> (usize, usize) {
        self.resolution.clone()
    }

    pub fn get_mode(&self) -> RendererMode {
        self.mode
    }
    pub fn set_mode(&mut self, mode: RendererMode) {
        self.reset_render();
        self.mode = mode;
    }
    pub fn set_resolution(&mut self, resolution: (usize, usize)) {
        self.reset_render();
        self.resolution = resolution;
        self.collect_samples();
    }

    pub fn get_branch_count(&mut self) -> u32 {
        self.branch_count
    }
    pub fn set_target_spp(&mut self, target_spp: u32) {
        self.target_spp = target_spp;
        if let Some(msg_channel) = &self.msg_channel {
            match msg_channel.send(RendererMessage::ChangeSpp(target_spp)) {
                Ok(()) => {}
                Err(error) => {
                    dbg!(error);
                }
            }
        }
    }
    pub fn get_target_spp(&self) -> u32 {
        self.target_spp
    }
    pub fn get_image(&mut self) -> Option<&[U8Color]> {
        let image_buffer = self.output_image_buffer.take();
        match image_buffer {
            Some(buffer) => {
                self.msg_channel
                    .as_ref()?
                    .send(RendererMessage::GetImage(buffer))
                    .ok()?;

                None
            }
            None => match self.output_image_receiver.as_ref()?.try_recv() {
                Ok(result) => {
                    self.output_image_buffer = Some(result);
                    return Some(self.output_image_buffer.as_ref()?);
                }
                Err(_) => {
                    return None;
                }
            },
        }
    }
    pub fn get_current_spp(&self) -> u32 {
        self.current_spp.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn spp_add(&self, amount: u32) {
        self.current_spp
            .fetch_add(amount, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn pause(&self) {
        if let Some(msg_channel) = &self.msg_channel {
            match msg_channel.send(RendererMessage::Pause) {
                Ok(_) => {}
                Err(error) => {
                    dbg!(error.to_string());
                }
            }
        }
    }
    pub fn stop(&mut self) {
        if let Some(msg_channel) = &self.msg_channel {
            match msg_channel.send(RendererMessage::Stop) {
                Ok(_) => {}
                Err(error) => {
                    dbg!(error.to_string());
                }
            }
        }

        let thread_handle = self.render_thread.take();
        match thread_handle {
            Some(thread) => thread.join().unwrap(),
            None => {}
        }
    }
    pub fn get_renderer_status(&self) -> RendererStatus {
        RendererStatus::from_usize(self.status.load(sync::atomic::Ordering::SeqCst))
    }
    pub fn resume(&self) {
        if let Some(msg_channel) = &self.msg_channel {
            match msg_channel.send(RendererMessage::Resume) {
                Ok(_) => {}
                Err(error) => {
                    dbg!(error.to_string());
                }
            }
        }
        match &self.render_thread {
            Some(thread) => thread.thread().unpark(),
            None => {}
        }
    }

    pub fn start(&mut self) {
        match self.mode {
            RendererMode::Preview => self.render_preview(),
            RendererMode::PathTraced => self.collect_samples(),
        }
    }
    fn collect_samples(&mut self) {
        if self.render_thread.is_some() {
            return;
        }
        self.current_spp.store(0, sync::atomic::Ordering::SeqCst);
        let (msg_sender, msg_receiver) = channel::<RendererMessage>();
        let (img_sender, img_receiver) = channel::<Vec<U8Color>>();

        self.msg_channel = Some(msg_sender);
        self.output_image_receiver = Some(img_receiver);

        let target_spp = self.target_spp;
        let spp_arc = self.current_spp.clone();
        let scene_arc = self.scene.clone();

        let resolution = self.resolution;
        let thread_count = self.thread_count;
        let status_arc = self.status.clone();

        let image_output_buffer = (0..resolution.0 * resolution.1)
            .into_iter()
            .map(|_| U8Color::BLACK)
            .collect::<Vec<_>>();

        self.output_image_buffer = Some(image_output_buffer);

        Some(spawn(move || {
            Self::thread_task(
                spp_arc,
                status_arc,
                scene_arc,
                msg_receiver,
                img_sender,
                resolution,
                thread_count,
                target_spp,
            )
        }));
    }

    fn render_preview(&mut self) {
        if self.render_thread.is_some() {
            return;
        }
        let (msg_sender, msg_receiver) = channel::<RendererMessage>();
        let (img_sender, img_receiver) = channel::<Vec<U8Color>>();

        self.msg_channel = Some(msg_sender);
        self.output_image_receiver = Some(img_receiver);

        let scene_arc = self.scene.clone();

        let resolution = self.resolution;
        let thread_count = self.thread_count;
        let status_arc = self.status.clone();

        let image_output_buffer = (0..resolution.0 * resolution.1)
            .into_iter()
            .map(|_| U8Color::BLACK)
            .collect::<Vec<_>>();

        self.output_image_buffer = Some(image_output_buffer);

        Some(spawn(move || {
            Self::preview_thread_task(
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
        mut target_spp: u32,
    ) {
        status_arc.store(
            RendererStatus::Running as usize,
            sync::atomic::Ordering::SeqCst,
        );
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
        'outer: loop {
            let current_spp = spp_arc.load(std::sync::atomic::Ordering::SeqCst);
            let scene = scene_arc.read().unwrap();
            let branch_count = Scene::get_current_branch_count(scene.branch_count, current_spp);

            loop {
                let status =
                    RendererStatus::from_usize(status_arc.load(sync::atomic::Ordering::SeqCst));
                let message = match status {
                    RendererStatus::Running => msg_receiver.try_recv().ok(),
                    RendererStatus::Paused => {
                        Some(msg_receiver.recv().unwrap_or(RendererMessage::Stop))
                    }
                    RendererStatus::Stopped => break 'outer,
                };

                match message {
                    Some(RendererMessage::ChangeSpp(new_spp)) => {
                        if target_spp < new_spp {
                            target_spp = new_spp;
                        }
                    }
                    Some(RendererMessage::Resume) => {
                        status_arc.store(
                            RendererStatus::Running as usize,
                            sync::atomic::Ordering::SeqCst,
                        );
                    }
                    Some(RendererMessage::GetImage(mut buffer)) => {
                        let frame_buffer_guard = frame_buffer.lock().unwrap();
                        Self::float_buffer_to_u8(&mut buffer, &frame_buffer_guard);
                        drop(frame_buffer_guard);
                        match output_image_sender.send(buffer) {
                            Ok(_) => {}
                            Err(error) => {
                                dbg!(error.to_string());
                                drop(error.0);
                            }
                        }
                    }
                    Some(RendererMessage::Pause) => {
                        status_arc.store(
                            RendererStatus::Paused as usize,
                            sync::atomic::Ordering::SeqCst,
                        );
                    }
                    Some(RendererMessage::Stop) => break 'outer,
                    None => {
                        break;
                    }
                }
            }

            if current_spp < target_spp {
                tiles.iter_mut().for_each(|tile| {
                    TileRenderer::render_tile_average(tile, &scene, current_spp, branch_count);
                });
                spp_arc.fetch_add(branch_count, std::sync::atomic::Ordering::SeqCst);
            }
        }
        status_arc.store(
            RendererStatus::Stopped as usize,
            sync::atomic::Ordering::SeqCst,
        );
        dbg!("thread finished");
    }

    fn preview_thread_task(
        status_arc: Arc<AtomicUsize>,
        scene_arc: Arc<RwLock<Scene>>,
        msg_receiver: Receiver<RendererMessage>,
        output_image_sender: Sender<Vec<U8Color>>,
        resolution: (usize, usize),
        rayon_thread_count: usize,
    ) {
        status_arc.store(
            RendererStatus::Running as usize,
            sync::atomic::Ordering::SeqCst,
        );
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
        'outer: loop {
            let scene = scene_arc.read().unwrap();
            loop {
                let status =
                    RendererStatus::from_usize(status_arc.load(sync::atomic::Ordering::SeqCst));
                let message = match status {
                    RendererStatus::Running => msg_receiver.try_recv().ok(),
                    RendererStatus::Paused => {
                        Some(msg_receiver.recv().unwrap_or(RendererMessage::Stop))
                    }
                    RendererStatus::Stopped => break 'outer,
                };

                match message {
                    Some(RendererMessage::ChangeSpp(_)) => {}
                    Some(RendererMessage::Resume) => {
                        status_arc.store(
                            RendererStatus::Running as usize,
                            sync::atomic::Ordering::SeqCst,
                        );
                    }
                    Some(RendererMessage::GetImage(mut buffer)) => {
                        let frame_buffer_guard = frame_buffer.lock().unwrap();
                        Self::float_buffer_to_u8(&mut buffer, &frame_buffer_guard);
                        drop(frame_buffer_guard);
                        match output_image_sender.send(buffer) {
                            Ok(_) => {}
                            Err(error) => {
                                dbg!(error.to_string());
                                drop(error.0);
                            }
                        }
                    }
                    Some(RendererMessage::Pause) => {
                        status_arc.store(
                            RendererStatus::Paused as usize,
                            sync::atomic::Ordering::SeqCst,
                        );
                    }
                    Some(RendererMessage::Stop) => break 'outer,
                    None => {
                        break;
                    }
                }
            }
            tiles.iter_mut().for_each(|tile| {
                TileRenderer::render_tile_replace(tile, &scene);
            });
        }
        status_arc.store(
            RendererStatus::Stopped as usize,
            sync::atomic::Ordering::SeqCst,
        );
        dbg!("thread finished");
    }

    fn float_buffer_to_u8(u8: &mut [U8Color], float: &[F32Color]) {
        u8.iter_mut().zip(float.iter()).for_each(|(u8, float)| {
            *u8 = U8Color::from(float);
        });
    }

    pub fn render_tile_replace(tile: &mut Tile, scene: &Scene) {
        let mut rng = StdRng::from_entropy();
        for y in tile.y0..tile.y1 {
            for x in tile.x0..tile.x1 {
                let x_normalized =
                    ((2 * x + 1) as f32 - tile.frame_buffer_resolution.0 as f32) / tile.dim;
                let y_normalized = ((2 * (tile.frame_buffer_resolution.1 - y) - 1) as f32
                    - tile.frame_buffer_resolution.1 as f32)
                    / tile.dim;
                let color = scene.get_preview_color(x_normalized, y_normalized, &mut rng);
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

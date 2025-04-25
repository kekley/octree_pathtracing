use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::egui::{
    self, include_image, load::SizedTexture, mutex::Mutex, Color32, ColorImage, Image, ImageData,
    ImageOptions, ImageSource, Slider, TextureHandle, TextureOptions,
};

use crate::ray_tracing::{
    scene::Scene,
    tile_renderer::{TileRenderer, U8Pixel},
};

pub struct Application {
    refresh_time: Instant,
    window_title: String,
    window_size: (usize, usize),
    renderer: TileRenderer,
    render_texture: Option<TextureHandle>,
    local_texture_buffer: Vec<U8Pixel>,
    spp: u32,
    pause: bool,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            render_texture: None,
            spp: 0,
            renderer: TileRenderer::new((500, 500), 8, Scene::mc()),
            refresh_time: Instant::now(),
            local_texture_buffer: vec![U8Pixel::BLACK; 500 * 500],
            pause: true,
        }
    }
}

fn pixel_slice_to_u8_slice(slice: &[U8Pixel]) -> &[u8] {
    let ptr = slice.as_ptr();
    let len = slice.len() * size_of::<U8Pixel>();
    unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
}
impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let texture: &mut egui::TextureHandle = self.render_texture.get_or_insert_with(|| {
            ctx.load_texture(
                "render_texture",
                ImageData::Color(Arc::new(ColorImage {
                    size: self.window_size.into(),
                    pixels: vec![Color32::DARK_GRAY; self.window_size.0 * self.window_size.1],
                })),
                TextureOptions::default(),
            )
        });

        let new_spp = self
            .renderer
            .current_spp
            .load(std::sync::atomic::Ordering::SeqCst);
        let render_res = self.renderer.resolution;
        if new_spp > self.spp
            && !self.pause
            && (Instant::now().duration_since(self.refresh_time) > Duration::from_millis(16))
        {
            self.renderer.send_pause_signal();
            if self.renderer.is_idle() {
                self.renderer
                    .get_frame_buffer_data(&mut self.local_texture_buffer);

                let u8_buffer = pixel_slice_to_u8_slice(&self.local_texture_buffer);

                let color_image: Arc<ColorImage> = Arc::new(ColorImage::from_rgba_premultiplied(
                    render_res.into(),
                    u8_buffer,
                ));

                texture.set(color_image, TextureOptions::default());

                self.spp = new_spp;
                self.renderer.resume();

                self.refresh_time = Instant::now();
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Start Rendering").clicked() {
                self.pause = false;
                if self.renderer.worker_thread.is_none() {
                    self.renderer.collect_samples()
                };
            }
            if ui.button("Stop").clicked() {
                self.renderer.send_pause_signal();
                self.pause = true;
            }

            ui.label(format!("Hi {:p}", texture));

            ui.image(SizedTexture::from_handle(texture));
        });
        ctx.request_repaint();
    }
}

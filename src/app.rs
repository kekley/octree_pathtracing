use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::egui::{
    self, include_image, load::SizedTexture, mutex::Mutex, Color32, ColorImage, Image, ImageData,
    ImageOptions, ImageSource, Slider, TextureHandle, TextureOptions,
};

use crate::ray_tracing::{scene::Scene, tile_renderer::TileRenderer};

pub struct Application {
    refresh_time: Instant,
    window_title: String,
    window_size: (usize, usize),
    renderer: TileRenderer,
    render_texture: Option<TextureHandle>,
    spp: u32,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            render_texture: None,
            spp: 0,
            renderer: TileRenderer::new((1280, 720), 1, Scene::mc()),
            refresh_time: Instant::now(),
        }
    }
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
            && (Instant::now().duration_since(self.refresh_time) > Duration::from_millis(200))
        {
            self.renderer.send_pause_signal();
            if self.renderer.is_idle() {
                let image = self.renderer.get_frame_buffer_data();
                let color_image: Arc<ColorImage> = Arc::new(ColorImage::from_rgba_premultiplied(
                    render_res.into(),
                    &image,
                ));
                texture.set(color_image, TextureOptions::default());
                self.spp = new_spp;
                self.renderer.resume();
                self.refresh_time = Instant::now();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hi");
            ui.horizontal(|ui| {
                let name_label = ui.label("greasy balls");
                ui.text_edit_singleline(&mut self.window_title)
                    .labelled_by(name_label.id);
            });
            if ui.button("do thing").clicked() {
                if self.renderer.worker_thread.is_none() {
                    self.renderer.collect_samples()
                };
            }

            ui.label(format!("Hi {:p}", texture));

            ui.image(SizedTexture::from_handle(texture));
        });
    }
}

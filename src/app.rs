use std::{
    fmt::format,
    future::{poll_fn, Future, IntoFuture},
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::Poll,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::egui::{
    self, include_image, load::SizedTexture, mutex::Mutex, Color32, ColorImage, DragValue, Image,
    ImageData, ImageOptions, ImageSource, Label, RadioButton, Slider, TextureHandle,
    TextureOptions,
};

use crate::ray_tracing::{
    camera::Camera,
    scene::{self, Scene},
    tile_renderer::{RendererMode, RendererStatus, TileRenderer, U8Color},
};

pub struct Application {
    refresh_time: Instant,
    window_title: String,
    window_size: (usize, usize),
    renderer: TileRenderer,
    render_texture: Option<TextureHandle>,

    local_renderer_mode: RendererMode,
    local_renderer_resolution: (usize, usize),
    local_current_spp: u32,
    local_target_spp: u32,
    local_camera: Camera,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            render_texture: None,
            local_current_spp: 0,
            renderer: TileRenderer::new((1280, 720), 100, 8, Scene::mc()),
            local_renderer_resolution: (1280, 720),
            refresh_time: Instant::now(),
            local_target_spp: 100,
            local_renderer_mode: RendererMode::Preview,
            local_camera: Camera::default(),
        }
    }
}

fn pixel_slice_to_u8_slice(slice: &[U8Color]) -> &[u8] {
    let ptr = slice.as_ptr();
    let len = slice.len() * size_of::<U8Color>();
    unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let texture: &mut egui::TextureHandle = self.render_texture.get_or_insert_with(|| {
            self.local_camera = self.renderer.get_camera();
            ctx.load_texture(
                "render_texture",
                ImageData::Color(Arc::new(ColorImage {
                    size: self.window_size.into(),
                    pixels: vec![Color32::DARK_GRAY; self.window_size.0 * self.window_size.1],
                })),
                TextureOptions::default(),
            )
        });

        let latest_render_resolution = self.renderer.get_resolution();
        self.local_camera.move_with_wasd(ctx);
        self.local_camera.rotate(ctx);
        if self.renderer.get_camera() != self.local_camera {
            self.renderer
                .edit_scene_with(|f| f.camera = self.local_camera);
        }
        let latest_spp = self.renderer.get_current_spp();
        if ((latest_spp != self.local_current_spp)
            || (self.local_renderer_mode == RendererMode::Preview))
            && (Instant::now().duration_since(self.refresh_time) > Duration::from_millis(1))
        {
            let image = self.renderer.get_image();
            if image.is_some() {
                let u8_buffer = pixel_slice_to_u8_slice(image.unwrap());

                let color_image: Arc<ColorImage> = Arc::new(ColorImage::from_rgba_premultiplied(
                    latest_render_resolution.into(),
                    u8_buffer,
                ));

                texture.set(color_image, TextureOptions::default());

                self.local_current_spp = latest_spp;
                self.refresh_time = Instant::now();
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start Rendering").clicked() {
                    match self.renderer.get_renderer_status() {
                        crate::ray_tracing::tile_renderer::RendererStatus::Running => {}
                        crate::ray_tracing::tile_renderer::RendererStatus::Paused => {
                            self.renderer.resume()
                        }
                        crate::ray_tracing::tile_renderer::RendererStatus::Stopped => {
                            self.renderer.start()
                        }
                    }
                }
                if ui.button("Pause").clicked() {
                    self.renderer.pause();
                }
                if ui.button("Stop").clicked() {
                    self.renderer.stop();
                }
                ui.add(Label::new(format(format_args!(
                    "Renderer Status: {}",
                    self.renderer.get_renderer_status().to_str()
                ))));
                ui.separator();
                ui.add(Label::new("Rendering Mode: "));
                if ui
                    .add(RadioButton::new(
                        self.local_renderer_mode == RendererMode::Preview,
                        "Preview",
                    ))
                    .clicked()
                {
                    self.local_renderer_mode = RendererMode::Preview;
                    self.renderer.set_mode(RendererMode::Preview);
                }
                if ui
                    .add(RadioButton::new(
                        self.local_renderer_mode == RendererMode::PathTraced,
                        "Path Traced",
                    ))
                    .clicked()
                {
                    self.local_renderer_mode = RendererMode::PathTraced;
                    self.renderer.set_mode(RendererMode::PathTraced);
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.add(Label::new(format(format_args!(
                        "Current Rendered Samples: {}",
                        self.local_current_spp
                    ))));
                    ui.horizontal(|ui| {
                        ui.add(Label::new("Target Samples Per Pixel: "));
                        ui.add(
                            DragValue::new(&mut self.local_target_spp)
                                .speed(self.renderer.get_branch_count())
                                .update_while_editing(false),
                        );
                        if ui.button("Apply").clicked() {
                            self.renderer.set_target_spp(self.local_target_spp);
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.add(Label::new("X Resolution: "));
                    ui.add(DragValue::new(&mut self.local_renderer_resolution.0).range(100..=5000));
                });
                ui.vertical(|ui| {
                    ui.add(Label::new("Y Resolution: "));
                    ui.add(DragValue::new(&mut self.local_renderer_resolution.1).range(100..=5000))
                });
                if ui.button("Apply").clicked() {
                    if self.local_renderer_resolution != self.renderer.get_resolution() {
                        self.renderer.set_resolution(self.local_renderer_resolution);
                    }
                }
                ui.separator()
            });
            ui.separator();

            ui.image(SizedTexture::from_handle(texture));
        });
        ctx.request_repaint();
    }
}

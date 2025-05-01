use std::{
    fmt::format,
    future::{poll_fn, Future, IntoFuture},
    pin::Pin,
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock},
    task::Poll,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::egui::{
    self, include_image, load::SizedTexture, Color32, ColorImage, DragValue, Image, ImageData,
    ImageOptions, ImageSource, Label, RadioButton, Slider, TextureHandle, TextureOptions,
};
use glam::UVec3;
use spider_eye::{loaded_world::WorldCoords, MCResourceLoader};

use crate::{
    ray_tracing::{
        camera::Camera,
        resource_manager::{ModelManager, ResourceModel},
        scene::{self, Scene},
        tile_renderer::{RendererMode, RendererStatus, TileRenderer, U8Color},
    },
    voxels::octree::Octree,
};

pub struct Application {
    refresh_time: Instant,
    window_title: String,
    window_size: (usize, usize),
    model_manager: ModelManager,
    renderer: TileRenderer,
    render_texture: Option<TextureHandle>,
    scene: Option<Arc<RwLock<Scene>>>,
    local_renderer_mode: RendererMode,
    local_renderer_resolution: (usize, usize),
    local_current_spp: u32,
    local_target_spp: u32,
    local_camera: Camera,
}
pub fn mc() -> (ModelManager, Scene) {
    let model_manager = ModelManager::new();
    let minecraft_loader = &model_manager.resource_loader;
    let world = minecraft_loader.open_world("./world");
    let air = minecraft_loader.rodeo.get_or_intern("minecraft:air");
    let cave_air = minecraft_loader.rodeo.get_or_intern("minecraft:cave_air");
    let grass = minecraft_loader.rodeo.get_or_intern("minecraft:grass");
    let water = minecraft_loader.rodeo.get_or_intern("minecraft:water");
    let lava = minecraft_loader.rodeo.get_or_intern("minecraft:lava");
    let chest = minecraft_loader.rodeo.get_or_intern("minecraft:chest");
    let birch_wall_sign = minecraft_loader
        .rodeo
        .get_or_intern("minecraft:birch_wall_sign");
    let bubble_column = minecraft_loader
        .rodeo
        .get_or_intern("minecraft:bubble_column");

    let f = |position: UVec3| -> Option<ResourceModel> {
        let UVec3 { x, y, z } = position;
        //println!("position: {}", position);
        let block = world.get_block(&WorldCoords {
            x: (x as i64),
            y: (y as i64 - 64),
            z: (z as i64),
        });
        if let Some(block) = block {
            if block.block_name == air
                || block.block_name == cave_air
                || block.block_name == grass
                || block.block_name == water
                || block.block_name == lava
                || block.block_name == chest
                || block.block_name == birch_wall_sign
                || block.block_name == bubble_column
            {
                return None;
            } else {
                //println!("not air");
                let model_id = model_manager.load_resource(&block);
                Some(model_id)
            }
        } else {
            None
        }
    };
    let tree: Octree<ResourceModel> = Octree::construct_parallel(8, &f);

    let scene = model_manager.build(tree);
    //println!("{:?}", tree);
    (model_manager, scene)
}
impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            render_texture: None,
            local_current_spp: 0,
            model_manager: ModelManager::new(),
            renderer: TileRenderer::new((1280, 720), 100, 10, 16),
            local_renderer_resolution: (1280, 720),
            refresh_time: Instant::now(),
            local_target_spp: 100,
            local_renderer_mode: RendererMode::Preview,
            local_camera: Camera::default(),
            scene: None,
        }
    }
}

fn pixel_slice_to_u8_slice(slice: &[U8Color]) -> &[u8] {
    let ptr = slice.as_ptr();
    let len = slice.len() * size_of::<U8Color>();
    unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
}

impl Application {
    pub fn build_scene(&mut self) {
        let a = mc();
        self.model_manager = a.0;
        self.scene = Some(Arc::new(RwLock::new(a.1)));
    }
}
impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.scene.is_none() {
            self.build_scene();
        }
        let texture: &mut egui::TextureHandle = self.render_texture.get_or_insert_with(|| {
            self.local_camera = self.renderer.get_camera().clone();

            ctx.load_texture(
                "render_texture",
                ImageData::Color(Arc::new(ColorImage {
                    size: self.window_size.into(),
                    pixels: vec![Color32::DARK_GRAY; self.window_size.0 * self.window_size.1],
                })),
                TextureOptions::default(),
            )
        });
        self.renderer.do_it_syncronously(
            self.scene.clone().unwrap(),
            Arc::new(Mutex::new(self.local_camera.clone())),
        );
        let latest_render_resolution = self.renderer.get_resolution();

        if self.renderer.get_mode() == RendererMode::Preview {
            self.renderer.edit_camera(|camera| {
                camera.move_with_wasd(ctx);
                camera.rotate(ctx);
            });
        }
        let latest_spp = self.renderer.get_current_spp();
        if ((latest_spp != self.local_current_spp)
            || (self.local_renderer_mode == RendererMode::Preview))
            && (Instant::now().duration_since(self.refresh_time) > Duration::from_millis(5))
        {
            let image = self.renderer.get_image();
            if image.is_some()
                && latest_render_resolution.0 * latest_render_resolution.1
                    == image.as_ref().unwrap().len()
            {
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
                        crate::ray_tracing::tile_renderer::RendererStatus::Stopped => self
                            .renderer
                            .render_scene(self.scene.as_ref().unwrap().clone()),
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
                ui.separator();
                ui.vertical(|ui| {
                    ui.add(Label::new("Camera Position: "));
                    ui.horizontal(|ui| {
                        ui.add(Label::new("X: "));
                        ui.add(DragValue::new(&mut self.local_camera.eye.x));

                        ui.add(Label::new("Y: "));
                        ui.add(DragValue::new(&mut self.local_camera.eye.y));

                        ui.add(Label::new("Z: "));
                        ui.add(DragValue::new(&mut self.local_camera.eye.z));
                    })
                });

                ui.separator()
            });
            ui.separator();

            ui.image(SizedTexture::from_handle(texture));
        });
        ctx.request_repaint();
    }
}

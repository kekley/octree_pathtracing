use std::{
    fmt::format,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use eframe::{
    egui::{
        self, load::SizedTexture, Color32, ColorImage, DragValue, Image, ImageData, Label,
        RadioButton, TextureHandle, TextureOptions,
    },
    wgpu::Texture,
};
use glam::{UVec3, Vec3};
use log::info;
use spider_eye::loaded_world::WorldCoords;

use crate::{
    colors::colors::U8Color,
    geometry::quad::Quad,
    gpu_structs::gpu_octree::TraversalContext,
    gpu_test::{self, SVOPipeline},
    octree::octree_parallel::ParallelOctree,
    renderer::{
        gpu_renderer::GPURenderer,
        renderer_trait::RenderingBackend,
        tile_renderer::{RendererMode, RendererStatus, TileRenderer},
    },
    scene::{self, resource_manager::ModelManager, scene::Scene},
    textures::material::Material,
};

pub struct Application {
    window_title: String,
    window_size: (usize, usize),
    renderer: Box<dyn RenderingBackend>,
    render_texture: Option<TextureHandle>,
}
/* pub fn load_world() -> (ModelManager, Scene) {
    let model_manager = ModelManager::new();
    let minecraft_loader = &model_manager.resource_loader;
    let world = minecraft_loader
        .open_world("./assets/worlds/test_world")
        .unwrap();

    let f = |position: UVec3| -> Option<ResourceModel> {
        let UVec3 { x, y, z } = position;
        //println!("position: {}", position);
        let block = world.get_block(&WorldCoords {
            x: (x as i64),
            y: (y as i64 - 30),
            z: (z as i64),
        });
        if let Some(block) = block {
            let model = model_manager.load_resource(&block);
            if let Some(model) = model {
                return Some(model);
            } else {
                return None;
            }
        } else {
            None
        }
    };
    let tree: Octree<ResourceModel> = Octree::construct_parallel(8, &f);
    dbg!(tree.octants.len());
    let scene = model_manager.build(tree);
    //println!("{:?}", tree);
    (model_manager, scene)
} */
pub fn load_world_2() -> (ModelManager, Scene) {
    let origin = WorldCoords { x: 0, y: -64, z: 0 };
    let depth = 9;
    let model_manager = ModelManager::new();
    let minecraft_loader = &model_manager.resource_loader;
    let world = minecraft_loader
        .open_world("./assets/worlds/test_world")
        .unwrap();
    let octree = ParallelOctree::load_mc_world::<UVec3>(origin, depth, world, &model_manager);
    let octree_memory =
        (octree.octants.len() * size_of_val(&octree.octants[0].get())) as f32 / 1000000.0;
    let material_memory =
        (model_manager.materials.len() * size_of::<Material>()) as f32 / 1000000.0;
    let texture_memory = (model_manager.materials.len() * 16 * 16 * 4) as f32 / 1000000.0;
    let quad_memory = (model_manager.quads.read().len() * size_of::<Quad>()) as f32 / 1000000.0;
    info!(
        "Octree memory: {}MB, Materials memory: {}MB, Texture memory est.:{}MB, Quad memory: {}MB",
        octree_memory, material_memory, texture_memory, quad_memory
    );
    let scene = model_manager.build(octree.into());
    //println!("{:?}", tree);
    (model_manager, scene)
}
impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            render_texture: None,
            renderer: Box::new(),
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
        let a = load_world_2();
        self.model_manager = a.0;
    }
    pub fn draw_preview_ui(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        texture: &TextureHandle,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start Rendering").clicked() {
                    match self.renderer.get_renderer_status() {
                        RendererStatus::Running => {}
                        RendererStatus::Paused => self.renderer.resume(),
                        RendererStatus::Stopped => self
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
                        let mut camera_changed = false;
                        ui.add(Label::new("X: "));
                        camera_changed |= ui
                            .add(DragValue::new(&mut self.local_camera_position.x))
                            .changed();

                        ui.add(Label::new("Y: "));
                        camera_changed |= ui
                            .add(DragValue::new(&mut self.local_camera_position.y))
                            .changed();

                        ui.add(Label::new("Z: "));
                        camera_changed |= ui
                            .add(DragValue::new(&mut self.local_camera_position.z))
                            .changed();
                    })
                });

                ui.separator()
            });
            ui.separator();

            ui.add(Image::new(SizedTexture::from_handle(texture)).shrink_to_fit())
        });
    }
    pub fn draw_path_tracer_ui(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        texture: &TextureHandle,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start Rendering").clicked() {
                    match self.renderer.get_renderer_status() {
                        RendererStatus::Running => {}
                        RendererStatus::Paused => self.renderer.resume(),
                        RendererStatus::Stopped => self
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
                        ui.add_enabled(false, DragValue::new(&mut self.local_camera_position.x));

                        ui.add(Label::new("Y: "));
                        ui.add_enabled(false, DragValue::new(&mut self.local_camera_position.y));

                        ui.add(Label::new("Z: "));
                        ui.add_enabled(false, DragValue::new(&mut self.local_camera_position.z));
                    })
                });

                ui.separator()
            });
            ui.separator();

            ui.add(Image::new(SizedTexture::from_handle(texture)).shrink_to_fit())
        });
    }
}
impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.scene.is_none() {
            let start = Instant::now();
            println!("Building Scene...");
            self.build_scene();
            let end = Instant::now();
            let duration = end.duration_since(start);
            println!("Took {duration:?} to build scene");
        };

        let texture: &egui::TextureHandle = self.render_texture.get_or_insert_with(|| {
            let image = ImageData::Color(ColorImage::new([1280, 720], Color32::GRAY).into());
            let texture_handle =
                ctx.load_texture("render target", image, TextureOptions::default());

            texture_handle
        });
        let gpu_objects: &SVOPipeline = self.gpu_objects.get_or_insert_with(|| {
            let lock = self.scene.as_ref().unwrap().read().unwrap();
            let render_state = frame.wgpu_render_state().unwrap();
            gpu_test::create_pipeline(&render_state.device, &render_state.queue, &lock)
        });
        let render_state = frame.wgpu_render_state().unwrap();

        match self.local_renderer_mode {
            RendererMode::Preview => self.draw_preview_ui(ctx, frame, texture),
            RendererMode::PathTraced => self.draw_path_tracer_ui(ctx, frame, texture),
        }
        ctx.request_repaint();
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()

        // _visuals.window_fill() would also be a natural choice
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}

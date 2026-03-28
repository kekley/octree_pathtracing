use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use eframe::{
    CreationContext, Frame,
    egui::{
        Button, CentralPanel, Color32, ColorImage, Context, DragValue, Image, ImageSource, Label,
        RadioButton, TextureFilter, TextureHandle, TextureOptions, TextureWrapMode, Ui, Visuals,
        load::SizedTexture,
    },
};
use glam::Vec3A;
use hashbrown::HashMap;
use mc_utils::{
    coords::{block::BlockCoords, region::RegionCoords},
    owned::nbt_string::NBTString,
    region::borrow::Region,
    resource_loader::LoadedResources,
    world::World,
};

use crate::{
    colors::U8Color,
    octree::{builders::region::build_region_octree, world::WorldOctree},
    renderer::{
        camera::Camera,
        cpu_renderer::CpuRenderer,
        dummy_renderer::DummyRenderer,
        renderer_trait::Renderer,
        tile_renderer::{RendererMode, RendererStatus},
    },
    scene::Scene,
};

use super::{
    settings::{BackendType, RenderSettingsWindow},
    world_loading::WorldLoadingDialog,
};

pub struct MainApp {
    settings_window: RenderSettingsWindow,
    world_loading_window: WorldLoadingDialog,
    tree: WorldOctree,
    resources: Option<LoadedResources>,
    renderer: Box<dyn Renderer>,
    render_texture: Option<TextureHandle>,
}

impl MainApp {
    pub fn new() -> Self {
        let camera = Camera::look_at(
            Vec3A::new(0.0, 135.0, 0.0),
            Vec3A::new(12.0, 130.0, 12.0),
            Vec3A::Y,
            70.0f32.to_radians(),
        );
        let cpu_renderer = CpuRenderer::builder().with_camera(camera).build();
        let path = PathBuf::from("./assets/test_worlds/region/r.1.0.mca");

        let bytes = std::fs::read(&path).unwrap();

        let region = Region::from_bytes(&bytes, RegionCoords { x: 1, z: 0 });

        let blockstate_map = Arc::new(Mutex::new(HashMap::new()));

        let air = NBTString::new_from_str("minecraft:air#normal");
        blockstate_map.lock().unwrap().insert(air, 0);

        let (tree, map) = build_region_octree(region, blockstate_map).unwrap();

        Self {
            settings_window: RenderSettingsWindow::default(),
            resources: None,
            world_loading_window: WorldLoadingDialog::default(),
            renderer: Box::new(cpu_renderer),
            render_texture: None,
            tree,
        }
    }

    pub fn draw_start_stop_button(
        &mut self,
        _ctx: &Context,
        _frame: &mut eframe::Frame,
        ui: &mut Ui,
    ) {
        let text = match self.renderer.get_status() {
            RendererStatus::Running => "Pause",
            RendererStatus::Paused => "Resume",
            RendererStatus::Stopped => "Start",
        };
        let status_text = format!("Renderer Status: {text}");

        ui.add(Label::new(status_text));
        if ui.add_enabled(true, Button::new(text)).clicked() {
            todo!();
        };
    }
    pub fn draw_mode_switch_radio_buttons(
        &mut self,
        _ctx: &Context,
        _frame: &mut Frame,
        ui: &mut Ui,
    ) {
        ui.add_enabled(
            true,
            RadioButton::new(self.renderer.get_mode() == RendererMode::Preview, "Preview"),
        );
        ui.add_enabled(
            true,
            RadioButton::new(
                self.renderer.get_mode() == RendererMode::PathTraced,
                "Ray Traced",
            ),
        );
    }
    pub fn draw_render_settings_button(
        &mut self,
        ctx: &Context,
        frame: &mut eframe::Frame,
        ui: &mut Ui,
    ) {
        if ui
            .add_enabled(true, Button::new("Render Settings"))
            .clicked()
        {
            self.settings_window.open = true;
        }
        self.settings_window.show(ctx, frame, &mut self.renderer);
    }

    pub fn draw_load_world_button(&mut self, ctx: &Context, ui: &mut Ui) {
        if ui.add_enabled(true, Button::new("Load World")).clicked() {
            self.world_loading_window.open = true;
        }
        self.world_loading_window.show(ctx, &mut self.renderer);
    }

    pub fn draw_camera_coordinates(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.add_enabled(
            self.renderer.get_backend_type() != BackendType::Dummy,
            move |ui: &mut Ui| {
                ui.vertical(|ui| {
                    ui.label("Camera Coordinates");
                    ui.horizontal(|ui| {
                        let mut camera = self.renderer.get_camera().clone();
                        ui.add(Label::new("X"));
                        ui.add(DragValue::new(&mut camera.eye.x));
                        ui.add(Label::new("Y"));
                        ui.add(DragValue::new(&mut camera.eye.y));
                        ui.add(Label::new("Z"));
                        ui.add(DragValue::new(&mut camera.eye.z));
                        self.renderer.set_camera(camera);
                    })
                })
                .response
            },
        );
    }

    pub fn draw_backend_label(&mut self, _ctx: &Context, ui: &mut Ui) {
        let string = format!(
            "Current Backend: {}",
            self.renderer.get_backend_type().to_str()
        );
        ui.add(Label::new(&string));
    }

    pub fn draw_ui(
        &mut self,
        ctx: &Context,
        frame: &mut eframe::Frame,
        texture: Option<TextureHandle>,
    ) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.draw_start_stop_button(ctx, frame, ui);
                self.draw_mode_switch_radio_buttons(ctx, frame, ui);
                self.draw_render_settings_button(ctx, frame, ui);
                self.draw_load_world_button(ctx, ui);
                self.draw_backend_label(ctx, ui);
                self.draw_camera_coordinates(ctx, ui);
            });

            if let Some(texture) = texture {
                ui.add(
                    Image::new(ImageSource::Texture(SizedTexture {
                        id: texture.id(),
                        size: texture.size_vec2(),
                    }))
                    .shrink_to_fit(),
                );
            } else {
                ui.spinner();
            }
        });
    }
}
impl eframe::App for MainApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.renderer.update(&self.tree);
        let texture = if let Some(texture) = self.render_texture.clone() {
            texture.clone()
        } else {
            let resolution = self.renderer.get_resolution();
            let gray_iter = std::iter::repeat_n(100, (resolution.0 * resolution.1) as usize);
            let image = ColorImage::from_gray_iter(
                [resolution.0 as usize, resolution.1 as usize],
                gray_iter,
            );
            ctx.load_texture(
                "RenderTexture",
                image,
                TextureOptions {
                    magnification: TextureFilter::Nearest,
                    minification: TextureFilter::Linear,
                    wrap_mode: TextureWrapMode::ClampToEdge,
                    mipmap_mode: None,
                },
            )
        };

        self.renderer.render_frame_to_texture(texture.clone());
        self.draw_ui(ctx, frame, Some(texture));

        ctx.request_repaint();
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()

        // _visuals.window_fill() would also be a natural choice
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}

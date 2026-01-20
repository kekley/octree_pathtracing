use eframe::egui::{
    self, Button, Color32, ColorImage, DragValue, Image, ImageData, ImageSource, Label,
    RadioButton, TextureHandle, TextureOptions, Ui, load::SizedTexture,
};
use glam::IVec3;
use log::info;
use mc_utils::{
    coords::{block::BlockCoords, region::RegionCoords},
    resource_loader::ResourceLoadError,
    world::World,
};

use crate::{
    colors::U8Color,
    octree::world::Octree,
    renderer::{
        gpu_renderer::GPURenderer,
        renderer_trait::{FrameInFlight, FrameInFlightPoll, RenderingBackend},
        tile_renderer::{RendererMode, RendererStatus},
    },
    scene::Scene,
};

use super::{
    settings::{RenderSettingsWindow, RendererBackendSetting},
    world_loading::WorldLoadingDialog,
};

pub struct Application {
    status: RendererStatus,
    settings: RenderSettingsWindow,
    resources: ResourceLoadError,
    world_loading_dialog: WorldLoadingDialog,
    renderer: Box<dyn RenderingBackend>,
    frame_in_flight: Option<Box<dyn FrameInFlight>>,
    render_texture: Option<TextureHandle>,
}

/*
 * notes:
 * avoid loading too many chunks at once
 *
 * start at `depth`, iterate each node, recurse until we can start loading regions into octrees
 * merge those octrees, move to next node...
 *
 * */
pub fn load_world_2(path: &str, origin: &BlockCoords, depth: u8) -> Scene {
    let world = World::new(path).unwrap();
    let origin_region = RegionCoords::from(*origin);
    let world_size = 2_i64.pow(depth as u32);
    let world_extent = world_size / 2;

    const LOWEST_Y: i64 = -64;

    todo!()
}

fn recurse(octree: &mut Octree, pos: BlockCoords, depth: u8) -> Option<u32> {
    let new_parent: Option<u32> = None;
    let size = 2_i64.pow(depth as u32);
    (0..8u8).for_each(|child_idx| {
        let x_offset = size * ((child_idx as i64) & 1);

        let y_offset = size * ((child_idx as i64 >> 1) & 1);

        let z_offset = size * ((child_idx as i64 >> 2) & 1);

        let child_pos = pos.offset(x_offset, y_offset, z_offset);

        match depth as usize {
            0..9 => {
                todo!();
            }
            9 => {
                todo!();
            }
            _ => {
                let child_index = recurse(child_pos, depth - 1);
                let Some(child) = child_index else {
                    return;
                };
            }
        }
    });
}

impl Default for Application {
    fn default() -> Self {
        Self {
            render_texture: None,
            renderer: Default::default(),
            status: RendererStatus::Stopped,
            frame_in_flight: None,
            settings: Default::default(),
            world_loading_dialog: Default::default(),
            resources: todo!(),
        }
    }
}

//TODO move this to colors module
fn pixel_slice_to_u8_slice(slice: &[U8Color]) -> &[u8] {
    let ptr = slice.as_ptr();
    let len = std::mem::size_of_val(slice);
    unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
}

impl Application {
    pub fn draw_start_stop_button(
        &mut self,
        _ctx: &egui::Context,
        frame: &mut eframe::Frame,
        ui: &mut Ui,
    ) {
        let text = match self.status {
            RendererStatus::Running => "Pause",
            RendererStatus::Paused => "Resume",
            RendererStatus::Stopped => "Start",
        };
        let status_text = format!("Renderer Status: {}", self.status.to_str());

        ui.add(Label::new(status_text));
        if ui.add_enabled(true, Button::new(text)).clicked() {
            match self.status {
                RendererStatus::Running => self.status = RendererStatus::Paused,
                RendererStatus::Paused => self.status = RendererStatus::Running,
                RendererStatus::Stopped => self.status = RendererStatus::Running,
            }
        };
    }
    pub fn draw_mode_switch_radio_buttons(
        &mut self,
        _ctx: &egui::Context,
        frame: &mut eframe::Frame,
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
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        ui: &mut Ui,
    ) {
        if ui
            .add_enabled(true, Button::new("Render Settings"))
            .clicked()
        {
            self.settings.open = true;
        }
        self.settings.show(ctx, frame, &mut self.renderer);
    }
    pub fn draw_load_world_button(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        if ui.add_enabled(true, Button::new("Load World")).clicked() {
            self.world_loading_dialog.open = true;
        }
        self.world_loading_dialog.show(ctx, &mut self.renderer);
    }
    pub fn draw_camera_coordinates(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        ui.add_enabled(
            self.renderer.which_backend() != RendererBackendSetting::Dummy,
            move |ui: &mut egui::Ui| {
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
    pub fn draw_backend_label(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let string = format!(
            "Current Backend: {}",
            self.renderer.which_backend().to_str()
        );
        ui.add(Label::new(&string));
    }
    pub fn draw_ui(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        texture: &TextureHandle,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.draw_start_stop_button(ctx, frame, ui);
                self.draw_mode_switch_radio_buttons(ctx, frame, ui);
                self.draw_render_settings_button(ctx, frame, ui);
                self.draw_load_world_button(ctx, ui);
                self.draw_backend_label(ctx, ui);
                self.draw_camera_coordinates(ctx, ui);
            });
            ui.add(
                Image::new(ImageSource::Texture(SizedTexture {
                    id: texture.id(),
                    size: texture.size_vec2(),
                }))
                .shrink_to_fit(),
            )
        });
    }
}
impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.renderer.which_backend() == RendererBackendSetting::Dummy {
            let render_state = frame.wgpu_render_state().unwrap();
            self.renderer = Box::new(GPURenderer::new(
                &render_state.device,
                &render_state.queue,
                (1280, 720),
            ));
        }
        if self.render_texture.is_none() {
            info!("Creating Render Texture");
            let image = ImageData::Color(ColorImage::new([1280, 720], Color32::GRAY).into());
            let texture_handle =
                ctx.load_texture("render target", image, TextureOptions::default());

            let _ = self.render_texture.insert(texture_handle);
            return;
        }
        let texture = self.render_texture.as_ref().cloned().unwrap();

        let frame_in_flight: Option<Box<dyn FrameInFlight>> = match self.frame_in_flight.take() {
            Some(frame) => {
                let poll = frame.poll();
                match poll {
                    FrameInFlightPoll::Ready(_texture_handle) => None,
                    FrameInFlightPoll::NotReady(frame_in_flight) => Some(frame_in_flight),
                    FrameInFlightPoll::Cancelled => panic!(),
                }
            }
            None => self.renderer.render_frame(frame, texture.clone()).ok(),
        };
        if self.frame_in_flight.is_none() {
            self.renderer.as_mut().update_scene(ctx);
        } else {
            self.frame_in_flight = frame_in_flight;
        }
        self.draw_ui(ctx, frame, &texture);

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

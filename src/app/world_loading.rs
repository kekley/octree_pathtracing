use std::sync::Arc;

use eframe::egui::{self, Button, DragValue, Label, Slider, Window};
use spider_eye::coords::block::BlockCoords;

use crate::{renderer::renderer_trait::RenderingBackend, scene::resource_manager::ModelBuilder};

use super::main_app::load_world_2;

#[derive(Default)]
pub struct WorldLoadingDialog {
    model_manager: ModelBuilder,
    pub open: bool,
    path: String,
    position: BlockCoords,
    depth: u32,
}

impl WorldLoadingDialog {
    pub fn show(&mut self, ctx: &egui::Context, renderer: &mut Box<dyn RenderingBackend>) {
        match Window::new("World Loading")
            .resizable([true, true])
            .open(&mut self.open)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(Label::new("Octree Depth"));
                    ui.add(Slider::new(&mut self.depth, 2..=12));
                });
                ui.separator();
                ui.add(Label::new("Camera Position"));
                ui.horizontal(|ui| {
                    ui.add(Label::new("X:"));
                    ui.add(DragValue::new(&mut self.position.x));
                    ui.add(Label::new("Y:"));
                    ui.add(DragValue::new(&mut self.position.y));
                    ui.add(Label::new("Z:"));
                    ui.add(DragValue::new(&mut self.position.z));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.path);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.path = path.display().to_string();
                        }
                    }
                });
                if ui.add(Button::new("Load")).clicked() {
                    let scene = load_world_2(
                        &self.path,
                        &self.position,
                        self.depth as u8,
                        &self.model_manager,
                    );
                    let scene = Arc::new(parking_lot::RwLock::new(scene));
                    renderer.as_mut().set_scene(&scene);
                }
                ui.separator();
            }) {
            Some(_) => {}
            None => {}
        }
    }
}

use std::sync::Arc;

use eframe::egui::{self, Button, DragValue, Label, Slider, Window};
use mc_utils::coords::block::BlockCoords;

use crate::renderer::renderer_trait::Renderer;

#[derive(Default)]
pub struct WorldLoadingDialog {
    pub open: bool,
    path: String,
    position: BlockCoords,
    depth: u32,
}

impl WorldLoadingDialog {
    pub fn show(&mut self, ctx: &egui::Context, renderer: &mut Box<dyn Renderer>) {
        if Window::new("World Loading")
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
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_folder()
                    {
                        self.path = path.display().to_string();
                    }
                });
                if ui.add(Button::new("Load")).clicked() {
                    let scene = todo!();
                    let scene = Arc::new(parking_lot::RwLock::new(scene));
                    todo!();
                }
                ui.separator();
            })
            .is_some()
        {}
    }
}

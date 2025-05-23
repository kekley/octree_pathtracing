use std::default;

use eframe::egui::{self, DragValue, Label, RadioButton, Slider, Window};

use crate::renderer::{gpu_renderer::GPURenderer, renderer_trait::RenderingBackend};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackendSetting {
    Dummy,
    CPU,
    #[default]
    GPU,
}

impl RendererBackendSetting {
    pub fn to_str(&self) -> &'static str {
        match self {
            RendererBackendSetting::Dummy => "Dummy",
            RendererBackendSetting::CPU => "CPU",
            RendererBackendSetting::GPU => "GPU",
        }
    }
}
#[derive(Default)]
pub struct RenderSettingsWindow {
    pub open: bool,
    backend: RendererBackendSetting,
    resolution: (u32, u32),
}

impl RenderSettingsWindow {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        renderer: &mut Box<dyn RenderingBackend>,
    ) {
        match Window::new("Render Settings")
            .resizable([true, true])
            .open(&mut self.open)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add(Label::new("Backend"));
                ui.horizontal(|ui| {
                    if ui
                        .add(RadioButton::new(
                            self.backend == RendererBackendSetting::CPU,
                            "CPU",
                        ))
                        .clicked()
                    {
                        self.backend = RendererBackendSetting::CPU;
                    };
                    if ui
                        .add(RadioButton::new(
                            self.backend == RendererBackendSetting::GPU,
                            "GPU",
                        ))
                        .clicked()
                    {
                        self.backend = RendererBackendSetting::GPU;
                    };
                });
                ui.separator();
                ui.add(Label::new("Resolution"));
                ui.horizontal(|ui| {
                    ui.add(Label::new("X:"));
                    ui.add(DragValue::new(&mut self.resolution.0));
                });
                ui.horizontal(|ui| {
                    ui.add(Label::new("Y:"));
                    ui.add(DragValue::new(&mut self.resolution.1));
                });
                ui.separator();
                if ui.button("Apply").clicked() {
                    if renderer.get_resolution() != self.resolution {
                        renderer.as_mut().set_resolution(self.resolution);
                    }
                    if renderer.which_backend() != self.backend {
                        let render_state = frame.wgpu_render_state().unwrap();

                        let old_backend = match self.backend {
                            RendererBackendSetting::Dummy => panic!(),
                            RendererBackendSetting::CPU => unimplemented!(),
                            RendererBackendSetting::GPU => std::mem::replace(
                                renderer,
                                Box::new(GPURenderer::new(
                                    &render_state.device,
                                    &render_state.queue,
                                    self.resolution,
                                )),
                            ),
                        };
                        drop(old_backend);
                    }
                }
            }) {
            Some(_) => {}
            None => {
                let render_resolution = renderer.get_resolution();
                self.resolution = (render_resolution.0 as u32, render_resolution.1 as u32);
            }
        };
    }
}

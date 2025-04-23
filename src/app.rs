use std::{sync::atomic::AtomicBool, thread::JoinHandle};

use eframe::egui::{self, include_image, Slider};

use crate::ray_tracing::tile_renderer::TileRenderer;

struct PathTracerThread {
    path_tracer: TileRenderer,
    paused: AtomicBool,
    stopped: AtomicBool,
    join_handle: Option<JoinHandle<()>>,
}

impl Default for PathTracerThread {
    fn default() -> Self {
        Self {
            path_tracer: Default::default(),
            paused: Default::default(),
            stopped: Default::default(),
            join_handle: Default::default(),
        }
    }
}

impl PathTracerThread {
    fn get_frame_buffer(&self) {
        self.path_tracer
    }
}

pub struct Application {
    window_title: String,
    window_size: (usize, usize),
    number: i32,
    path_tracer_thread: PathTracerThread,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            window_size: (1280, 720),
            window_title: "hi there".to_string(),
            number: 0,
            path_tracer_thread: Default::default(),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hi");
            ui.horizontal(|ui| {
                let name_label = ui.label("greasy balls");
                ui.text_edit_singleline(&mut self.window_title)
                    .labelled_by(name_label.id);
            });
            ui.add(Slider::new(&mut self.number, -100..=100).text("number"));
            if ui.button("balls").clicked() {
                self.number += 1;
            }

            ui.label(format!("Hi {:p}", self));

            ui.image(include_image!("../test_assets/greasy.jpg"))
        });
    }
}

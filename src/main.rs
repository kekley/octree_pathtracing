extern crate ray_tracing;

use anyhow::Context;
use eframe::wgpu::{
    self, BackendOptions, Backends, DeviceDescriptor, Features, InstanceDescriptor, InstanceFlags,
    Limits, RequestAdapterOptions,
};
use egui_wgpu::{WgpuSetupCreateNew, WgpuSetupExisting};
use ray_tracing::Application;
pub const ASPECT_RATIO: f32 = 1.5;

fn main() -> Result<(), anyhow::Error> {
    //face_id_test();
    ui().unwrap();
    Ok(())
}

fn ui() -> eframe::Result {
    env_logger::init();
    let instance = wgpu::Instance::new(&InstanceDescriptor {
        backends: Backends::VULKAN
            | Backends::BROWSER_WEBGPU
            | Backends::PRIMARY
            | Backends::SECONDARY,
        flags: InstanceFlags::default(),
        backend_options: BackendOptions::default(),
    });

    let future = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    });

    let adapter = pollster::block_on(future).unwrap();
    let device_descriptor = DeviceDescriptor {
        label: None,
        required_features: Features::SHADER_INT64,
        required_limits: Limits::default(),
        memory_hints: wgpu::MemoryHints::Performance,
    };

    let future = adapter.request_device(&device_descriptor, None);
    let (device, queue): (wgpu::Device, wgpu::Queue) = pollster::block_on(future).unwrap();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        wgpu_options: egui_wgpu::WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: None,
            wgpu_setup: egui_wgpu::WgpuSetup::Existing(WgpuSetupExisting {
                instance: instance,
                adapter: adapter,
                device: device,
                queue: queue,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "f",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<Application>::default())
        }),
    )
}

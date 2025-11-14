use eframe::wgpu::{
    self, BackendOptions, Backends, DeviceDescriptor, Features, InstanceDescriptor, InstanceFlags,
    RequestAdapterOptions,
};
use egui_wgpu::WgpuSetupExisting;
use octree_pathtracing::main_app::Application;
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
    let limits = adapter.limits();
    dbg!(limits.max_buffer_size);
    let device_descriptor = DeviceDescriptor {
        label: None,
        required_features: Features::SHADER_INT64
            | Features::TEXTURE_BINDING_ARRAY
            | Features::STORAGE_RESOURCE_BINDING_ARRAY
            | Features::BUFFER_BINDING_ARRAY
            | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
        required_limits: limits,
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
                instance,
                adapter,
                device,
                queue,
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

use std::{fs, time::Instant};

use eframe::wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor, Device, Extent3d,
    MaintainBase, Origin3d, PipelineLayoutDescriptor, Queue,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StorageTextureAccess, TexelCopyTextureInfo, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, TextureViewDimension,
};
use glam::Vec4;
use log::info;

use crate::{
    gpu_structs::gpu_octree::{GPUOctree, TraversalContext},
    ray_tracing::resource_manager::ResourceModel,
    voxels::octree::Octree,
};

pub fn compute(
    octree: &Octree<ResourceModel>,
    device: &Device,
    queue: &Queue,
    target: &egui_wgpu::Texture,
) {
    let context = vec![TraversalContext {
        octree_scale: octree.octree_scale,
        root: octree.root.unwrap(),
        scale: 23 - 1,
        octant_stack: Default::default(),
        time_stack: Default::default(),
        ..Default::default()
    }];
    let octree_ = GPUOctree::from(octree);

    let octant_data = octree_.octants;

    let shader_code = fs::read_to_string("./assets/shaders/svo.wgsl").unwrap();
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
    });

    let octree = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&octant_data),
        usage: BufferUsages::STORAGE,
    });

    let context = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&context),
        usage: BufferUsages::STORAGE,
    });

    let output_texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: 1280,
            height: 720,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[TextureFormat::Rgba8Unorm],
    });

    let staging_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("staging"),
        size: (size_of::<Vec4>() * 1280 * 720) as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let texture_view = output_texture.create_view(&TextureViewDescriptor {
        usage: Some(TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC),
        label: None,
        format: Some(TextureFormat::Rgba8Unorm),
        dimension: Some(TextureViewDimension::D2),
        aspect: eframe::wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba8Unorm,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: octree.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: context.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&texture_view),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut command_encoder =
        device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    {
        let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(1280, 720, 1);
    }
    {
        command_encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: &output_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: eframe::wgpu::TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: &target.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: eframe::wgpu::TextureAspect::All,
            },
            Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
        );
    }
    queue.submit(Some(command_encoder.finish()));
    let start = Instant::now();
    device.poll(MaintainBase::Wait);
    let time = Instant::now().duration_since(start);
    info!("Took {time:?} to render on GPU");
}

use std::{fs, sync::Arc};

use glam::Vec4;
use wgpu::{
    hal::auxil::db, util::DeviceExt, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BufferBindingType, BufferDescriptor,
    BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor,
    DownlevelFlags, PipelineLayoutDescriptor, PollType, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderStages,
};

use crate::{
    gpu_structs::gpu_octree::{GPUOctree, GPUOctreeNode, TraversalContext},
    ray_tracing::{
        resource_manager::ResourceModel,
        tile_renderer::{F32Color, U8Color},
    },
    voxels::octree::Octree,
};

pub fn compute(octree: &Octree<ResourceModel>) -> Vec<U8Color> {
    dbg!(&octree.octree_scale);
    dbg!(&octree.octants[*octree.root.as_ref().unwrap() as usize]);
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
    dbg!(&octant_data[octree.root.unwrap() as usize]);
    dbg!(&context);
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    let adapter =
        pollster::block_on(instance.request_adapter(&RequestAdapterOptions::default())).unwrap();

    print!("Adapter: {:?}", adapter);

    let downlevel_capabilities = adapter.get_downlevel_capabilities();

    if !downlevel_capabilities
        .flags
        .contains(DownlevelFlags::COMPUTE_SHADERS)
    {
        panic!("No compute shader support on gpu :(")
    }

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("Failed to create device");
    let shader_code = fs::read_to_string("./assets/shaders/svo.wgsl").unwrap();
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
    });

    let octree = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&octant_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let context = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&context),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output = device.create_buffer(&BufferDescriptor {
        label: Some("output"),
        size: (size_of::<Vec4>() * 1280 * 720) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("staging"),
        size: (size_of::<Vec4>() * 1280 * 720) as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
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
                resource: output.as_entire_binding(),
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

    command_encoder.copy_buffer_to_buffer(&output, 0, &staging_buffer, 0, output.size());

    queue.submit(Some(command_encoder.finish()));

    let a = staging_buffer.slice(..);
    a.map_async(wgpu::MapMode::Read, move |r| r.unwrap());
    device.poll(PollType::wait()).unwrap();
    let mut local_buffer = vec![[0.0; 4]; 1280 * 720];
    {
        let view = a.get_mapped_range();
        local_buffer.copy_from_slice(bytemuck::cast_slice(&view));
    }

    staging_buffer.unmap();
    dbg!(&local_buffer[0]);
    let vec = local_buffer
        .iter()
        .map(|f| {
            let mut color = F32Color::BLACK;
            *color.r_mut() = f[0];
            *color.g_mut() = f[1];
            *color.b_mut() = f[2];
            *color.a_mut() = f[3];
            color
        })
        .map(|color| U8Color::from(color))
        .collect::<Vec<_>>();
    vec
}

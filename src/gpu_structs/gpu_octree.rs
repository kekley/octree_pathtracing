use std::{fmt::Debug, hash::Hash, mem, u32};

use crate::{
    ray_tracing::resource_manager::ResourceModel,
    voxels::octree::{Octant, Octree},
};
use bytemuck::{Pod, Zeroable};
use glam::{UVec3, Vec3, Vec4};
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor, DownlevelFlags,
    PipelineLayoutDescriptor, PollType, RequestAdapterOptions, ShaderStages,
};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable, Default, Debug)]
pub struct TraversalContext {
    pub octree_scale: f32,
    pub root: u32,
    pub scale: u32,
    pub octant_stack: [u32; 23 + 1],
    pub time_stack: [f32; 23 + 1],
    pub padding: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeNode {
    header: [u32; 4],
    indices: [u32; 8],
}

impl From<&Octant<ResourceModel>> for GPUOctreeNode {
    fn from(value: &Octant<ResourceModel>) -> Self {
        let Octant {
            parent,
            child_count,
            children,
        } = value;

        let mut header = [0u32; 4];
        let mut indices = [0u32; 8];
        children
            .iter()
            .enumerate()
            .for_each(|(i, child)| match child {
                crate::voxels::octree::Child::None => {}
                crate::voxels::octree::Child::Octant(ind) => {
                    header[i / 2] |= if i & 1 != 0 {
                        0b00000000111111110000000000000000
                    } else {
                        0b00000000000000000000000011111111
                    };
                    indices[i] = *ind;
                }
                crate::voxels::octree::Child::Leaf(val) => {
                    header[i / 2] |= if i & 1 != 0 {
                        0b11111111111111110000000000000000
                    } else {
                        0b00000000000000001111111111111111
                    };
                    indices[i] = val.get_first_index();
                }
            });

        GPUOctreeNode { header, indices }
    }
}

pub struct GPUOctree {
    pub octree_scale: f32,
    pub octants: Vec<GPUOctreeNode>,
    pub depth: u8,
}

impl From<&Octree<ResourceModel>> for GPUOctree {
    fn from(value: &Octree<ResourceModel>) -> Self {
        let vec = value
            .octants
            .iter()
            .map(|octant| GPUOctreeNode::from(octant))
            .collect::<Vec<_>>();

        GPUOctree {
            octree_scale: value.octree_scale,
            octants: vec,
            depth: value.depth,
        }
    }
}

#[test]
fn hello_compute() -> anyhow::Result<()> {
    let data: GPUOctreeNode = GPUOctreeNode {
        header: Default::default(),
        indices: Default::default(),
    };
    let a = [data; 64];
    env_logger::init();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions::default()))?;

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

    let module =
        device.create_shader_module(wgpu::include_wgsl!("../.././assets/shaders/svo.wgsl"));

    let input_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&a),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output = device.create_buffer(&BufferDescriptor {
        label: Some("output"),
        size: (size_of::<Vec4>() * 64 * 64) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("output"),
        size: (size_of::<Vec4>() * 64 * 64) as u64,
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
                resource: input_data_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
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
        compute_pass.dispatch_workgroups(64, 64, 1);
    }

    command_encoder.copy_buffer_to_buffer(&output, 0, &staging_buffer, 0, output.size());

    queue.submit(Some(command_encoder.finish()));

    let a = staging_buffer.slice(..);
    a.map_async(wgpu::MapMode::Read, move |r| r.unwrap());
    device.poll(PollType::wait()).unwrap();
    let mut local_buffer = [[0f32; 4]; 64 * 64];
    {
        let view = a.get_mapped_range();
        local_buffer.copy_from_slice(bytemuck::cast_slice(&view));
    }

    staging_buffer.unmap();

    for f in local_buffer {
        println!("{:?}", f);
    }
    Ok(())
}

use std::{fmt::Debug, hash::Hash, mem, u32};

use crate::ray_tracing::resource_manager::ResourceModel;
use bytemuck::{Pod, Zeroable};
use glam::UVec3;
use wgpu::{util::DeviceExt, DownlevelFlags, RequestAdapterOptions};
pub type OctantId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LeafId {
    pub parent: OctantId,
    pub idx: u8,
}

pub trait Position: Copy + Clone + Debug + Sized {
    fn construct(x: u32, y: u32, z: u32) -> Self;
    fn idx(&self) -> u8;
    fn required_depth(&self) -> u8;
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn z(&self) -> u32;
    fn div(&self, rhs: u32) -> Self;
    fn rem_assign(&mut self, rhs: u32);
}

impl Position for UVec3 {
    fn idx(&self) -> u8 {
        let val = (self.x + self.y * 2 + self.z * 4) as u8;
        val
    }
    fn required_depth(&self) -> u8 {
        let depth = self.max_element();
        (depth as f32).log2().floor() as u8 + 1
    }

    fn construct(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    fn x(&self) -> u32 {
        self.x
    }

    fn y(&self) -> u32 {
        self.y
    }

    fn z(&self) -> u32 {
        self.z
    }

    fn div(&self, rhs: u32) -> Self {
        *self / rhs
    }

    fn rem_assign(&mut self, rhs: u32) {
        *self %= rhs;
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GPUOctreeHeader {
    mask_1_0: u32, // [00000000 00000000]leaf mask,child_mask [00000000 00000000]leaf mask,child_mask
    mask_3_2: u32,
    mask_5_4: u32,
    mask_7_6: u32,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeNode {
    header: GPUOctreeHeader,
    indices: [u32; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUModelIndices {
    starting_index: u32,
    len: u32,
}

pub struct GPUOctree {
    pub octree_scale: f32,
    pub octants: Vec<GPUOctreeNode>,
    pub free_list: Vec<OctantId>,
    pub depth: u8,
}

#[test]
fn compute() -> anyhow::Result<()> {
    const DATA: GPUOctreeNode = GPUOctreeNode {
        header: GPUOctreeHeader {
            mask_1_0: u32::MAX,
            mask_3_2: 0,
            mask_5_4: u32::MAX,
            mask_7_6: 0,
        },
        indices: [1u32; 8],
    };
    let a = [DATA; 64];
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
        usage: wgpu::BufferUsages::UNIFORM,
    });

    todo!()
}

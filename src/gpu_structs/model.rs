use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ModelFlags(u32);

bitflags! {
   impl ModelFlags:u32{
        const SIMPLE_AABB = 0b0000_0000_0000_0000_0000_0000_0000_0001;
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Model {
    model_flags: ModelFlags,
    cuboid_start_index: u32,
    length: u32,
    padding: u32,
}

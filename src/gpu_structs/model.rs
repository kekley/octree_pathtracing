use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Model {}

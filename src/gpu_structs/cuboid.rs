use bytemuck::{Pod, Zeroable};
use glam::Mat4;

///we need a flag to label this a simple AABB,flags to denote a face as not rendered, and texture uvs per face
#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Cuboid {
    inv_matrix: Mat4,
    material_ids: [u32; 6],
    pad1: u32,
    pad2: u32,
    pad3: u32,
    pad4: u32,
    pad5: u32,
    pad6: u32,
}

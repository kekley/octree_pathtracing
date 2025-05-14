use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUQuad {
    pub origin: [f32; 4],
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub u_v_range: [f32; 4],
}

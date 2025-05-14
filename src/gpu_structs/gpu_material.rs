use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUMaterial {
    ior: f32,
    specular: f32,
    emittance: f32,
    roughness: f32,
    metalness: f32,
    texture_index: u32,
    padding: u64,
}

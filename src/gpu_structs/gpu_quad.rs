use bytemuck::{Pod, Zeroable};

use crate::geometry::quad::Quad;

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUQuad {
    pub origin: [f32; 3],
    pub material_id: u32,
    pub u: [f32; 3],
    pub tex_u: u32, //two f16s
    pub v: [f32; 3],
    pub tex_v: u32, //two f16s
    pub normal: [f32; 3],
    pub d: f32,
}

impl From<&Quad> for GPUQuad {
    fn from(value: &Quad) -> Self {
        let Quad {
            origin,
            normal,
            material_id,
            v,
            u,
            w,
            d,
            texture_u_range,
            texture_v_range,
        } = value;

        let (u0, u1) = (
            (texture_u_range.x * 65535.0) as u16,
            (texture_u_range.y * 65535.0) as u16,
        );
        let (v0, v1) = (
            (texture_v_range.x * 65535.0) as u16,
            (texture_v_range.y * 65535.0) as u16,
        );

        GPUQuad {
            origin: [origin.x, origin.y, origin.z],
            u: [u.x, u.y, u.z],
            v: [v.x, v.y, v.z],
            tex_u: (u0 as u32) << 16 | u1 as u32,
            tex_v: (v0 as u32) << 16 | v1 as u32,
            normal: [normal.x, normal.y, normal.z],
            material_id: *material_id,
            d: *d,
        }
    }
}

use bytemuck::{Pod, Zeroable};

use crate::textures::material::Material;

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GPUMaterial {
    ior: f32,
    specular: f32,
    emittance: f32,
    roughness: f32,
    metalness: f32,
    texture_index: u32,
    tint_index: u32,
    flags: u32,
}

impl GPUMaterial {
    pub fn from_material(material: &Material, texture_ind: u32) -> GPUMaterial {
        let Material {
            index_of_refraction,
            material_flags,
            specular,
            emittance,
            roughness,
            metalness,
            texture: _,
            tint_index,
        } = material;

        GPUMaterial {
            ior: *index_of_refraction,
            specular: *specular,
            emittance: *emittance,
            roughness: *roughness,
            metalness: *metalness,
            texture_index: texture_ind,
            tint_index: *tint_index,
            flags: material_flags.bits(),
        }
    }
}

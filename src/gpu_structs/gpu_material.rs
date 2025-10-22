use std::hash::Hash;

use bytemuck::{Pod, Zeroable};

use crate::textures::material::Material;

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable, PartialEq)]
pub struct GPUMaterial {
    pub(crate) ior: f32,
    pub(crate) specular: f32,
    pub(crate) emittance: f32,
    pub(crate) roughness: f32,
    pub(crate) metalness: f32,
    pub(crate) texture_index: u32,
    pub(crate) tint_index: u32,
    pub(crate) flags: u32,
}

impl Eq for GPUMaterial {}

impl Hash for GPUMaterial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ior.to_bits().hash(state);
        self.specular.to_bits().hash(state);
        self.emittance.to_bits().hash(state);
        self.roughness.to_bits().hash(state);
        self.metalness.to_bits().hash(state);
        self.texture_index.hash(state);
        self.tint_index.hash(state);
        self.flags.hash(state);
    }
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

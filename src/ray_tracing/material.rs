use crate::ray_tracing::texture::Texture;
use bitflags::bitflags;
use glam::Vec4;
use smol_str::SmolStr;

use super::{resource_manager::TextureID, tile_renderer::U8Color};

bitflags! {
    #[derive(Clone, Copy,Debug)]
    pub struct MaterialFlags: u32 {
        const OPAQUE = 0b00000001;
        const SUBSURFACE_SCATTER = 0b00000010;
        const REFRACTIVE = 0b00000100;
        const WATERLOGGED = 0b00001000;
        const SOLID = 0b00010000;
    }
}

impl Default for MaterialFlags {
    fn default() -> Self {
        Self::OPAQUE | Self::SOLID
    }
}

pub struct MaterialBuilder {
    index_of_refraction: Option<f32>,
    material_flags: Option<MaterialFlags>,
    specular: Option<f32>,
    emittance: Option<f32>,
    roughness: Option<f32>,
    metalness: Option<f32>,
    texture: Option<Texture>,
}

impl MaterialBuilder {
    pub fn build(self) -> Material {
        pub const DEFAULT_IOR: f32 = 1.000293;

        Material {
            index_of_refraction: self.index_of_refraction.unwrap_or(DEFAULT_IOR),
            material_flags: self.material_flags.unwrap_or(MaterialFlags::default()),
            specular: self.specular.unwrap_or(0.0),
            emittance: self.emittance.unwrap_or(0.0),
            roughness: self.roughness.unwrap_or(0.0),
            metalness: self.metalness.unwrap_or(0.0),
            texture: self.texture.unwrap_or(Texture::DEFAULT_TEXTURE),
        }
    }
    pub fn index_of_refraction(self, ior: f32) -> Self {
        Self {
            index_of_refraction: Some(ior),
            ..self
        }
    }
    pub fn specular(self, specular: f32) -> Self {
        Self {
            specular: Some(specular),
            ..self
        }
    }
    pub fn emittance(self, emittance: f32) -> Self {
        Self {
            emittance: Some(emittance),
            ..self
        }
    }
    pub fn roughness(self, roughness: f32) -> Self {
        Self {
            roughness: Some(roughness),
            ..self
        }
    }
    pub fn metalness(self, metalness: f32) -> Self {
        Self {
            metalness: Some(metalness),
            ..self
        }
    }
    pub fn albedo(self, texture: Texture) -> Self {
        Self {
            texture: Some(texture),
            ..self
        }
    }
    pub fn material_flags(self, material_flags: MaterialFlags) -> Self {
        Self {
            material_flags: Some(material_flags),
            ..self
        }
    }
}

#[derive(Debug, Clone)]
pub struct Material {
    pub index_of_refraction: f32,
    pub material_flags: MaterialFlags,
    pub specular: f32,
    pub emittance: f32,
    pub roughness: f32,
    pub metalness: f32,
    pub texture: Texture,
}

impl Material {
    pub const AIR: Material = Material {
        index_of_refraction: 1.000293,
        material_flags: MaterialFlags::empty(),
        specular: 0.0,
        emittance: 0.0,
        roughness: 0.0,
        metalness: 0.0,
        texture: Texture::DEFAULT_TEXTURE,
    };
    pub fn new() -> MaterialBuilder {
        MaterialBuilder {
            index_of_refraction: Some(1.000293),
            material_flags: None,
            specular: None,
            emittance: None,
            roughness: None,
            metalness: None,
            texture: None,
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Material::new().build()
    }
}

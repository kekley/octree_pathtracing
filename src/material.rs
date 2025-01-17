use crate::{ray::Ray, texture::Texture};
use bitflags::bitflags;
use glam::Vec3A;
use smol_str::SmolStr;

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

#[derive(Debug, Default)]
pub struct Scatter {
    pub ray: Ray,
    pub color: Vec3A,
}

impl Scatter {
    pub fn new(ray: Ray, color: Vec3A) -> Self {
        Self { ray, color }
    }
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: SmolStr,
    pub index_of_refraction: f32,
    pub material_flags: MaterialFlags,
    pub specular: f32,
    pub emittance: f32,
    pub roughness: f32,
    pub metalness: f32,
    pub albedo: Texture,
}

impl Material {
    const DEFAULT_IOR: f32 = 1.000293;
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "default".into(),
            index_of_refraction: Self::DEFAULT_IOR,
            specular: 0.0,
            emittance: 0.0,
            roughness: 0.0,
            metalness: 0.0,
            material_flags: MaterialFlags::OPAQUE | MaterialFlags::SOLID,
            albedo: Texture::DEFAULT_TEXTURE,
        }
    }
}

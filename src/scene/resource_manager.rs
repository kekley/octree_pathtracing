use std::sync::Arc;

use glam::{Affine3A, Mat4, Vec3, Vec3A};
use lasso::Rodeo;
use spider_eye::{
    blockstate::borrow::BlockState,
    interned::{block_model::InternedBlockModel, blockstate::InternedVariantType},
    resource_loader::LoadedResources,
    serde::block_model::FaceName,
};

use crate::{
    geometry::quad::Quad,
    gpu_structs::{cuboid::Cuboid, gpu_material::GPUMaterial, model::Model},
    textures::texture::Texture,
};

pub type TextureID = u32;
pub type CuboidID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

#[derive(Default)]
pub struct ModelBuilder {
    cuboids: Vec<Cuboid>,
    matrices: Vec<Mat4>,
    materials: Vec<GPUMaterial>,
    textures: Vec<Texture>,
}

impl ModelBuilder {
    pub fn generate_model_from_blockmodel(block_model: InternedBlockModel, rodeo: &Rodeo) {
        if let Some(parent_location) = block_model.get_parent_location() {}
    }
}

pub enum ModelType {
    Simple,
    Complex,
}

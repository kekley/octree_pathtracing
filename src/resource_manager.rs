use std::{collections::HashMap, sync::Arc};

use fxhash::{FxBuildHasher, FxHashMap};
use spider_eye::{block_models::BlockModel, variant::Variant};

use crate::{Material, QuadModel, RTWImage, SingleBlockModel, Texture};

pub type ModelID = u32;

pub struct ResourceManager {
    textures: Vec<Texture>,
    materials: FxHashMap<String, Material>,
    models: Vec<ResourceModel>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            textures: vec![],
            materials: HashMap::with_hasher(FxBuildHasher::default()),
            models: vec![],
        }
    }

    pub fn load_model(&mut self, path: &str) -> ModelID {
        todo!()
    }

    pub fn get_model(&self, model_id: ModelID) -> &ResourceModel {
        todo!()
    }
}

pub enum ResourceModel {
    SingleBlock(SingleBlockModel),
    Quad(QuadModel),
}
pub enum ModelType {
    SingleAABB,
    Quads,
}

impl ResourceModel {
    pub fn from_json(path: &str) -> Self {
        todo!()
    }
}

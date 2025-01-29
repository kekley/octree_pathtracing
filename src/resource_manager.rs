use std::{array, collections::HashMap, fs, sync::Arc};

use fxhash::{FxBuildHasher, FxHashMap};
use hashbrown::HashMap;
use rayon::array;
use smol_str::SmolStr;
use spider_eye::{
    block_element::{self, ElementRotation},
    block_models::{BlockModel, BlockRotation},
    block_states::BlockResource,
    resource_loader::ResourceLoader,
    variant::Variant,
};

use crate::{quad, texture, Material, Quad, QuadModel, RTWImage, SingleBlockModel, Texture};

pub type ModelID = u32;

pub struct ResourceManager {
    resource_loader: ResourceLoader,
    materials: FxHashMap<SmolStr, Material>,
    models: Vec<ResourceModel>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            materials: HashMap::with_hasher(FxBuildHasher::default()),
            models: vec![],
            resource_loader: ResourceLoader::new(),
        }
    }

    fn get_model_type(model: &BlockModel) -> ModelType {
        if model.is_cube() {
            ModelType::SingleAABB
        } else {
            ModelType::Quads
        }
    }
    pub fn load_model(&mut self, path: &str) -> ModelID {
        let block_model = self.resource_loader.load_model(path);
        let resource = self.build_resource(&block_model);
        resource
    }

    fn build_resource(&mut self, block_model: &BlockModel) -> ModelID {
        let textures = block_model.get_textures();
        let materials = textures
            .iter()
            .map(|texture| {
                let material = if self.materials.contains_key(texture.1) {
                    self.materials.get(texture.1).unwrap().clone()
                } else {
                    let image = RTWImage::load(texture.1).unwrap();
                    Material::new(texture.1.clone())
                        .albedo(Texture::Image(image))
                        .build()
                };
                (texture.0.clone(), material)
            })
            .collect::<HashMap<_, _>>();

        let model = match Self::get_model_type(block_model) {
            ModelType::SingleAABB => {
                let block_element = &block_model.elements[0];
                let mut block_materials: [Material; 6] = array::from_fn(|_| Material::default());
                block_element.faces.iter().for_each(|face| {
                    let face = face.as_ref().expect("single block model was missing face");
                    let ind: u32 = face.name as u32;
                    let material = materials
                        .get(face.texture.get())
                        .expect("texture variable was not in material hashmap");
                    block_materials[ind as usize] = material.clone();
                });
                let block_model = SingleBlockModel {
                    materials: block_materials,
                };
                ResourceModel::SingleBlock(block_model)
            }
            ModelType::Quads => {
                let mut quads = vec![];
                block_model.elements.iter().for_each(|element| {
                    element.faces.iter().filter_map(|face| {
                        let face = face.as_ref()?;

                        let material = materials
                            .get(face.texture.get())
                            .expect("texture variable was not in materials hashmap");

                        None
                    });
                });
                todo!()
            }
        };
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

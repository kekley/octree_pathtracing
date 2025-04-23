use std::{array, env::var};

use aovec::Aovec;
use dashmap::DashMap;
use fxhash::FxBuildHasher;
use glam::{Vec3, Vec3A, Vec4};
use hashbrown::{HashMap, HashSet};
use smol_str::SmolStr;
use spider_eye::{
    block::InternedBlock, block_face::FaceName, block_models::InternedBlockModel,
    block_states::InternedBlockState, block_texture::InternedTextureVariable,
    resource::BlockStates, resource_loader::MCLoader, variant::ModelVariant,
};

use crate::{
    ray_tracing::{
        models::{QuadModel, SingleBlockModel},
        quad::Quad,
    },
    voxels::octree_traversal::OctreeIntersectResult,
    RTWImage,
};

use super::{material::Material, ray::Ray, texture::Texture};

pub type ModelID = u32;
pub struct ModelManager {
    pub resource_loader: MCLoader,
    materials: DashMap<SmolStr, Material, FxBuildHasher>,
    models: Aovec<ResourceModel>,
    seen_blocks: DashMap<InternedBlock, u32, FxBuildHasher>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self {
            resource_loader: Default::default(),
            materials: Default::default(),
            models: Aovec::new(16),
            seen_blocks: Default::default(),
        }
    }
}

impl ModelManager {
    pub fn load_resource(&self, block: &InternedBlock) -> ModelID {
        if !self.seen_blocks.contains_key(block) {
            println!("len: {}", self.seen_blocks.len());
            let rodeo = &self.resource_loader.rodeo;
            let resolved_block = block.resolve(rodeo);
            dbg!(&resolved_block);
            let model = self.resource_loader.load_models(block);
            dbg!(&resolved_block);
            let id = self.build_variant(model);
            self.seen_blocks.insert(block.clone(), id);

            id
        } else {
            let id = self.seen_blocks.get(block);
            return id.unwrap().clone();
        }
    }
    pub fn new(resource_loader: &MCLoader) -> Self {
        let seen_blocks = DashMap::with_hasher(FxBuildHasher::default());
        let materials = DashMap::with_hasher(FxBuildHasher::default());

        materials.insert(SmolStr::from("air"), Material::AIR);
        Self {
            seen_blocks,
            materials,
            models: Aovec::new(16),
            resource_loader: resource_loader.clone(),
        }
    }

    pub fn build_variant(&self, variants: Vec<ModelVariant>) -> ModelID {
        assert!(variants.len() > 0);
        if variants.len() == 1 {
            match &variants[0] {
                ModelVariant::SingleModel(variant_entry) => {
                    let model = &variant_entry.model;

                    let resource = if model.is_cube() {
                        self.build_model(&model, ModelType::SingleAABB)
                    } else {
                        self.build_model(&model, ModelType::Quads)
                    };
                    let model_id = self.models.len() as u32;
                    self.models.push(resource);
                    return model_id;
                }
                ModelVariant::ModelArray(items) => {
                    let model = &items[0].model;
                    let resource = if model.is_cube() {
                        self.build_model(&model, ModelType::SingleAABB)
                    } else {
                        self.build_model(&model, ModelType::Quads)
                    };
                    let model_id = self.models.len() as u32;
                    self.models.push(resource);
                    return model_id;
                }
            }
        } else {
            let quads = variants
                .iter()
                .flat_map(|variant| match variant {
                    ModelVariant::SingleModel(variant_entry) => {
                        let model = &variant_entry.model;

                        let resource = self.build_model(&model, ModelType::Quads);
                        resource.take_quads()
                    }
                    ModelVariant::ModelArray(items) => {
                        //FIXME: randomly choose model
                        let model = &items[0].model;
                        let resource = self.build_model(&model, ModelType::Quads);
                        resource.take_quads()
                    }
                })
                .collect::<Vec<_>>();
            let resource = ResourceModel::Quad(QuadModel { quads });
            let model_id = self.models.len() as u32;
            self.models.push(resource);
            return model_id;
        }
    }

    fn build_model(
        &self,
        block_model: &InternedBlockModel,
        model_type: ModelType,
    ) -> ResourceModel {
        let textures = block_model.get_textures();

        let materials = textures
            .iter()
            .map(|texture| {
                let mut current_texture_var = &texture.1;
                while let InternedTextureVariable::Variable(var) = current_texture_var {
                    //dbg!(self.resource_loader.resolve_spur(var));
                    current_texture_var = &textures.iter().find(|tex_2| tex_2.0 == *var).unwrap().1;
                }
                let material = if self.materials.contains_key(
                    self.resource_loader
                        .resolve_spur(&current_texture_var.get_inner()),
                ) {
                    self.materials
                        .get(
                            self.resource_loader
                                .resolve_spur(&current_texture_var.get_inner()),
                        )
                        .unwrap()
                        .clone()
                } else {
                    let a = self.resource_loader.resolve_spur(&texture.0);
                    //dbg!(a);
                    let tex_path = self.resource_loader.get_texture_path(
                        self.resource_loader
                            .resolve_spur(&current_texture_var.get_inner()),
                    );
                    //dbg!(&tex_path);
                    let image = RTWImage::load(&tex_path).unwrap();
                    Material::new(
                        self.resource_loader
                            .resolve_spur(&current_texture_var.get_inner())
                            .into(),
                    )
                    .albedo(Texture::Image(image))
                    .build()
                };
                (texture.0.clone(), material)
            })
            .collect::<HashMap<_, _>>();

        let model = match model_type {
            ModelType::SingleAABB => {
                dbg!("single block");
                let block_element = &block_model.elements[0];
                let mut block_materials: [Material; 6] = array::from_fn(|_| Material::default());
                block_element.faces.iter().for_each(|face| {
                    let face = face.as_ref().expect("single block model was missing face");
                    let ind: u32 = face.name as u32;
                    let a = self.resource_loader.resolve_spur(&face.texture.get_inner());
                    //dbg!(a);
                    let material = materials
                        .get(&face.texture.get_inner())
                        .expect("texture variable was not in material hashmap");
                    block_materials[ind as usize] = material.clone();
                });
                let block_model = SingleBlockModel {
                    materials: block_materials,
                };
                ResourceModel::SingleBlock(block_model)
            }
            ModelType::Quads => {
                dbg!("quads");

                let quads = block_model
                    .elements
                    .iter()
                    .flat_map(|element| {
                        element.faces.iter().filter_map(|face| {
                            let element_rotation = element.rotation.as_ref().map(|f| f.to_matrix());
                            let face = face.as_ref()?;
                            let (v0, v1, v2) =
                                get_quad_coordinates(&element.from, &element.to, face.name);
                            let uv = if let Some(uv) = &face.uv {
                                uv.to_vec4()
                            } else {
                                get_face_coordinates(&element.from, &element.to, face.name)
                            };
                            let material = materials
                                .get(&face.texture.get_inner())
                                .expect("texture variable was not in materials hashmap");
                            let mut quad = Quad::new(v0, v1, v2, uv, material.clone());
                            if let Some(matrix) = element_rotation {
                                quad.transform(&matrix);
                            }
                            Some(quad)
                        })
                    })
                    .collect::<Vec<_>>();
                let model = QuadModel { quads };
                ResourceModel::Quad(model)
            }
        };
        model
    }

    pub fn get_model(&self, model_id: ModelID) -> &ResourceModel {
        &self.models[model_id as usize]
    }
}

fn get_quad_coordinates(from: &Vec3, to: &Vec3, face: FaceName) -> (Vec3A, Vec3A, Vec3A) {
    match face {
        FaceName::Down => (
            Vec3A::new(from.x, from.y, from.z),
            Vec3A::new(to.x, from.y, from.z),
            Vec3A::new(from.x, from.y, to.z),
        ),
        FaceName::Up => (
            Vec3A::new(to.x, to.y, from.z),
            Vec3A::new(from.x, to.y, from.z),
            Vec3A::new(to.x, to.y, to.z),
        ),
        FaceName::North => (
            Vec3A::new(to.x, from.y, from.z),
            Vec3A::new(from.x, from.y, from.z),
            Vec3A::new(to.x, to.y, from.z),
        ),
        FaceName::South => (
            Vec3A::new(from.x, from.y, to.z),
            Vec3A::new(to.x, from.y, to.z),
            Vec3A::new(from.x, to.y, to.z),
        ),
        FaceName::West => (
            Vec3A::new(from.x, from.y, from.z),
            Vec3A::new(from.x, from.y, to.z),
            Vec3A::new(from.x, to.y, from.z),
        ),
        FaceName::East => (
            Vec3A::new(to.x, from.y, to.z),
            Vec3A::new(to.x, from.y, from.z),
            Vec3A::new(to.x, to.y, to.z),
        ),
    }
}

fn get_face_coordinates(from: &Vec3, to: &Vec3, face: FaceName) -> Vec4 {
    match face {
        FaceName::Down => Vec4::new(from.x, to.x, from.z, to.z),
        FaceName::Up => Vec4::new(from.x, to.x, to.z, from.z),
        FaceName::North => Vec4::new(from.x, to.x, from.y, to.y),
        FaceName::South => Vec4::new(to.x, from.x, from.y, to.y),
        FaceName::West => Vec4::new(from.z, to.z, from.y, to.y),
        FaceName::East => Vec4::new(to.z, from.z, from.y, to.y),
    }
}

#[derive(Debug, Clone)]
pub enum ResourceModel {
    SingleBlock(SingleBlockModel),
    Quad(QuadModel),
}
pub enum ModelType {
    SingleAABB,
    Quads,
}

impl ResourceModel {
    pub(crate) fn take_quads(self) -> Vec<Quad> {
        match self {
            ResourceModel::SingleBlock(single_block_model) => panic!(),
            ResourceModel::Quad(quad_model) => quad_model.quads,
        }
    }
    pub fn intersect(&self, ray: &mut Ray, octree_result: &OctreeIntersectResult<u32>) -> bool {
        match self {
            ResourceModel::SingleBlock(single_block_model) => {
                single_block_model.intersect(&octree_result, ray)
            }
            ResourceModel::Quad(quad_model) => quad_model.intersect(ray),
        }
    }
}

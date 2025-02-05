use std::array;

use fxhash::{FxBuildHasher, FxHashMap};
use glam::{Vec3, Vec3A, Vec4};
use hashbrown::HashMap;
use smol_str::{SmolStr, SmolStrBuilder};
use spider_eye::{
    block::Block,
    block_element::{self, ElementRotation},
    block_face::FaceName,
    block_models::{BlockModel, BlockRotation},
    block_texture::Uv,
    loaded_world::BlockName,
    resource::BlockStates,
    resource_loader::ResourceLoader,
    variant::{ModelVariant, Variants},
};

use crate::{
    octree_traversal::OctreeIntersectResult, quad, texture, Face, Material, Quad, QuadModel,
    RTWImage, Ray, SingleBlockModel, Texture,
};

pub type ModelID = u32;
#[derive(Debug, Clone)]
pub struct ResourceManager {
    pub resource_loader: ResourceLoader,
    materials: HashMap<SmolStr, Material, FxBuildHasher>,
    models: Vec<ResourceModel>,
    cache: HashMap<BlockName, BlockStates, FxBuildHasher>,
}

impl ResourceManager {
    pub fn load_resource(&mut self, block: Block) -> ModelID {
        let rodeo = &self.resource_loader.rodeo;
        let block_states = if let Some(cached) = self.cache.get(&block.block_name) {
            cached
        } else {
            dbg!(rodeo.resolve(&block.block_name));
            let path = "./test_assets/assets/minecraft/blockstates/".to_string()
                + rodeo
                    .resolve(&block.block_name)
                    .strip_prefix("minecraft:")
                    .unwrap()
                + ".json";
            &BlockStates::new(&path, rodeo)
        };
        let model: Vec<ModelVariant> = match block_states {
            BlockStates::MultiPart(multi_part) => multi_part.get(&block.block_state),
            BlockStates::Variants(variants) => variants.get(&block.block_state),
        };
        let id = self.build_variant(model);
        id
    }
    pub fn new(resource_loader: &ResourceLoader) -> Self {
        let mut materials = HashMap::with_hasher(FxBuildHasher::default());

        materials.insert(SmolStr::new("air"), Material::AIR);
        Self {
            materials,
            models: vec![],
            resource_loader: resource_loader.clone(),
            cache: HashMap::with_hasher(FxBuildHasher::default()),
        }
    }
    fn get_material(&self, mat: &SmolStr) -> &Material {
        self.materials.get(mat).expect("")
    }

    pub fn build_variant(&mut self, variants: Vec<ModelVariant>) -> ModelID {
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

    fn build_model(&mut self, block_model: &BlockModel, model_type: ModelType) -> ResourceModel {
        let textures = block_model.get_textures();
        let materials = textures
            .iter()
            .map(|texture| {
                let material = if self
                    .materials
                    .contains_key(self.resource_loader.resolve_spur(&texture.1.get_inner()))
                {
                    self.materials
                        .get(self.resource_loader.resolve_spur(&texture.1.get_inner()))
                        .unwrap()
                        .clone()
                } else {
                    let tex_path = self.resource_loader.get_texture_path(
                        self.resource_loader.resolve_spur(&texture.1.get_inner()),
                    );
                    dbg!(&tex_path);
                    let image = RTWImage::load(&tex_path).unwrap();
                    Material::new(
                        self.resource_loader
                            .resolve_spur(&texture.1.get_inner())
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
                let block_element = &block_model.elements[0];
                let mut block_materials: [Material; 6] = array::from_fn(|_| Material::default());
                block_element.faces.iter().for_each(|face| {
                    let face = face.as_ref().expect("single block model was missing face");
                    let ind: u32 = face.name as u32;
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

use std::array;

use fxhash::{FxBuildHasher, FxHashMap};
use glam::{Vec3, Vec3A, Vec4};
use hashbrown::HashMap;
use smol_str::SmolStr;
use spider_eye::{
    block_element::{self, ElementRotation},
    block_face::FaceName,
    block_models::{BlockModel, BlockRotation},
    block_states::BlockResource,
    block_texture::Uv,
    resource_loader::ResourceLoader,
    variant::Variant,
};

use crate::{
    octree_traversal::OctreeIntersectResult, quad, texture, Face, Material, Quad, QuadModel,
    RTWImage, Ray, SingleBlockModel, Texture,
};

pub type ModelID = u32;

#[derive(Debug, Clone)]
pub struct ResourceManager {
    resource_loader: ResourceLoader,
    materials: HashMap<SmolStr, Material, FxBuildHasher>,
    models: Vec<ResourceModel>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let mut materials = HashMap::with_hasher(FxBuildHasher::default());

        materials.insert(SmolStr::new("air"), Material::AIR);
        Self {
            materials,
            models: vec![],
            resource_loader: ResourceLoader::new(),
        }
    }
    fn get_material(&self, mat: &SmolStr) -> &Material {
        self.materials.get(mat).expect("")
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
                                .get(face.texture.get())
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
        let model_id = self.models.len() as u32;
        self.models.push(model);
        model_id
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
    pub fn intersect(&self, ray: &mut Ray, octree_result: &OctreeIntersectResult<u32>) -> bool {
        match self {
            ResourceModel::SingleBlock(single_block_model) => {
                single_block_model.intersect(&octree_result, ray)
            }
            ResourceModel::Quad(quad_model) => quad_model.intersect(ray),
        }
    }
}

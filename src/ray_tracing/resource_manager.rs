use std::{array, env::var, num::NonZeroU32, sync::Arc};

use aovec::Aovec;
use dashmap::{mapref::one::Ref, DashMap, RwLock};
use fxhash::FxBuildHasher;
use glam::{Vec3, Vec3A, Vec4, Vec4Swizzles};
use hashbrown::{HashMap, HashSet};
use lasso::Spur;
use smol_str::SmolStr;
use spider_eye::{
    block::InternedBlock,
    block_face::FaceName,
    block_models::{BlockRotation, InternedBlockModel},
    block_states::InternedBlockState,
    block_texture::{InternedTextureVariable, TexPath},
    loaded_world::InternedBlockName,
    resource::BlockStates,
    variant::ModelVariant,
    MCResourceLoader,
};
use wgpu::hal::auxil::db;

use crate::{
    ray_tracing::{
        models::{QuadModel, SingleBlockModel},
        quad::Quad,
    },
    voxels::octree_traversal::OctreeIntersectResult,
    RTWImage,
};

use super::{
    material::Material,
    ray::Ray,
    texture::{self, Texture},
};

pub type TextureID = u32;
pub type QuadID = u32;
pub type MaterialID = u32;

pub struct ModelManager {
    pub resource_loader: MCResourceLoader,
    pub(crate) materials: Arc<RwLock<Vec<Material>>>,
    pub(crate) quads: Arc<RwLock<Vec<Quad>>>,
    seen_materials: DashMap<TexPath, MaterialID, FxBuildHasher>,
    seen_blocks: DashMap<InternedBlock, Option<ResourceModel>, FxBuildHasher>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self {
            resource_loader: MCResourceLoader::new(),
            materials: Default::default(),
            quads: Default::default(),
            seen_materials: Default::default(),
            seen_blocks: Default::default(),
        }
    }
}

impl ModelManager {
    pub fn load_resource(&self, block: &InternedBlock) -> Option<ResourceModel> {
        if let Some(model) = self.seen_blocks.get(block) {
            return model.clone();
        } else {
            let block_states = self
                .resource_loader
                .load_block_states_interned(block.block_name);
            if let Some(states) = block_states {
                let model_variants = self.resource_loader.load_variants_for(block, states);
                let resource = self.build_variant(model_variants);
                if let Some(resource) = resource {
                    self.seen_blocks.insert(block.clone(), Some(resource));
                    return Some(resource);
                }
            }
            self.seen_blocks.insert(block.clone(), None);
            return None;
        }
    }
    pub fn new() -> Self {
        let tmp = Self {
            resource_loader: MCResourceLoader::new(),
            ..Default::default()
        };

        tmp.materials.write().push(Material::AIR);
        tmp
    }

    pub fn build_variant(&self, variants: Vec<ModelVariant>) -> Option<ResourceModel> {
        if variants.len() == 0 {
            return None;
        }
        if variants.len() == 1 {
            match &variants[0] {
                ModelVariant::SingleModel(variant_entry) => {
                    let model = &variant_entry.model;

                    let resource = if model.is_cube() {
                        self.build_model(
                            &model,
                            ModelType::SingleAABB,
                            variant_entry.rotation_x,
                            variant_entry.rotation_y,
                        )
                    } else {
                        self.build_model(
                            &model,
                            ModelType::Quads,
                            variant_entry.rotation_x,
                            variant_entry.rotation_y,
                        )
                    };
                    resource
                }
                ModelVariant::ModelArray(items) => {
                    let variant = &items[0];
                    let model = &variant.model;
                    let resource = if model.is_cube() {
                        self.build_model(
                            &model,
                            ModelType::SingleAABB,
                            variant.rotation_x,
                            variant.rotation_y,
                        )
                    } else {
                        self.build_model(
                            &model,
                            ModelType::Quads,
                            variant.rotation_x,
                            variant.rotation_y,
                        )
                    };
                    resource
                }
            }
        } else {
            let quads = variants
                .iter()
                .flat_map(|variant| match variant {
                    ModelVariant::SingleModel(variant_entry) => {
                        let model = &variant_entry.model;

                        let quads = self.make_quads(
                            model,
                            variant_entry.rotation_x,
                            variant_entry.rotation_y,
                        );
                        quads
                    }
                    ModelVariant::ModelArray(items) => {
                        //FIXME: randomly choose model
                        let variant = &items[0];
                        let model = &variant.model;

                        let quads = self.make_quads(model, variant.rotation_x, variant.rotation_y);
                        quads
                    }
                })
                .collect::<Vec<_>>();
            let mut quads_lock = self.quads.write();
            let starting_quad_id = quads_lock.len() as QuadID;
            let len = quads.len() as u32;
            if len == 0 {
                return None;
            }
            quads.into_iter().for_each(|quad| {
                quads_lock.push(quad);
            });

            let resource = ResourceModel::Quad(QuadModel::new(starting_quad_id, len));
            return Some(resource);
        }
    }

    fn get_materials(
        &self,
        block_model: &InternedBlockModel,
    ) -> hashbrown::HashMap<Spur, MaterialID> {
        let textures = block_model.get_textures();

        textures
            .iter()
            .map(|texture| {
                let mut current_texture_var = &texture.1;
                while let InternedTextureVariable::Variable(var) = current_texture_var {
                    //dbg!(self.resource_loader.resolve_spur(var));
                    current_texture_var = &textures.iter().find(|tex_2| tex_2.0 == *var).unwrap().1;
                }
                let material = if self
                    .seen_materials
                    .contains_key(&current_texture_var.get_inner())
                {
                    self.seen_materials
                        .get(&current_texture_var.get_inner())
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
                    let texture_image = Arc::new(RTWImage::load(&tex_path).unwrap());
                    let texture = Texture::Image(texture_image);
                    let material = Material::new().albedo(texture).build();
                    let mut materials_lock = self.materials.write();
                    let material_id = materials_lock.len() as MaterialID;
                    materials_lock.push(material);
                    material_id
                };
                (texture.0.clone(), material)
            })
            .collect::<HashMap<_, _>>()
    }
    fn make_quads(
        &self,
        block_model: &InternedBlockModel,
        rotation_x: Option<BlockRotation>,
        rotation_y: Option<BlockRotation>,
    ) -> Vec<Quad> {
        let materials = self.get_materials(block_model);
        let mut quads = block_model
            .elements
            .iter()
            .flat_map(|element| {
                element.faces.iter().filter_map(|face| {
                    let element_rotation = element.rotation.as_ref().map(|f| f.to_matrix());
                    let face = face.as_ref()?;
                    let (v0, v1, v2) = get_quad_vectors(&element.from, &element.to, face.name);

                    let uv = if let Some(uv) = &face.uv {
                        uv.to_vec4() / 16.0
                    } else {
                        Vec4::new(element.from.x, element.from.y, element.to.x, element.to.y) / 16.0
                    };
                    let x_uv = uv.xz();
                    let y_uv = uv.yw();
                    let material = materials
                        .get(&face.texture.get_inner())
                        .expect("texture variable was not in materials hashmap");
                    let mut quad = Quad::new(v0, v1, v2, x_uv, y_uv, material.clone());
                    //dbg!("{:?}", &quad);
                    if let Some(matrix) = element_rotation {
                        quad.transform(&matrix);
                    }
                    Some(quad)
                })
            })
            .collect::<Vec<_>>();
        if let Some(rotation_x) = rotation_x {
            let matrix = rotation_x.to_matrix_x();
            quads
                .iter_mut()
                .for_each(|quad| quad.transform_about_pivot(&matrix, Vec3A::splat(0.5)));
        }
        if let Some(rotation_y) = rotation_y {
            let matrix = rotation_y.to_matrix_y();
            quads
                .iter_mut()
                .for_each(|quad| quad.transform_about_pivot(&matrix, Vec3A::splat(0.5)));
        };
        quads
    }
    fn build_model(
        &self,
        block_model: &InternedBlockModel,
        model_type: ModelType,
        rotation_x: Option<BlockRotation>,
        rotation_y: Option<BlockRotation>,
    ) -> Option<ResourceModel> {
        let model = match model_type {
            ModelType::SingleAABB => {
                let materials = self.get_materials(block_model);
                let block_element = &block_model.elements[0];
                let mut block_quads: [Quad; 6] = array::from_fn(|_| Quad::default());
                block_element.faces.iter().for_each(|face| {
                    let face = face.as_ref().expect("single block model was missing face");
                    let ind: u32 = face.name as u32;
                    let a = self.resource_loader.resolve_spur(&face.texture.get_inner());
                    //dbg!(a);
                    let material = materials
                        .get(&face.texture.get_inner())
                        .expect("texture variable was not in material hashmap");
                    block_quads[ind as usize] = Quad::from_face_name(
                        &face.name,
                        &face.uv,
                        &block_element.from,
                        &block_element.to,
                        *material,
                    );
                });
                let mut quads_lock = self.quads.write();
                let first_quad_id = quads_lock.len() as QuadID;
                block_quads.into_iter().for_each(|quad| {
                    quads_lock.push(quad);
                });
                let block_model = SingleBlockModel {
                    first_quad_index: first_quad_id,
                };
                ResourceModel::SingleBlock(block_model)
            }
            ModelType::Quads => {
                let mut quads = self.make_quads(block_model, rotation_x, rotation_y);

                let quad_len = quads.len() as u32;
                if quad_len == 0 {
                    return None;
                }
                let mut quads_lock = self.quads.write();
                let first_quad_id = quads_lock.len() as QuadID;
                quads.into_iter().for_each(|quad| {
                    quads_lock.push(quad);
                });

                let model = QuadModel::new(first_quad_id, quad_len);
                ResourceModel::Quad(model)
            }
        };
        Some(model)
    }
}

fn get_quad_vectors(from: &Vec3, to: &Vec3, face: FaceName) -> (Vec3A, Vec3A, Vec3A) {
    // Convert from block space (0–16) into unit cube space (0–1).
    let from = *from / 16.0;
    let to = *to / 16.0;

    match face {
        FaceName::Down => {
            let origin = Vec3A::new(from.x, from.y, from.z);
            let u = Vec3A::new(to.x - from.x, 0.0, 0.0);
            let v = Vec3A::new(0.0, 0.0, to.z - from.z);
            (origin, u, v)
        }

        FaceName::Up => {
            let origin = Vec3A::new(to.x, to.y, from.z);
            let u = Vec3A::new(from.x - to.x, 0.0, 0.0);
            let v = Vec3A::new(0.0, 0.0, to.z - from.z);
            (origin, u, v)
        }

        // North face: normal = (0,0,-1), so the face is on plane z = from.z.
        // We set the origin at (to.x, from.y, from.z)
        // with u running west (negative X) and v running up along Y.
        FaceName::North => {
            let origin = Vec3A::new(to.x, from.y, from.z);
            let u = Vec3A::new(from.x - to.x, 0.0, 0.0); // note: negative delta in X
            let v = Vec3A::new(0.0, to.y - from.y, 0.0);
            (origin, u, v)
        }

        // South face: normal = (0,0,+1), so the face is on plane z = to.z.
        // We choose the origin at (from.x, from.y, to.z)
        // and let u run east (positive X) and v run up (positive Y).
        FaceName::South => {
            let origin = Vec3A::new(from.x, from.y, to.z);
            let u = Vec3A::new(to.x - from.x, 0.0, 0.0);
            let v = Vec3A::new(0.0, to.y - from.y, 0.0);
            (origin, u, v)
        }

        // West face: normal = (-1,0,0), so the face is on plane x = from.x.
        // We use origin = (from.x, from.y, to.z);
        // let u run up (along Y) and v run “backward” (from high z to low z).
        FaceName::West => {
            let origin = Vec3A::new(from.x, from.y, from.z);
            let u = Vec3A::new(0.0, 0.0, to.z - from.z);
            let v = Vec3A::new(0.0, to.y - from.y, 0.0);
            (origin, u, v)
        }

        // East face: normal = (+1,0,0), so it lies on plane x = to.x.
        // We choose origin = (to.x, from.y, from.z)
        // with u going up along Y and v going forward (positive Z).
        FaceName::East => {
            let origin = Vec3A::new(to.x, from.y, to.z);
            let u = Vec3A::new(0.0, 0.0, from.z - to.z);
            let v = Vec3A::new(0.0, to.y - from.y, 0.0);
            (origin, u, v)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResourceModel {
    SingleBlock(SingleBlockModel),
    Quad(QuadModel),
}

impl ResourceModel {
    pub fn get_first_index(&self) -> u32 {
        match self {
            ResourceModel::SingleBlock(single_block_model) => single_block_model.first_quad_index,
            ResourceModel::Quad(quad_model) => quad_model.starting_quad_id,
        }
    }
}
pub enum ModelType {
    SingleAABB,
    Quads,
}


use glam::{Mat4, Quat, Vec2, Vec3, Vec3A};
use hashbrown::HashMap;
use lasso::{Rodeo, Spur};
use spider_eye::{
    blockstate::borrow::BlockState,
    interned::{
        block_model::{InternedBlockModel, InternedElement},
        blockstate::{InternedModelProperties, VariantModelType},
    },
    resource_loader::LoadedResources,
};

use crate::{
    gpu_structs::{
        cuboid::{Cuboid, CuboidFlags},
        gpu_material::GPUMaterial,
        model::Model,
    },
    textures::{material::Material, texture::Texture},
};

pub type TextureID = u32;
pub type CuboidID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

pub const UNIT_BLOCK_MIN: Vec3A = Vec3A::splat(-0.5);
pub const UNIT_BLOCK_MAX: Vec3A = Vec3A::splat(0.5);

pub struct FinalizedBlockModel {
    ambient_occlusion: bool,
    textures: HashMap<Spur, Spur>,
    elements: Vec<InternedElement>,
}

impl FinalizedBlockModel {
    pub fn resolve_texture_variable_to_path<'a>(
        &self,
        texture: Spur,
        interner: &'a Rodeo,
    ) -> Option<(Spur, &'a str)> {
        let mut current_var = texture;
        //Texture variables can point to other texture variables, return when we get to the actual
        //resource path
        loop {
            let result = self.textures.get(&current_var)?;

            let resolved = interner.resolve(result);

            if resolved.starts_with("#") {
                //variable
                current_var = interner.get(resolved.strip_prefix("#").unwrap())?;
            } else {
                return Some((*result, resolved));
            }
        }
    }
    pub fn elements(&self) -> &[InternedElement] {
        &self.elements
    }
}

pub struct ModelBuilder {
    cache: HashMap<Spur, Option<FinalizedBlockModel>>,
    cuboids: Vec<Cuboid>,
    matrices: Vec<Mat4>,
    materials: Vec<GPUMaterial>,
    textures: HashMap<Spur, Texture>,
}

impl ModelBuilder {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
            cuboids: Default::default(),
            matrices: Default::default(),
            materials: Default::default(),
            textures: Default::default(),
        }
    }
    pub fn properties_to_model(
        &mut self,
        model_properties: &InternedModelProperties,
    ) -> Option<Model> {
        let model_location = model_properties.get_model_location_spur();
        todo!()
    }

    fn cache_model_and_parents(
        &mut self,
        resource_location: Spur,
        model: InternedBlockModel,
        resources: &LoadedResources,
    ) {
        if let Some(parent_location_spur) = model.get_parent_location() {
            if let Some(cached_parent) = self.cache.get(&parent_location_spur) {
                //We've seen this parent model before

                let Some(cached_parent) = cached_parent else {
                    self.cache.insert(resource_location, None);
                    return;
                };

                let FinalizedBlockModel {
                    ambient_occlusion,
                    textures,
                    elements: _,
                } = cached_parent;

                let finalized_model = FinalizedBlockModel {
                    ambient_occlusion: *ambient_occlusion,
                    textures: HashMap::<Spur, Spur>::from_iter(
                        model
                            .textures
                            .iter()
                            .map(|val| (*val.0, *val.1))
                            .chain(textures.iter().map(|val| (*val.0, *val.1))),
                    ),
                    elements: model.elements.clone(),
                };

                self.cache.insert(resource_location, Some(finalized_model));
            } else {
                //parent has not been cached
                let parent_location_str = resources.interner.resolve(&parent_location_spur);

                let Some(parent_model) = resources.get_model_data(parent_location_str) else {
                    eprintln!("model with location {parent_location_str} was not found");
                    self.cache.insert(parent_location_spur, None);
                    return;
                };

                self.cache_model_and_parents(parent_location_spur, parent_model.clone(), resources);
                self.cache_model_and_parents(resource_location, model, resources);
            }
        } else {
            //model has no parent, finalize and place in cache
            let InternedBlockModel {
                parent: _,
                ambient_occlusion,
                display: _,
                textures,
                elements,
            } = model;

            let finalized_model = FinalizedBlockModel {
                ambient_occlusion,
                textures,
                elements,
            };

            self.cache.insert(resource_location, Some(finalized_model));
        }
    }
    pub fn load_model_for_mapped_state(
        &mut self,
        block_state: BlockState<'_, '_>,
        resources: &LoadedResources,
    ) -> Option<Model> {
        let mapped_state = block_state.to_mapped_state();
        let mapped_state_str = mapped_state.as_nbt_str().to_str();

        let waterlogged = block_state.is_waterlogged();

        let block_name = block_state.get_name().to_str();

        let Some(variants) = resources.get_variant_data(&block_name) else {
            eprintln!("No variants for {block_name}");
            return None;
        };

        let Some(mapped_state_spur) = resources.interner.get(&mapped_state_str) else {
            eprintln!("No entry in interner for {mapped_state_str}");
            return None;
        };

        match variants.get_model_properties_for_mapped_state(
            mapped_state_spur,
            &mapped_state_str,
            &resources.interner,
        )? {
            VariantModelType::SingleModel(items) => {
                //TODO this would be a random model every instance of the block. might not
                //implement this

                let model_properties = items
                    .first()
                    .expect("There should always be at least one model");
                let model_location_spur = model_properties.get_model_location_spur();

                let model: &FinalizedBlockModel;

                loop {
                    if let Some(cached_model_option) = self.cache.get(&model_location_spur) {
                        model = cached_model_option.as_ref()?;
                        break;
                    } else {
                        let model_location_str = resources.interner.resolve(&model_location_spur);
                        if let Some(model) = resources.get_model_data(model_location_str) {
                            self.cache_model_and_parents(
                                model_location_spur,
                                model.clone(),
                                resources,
                            );
                        } else {
                            self.cache.insert(model_location_spur, None);
                        }
                    }
                }

                let block_x_rotation = model_properties.get_x_rotation();
                let block_y_rotation = model_properties.get_y_rotation();
                let uvlock = model_properties.get_uvlock();

                let center_matrix = Mat4::from_translation(Vec3::splat(0.5));

                let x_rotation = Mat4::from_rotation_x(block_x_rotation as f32);

                let y_rotation = Mat4::from_rotation_y(block_y_rotation as f32);

                let block_matrix = Mat4::IDENTITY * center_matrix * x_rotation * y_rotation;

                let elements = model.elements();
                assert!(!elements.is_empty());

                //TODO figure out if this model is a single AABB and therefore doesn't require a
                //matrix

                let element_count = elements.len();

                fn element_is_aabb(element: &InternedElement) -> bool {
                    element.from() == &[0.0, 0.0, 0.0]
                        && element.to() == &[16.0, 16.0, 16.0]
                        && element.faces().len() == 6
                }

                if element_count > 1 || !element_is_aabb(&elements[0]) {
                    let cuboids = elements
                        .iter()
                        .map(block_element_to_cuboid)
                        .collect::<Vec<_>>();
                } else {
                    //TODO simple AABB model case
                }
            }

            VariantModelType::Multipart(items) => {
                let model_properties = items
                    .iter()
                    .map(|slice_of_models| {
                        slice_of_models
                            .first()
                            .expect("The should always be at least one model here")
                    })
                    .collect::<Vec<_>>();
            }
        }

        todo!()
    }
}

impl Default for ModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ModelType {
    Simple,
    Complex,
}

enum ModelData {
    SimpleAABB {
        uvs: [Vec2; 12],
        materials: [Material; 6],
    },
    Cuboid {
        matrix: Option<Mat4>,
        flags: CuboidFlags,
        uvs: [Vec2; 12],
        materials: [Material; 6],
    },
}

fn block_element_to_cuboid(element: &InternedElement) -> ModelData {
    let min: Vec3A = Vec3A::from_slice(element.from());
    let max: Vec3A = Vec3A::from_slice(element.to());
    let scaled_shifted_from = (min / 16.0) - 0.5;
    let scaled_shifted_to = (max / 16.0) - 0.5;
    let translation_vector = scaled_shifted_from - UNIT_BLOCK_MIN;
    let scale_vector = scaled_shifted_to - scaled_shifted_from;

    let scale_translation_matrix = Mat4::from_scale_rotation_translation(
        scale_vector.into(),
        Quat::IDENTITY,
        translation_vector.into(),
    );

    let faces = element.faces();

    let element_rotation = element.rotation();

    let rotation_matrix = if let Some(element_rotation) = element_rotation {
        let origin = element_rotation.origin();
        let axis = element_rotation.axis();
        let angle = element_rotation.angle();
        let quat = match axis {
            spider_eye::serde::block_model::Axis::X => Quat::from_rotation_x(angle),
            spider_eye::serde::block_model::Axis::Y => Quat::from_rotation_y(angle),
            spider_eye::serde::block_model::Axis::Z => Quat::from_rotation_z(angle),
        };
        let translation = Vec3::from_slice(origin);
        Some(Mat4::from_rotation_translation(quat, translation))
    } else {
        None
    };
    let final_matrix = if let Some(rotation_matrix) = rotation_matrix {
        scale_translation_matrix * rotation_matrix
    } else {
        scale_translation_matrix
    };

    let mut flags = CuboidFlags::empty();

    for (face_name, face_data) in element.faces() {
        flags |= todo!();
    }

    todo!()
}

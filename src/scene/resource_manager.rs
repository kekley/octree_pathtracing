use std::sync::Arc;

use glam::{Mat4, Vec3};
use hashbrown::HashMap;
use lasso::{Rodeo, Spur};
use spider_eye::{
    blockstate::borrow::BlockState,
    borrow::nbt_string::NBTStr,
    interned::{
        block_model::{InternedBlockModel, InternedElement},
        blockstate::{InternedBlockVariants, InternedModelProperties, VariantModelType},
    },
    resource_loader::LoadedResources,
    serde::block_model::{DisplayPosition, PositionData},
};

use crate::{
    gpu_structs::{cuboid::Cuboid, gpu_material::GPUMaterial, model::Model},
    textures::{rtw_image::RTWImage, texture::Texture},
};

pub type TextureID = u32;
pub type CuboidID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

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
    resources: LoadedResources,
}

impl ModelBuilder {
    pub fn new(resources: LoadedResources) -> Self {
        Self {
            resources,
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
        todo!()
    }
    fn get_block_model(&self, resource_location: &str) -> Option<&InternedBlockModel> {
        self.resources.get_model_data(resource_location)
    }

    fn cache_model_and_parents(&mut self, resource_location: Spur, model: InternedBlockModel) {
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
                let parent_location_str = self.resources.interner.resolve(&parent_location_spur);

                let Some(parent_model) = self.get_block_model(parent_location_str) else {
                    eprintln!("model with location {parent_location_str} was not found");
                    self.cache.insert(parent_location_spur, None);
                    return;
                };

                self.cache_model_and_parents(parent_location_spur, parent_model.clone());
                self.cache_model_and_parents(resource_location, model);
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
    ) -> Option<Model> {
        let mapped_state = block_state.to_mapped_state();
        let mapped_state_str = mapped_state.as_nbt_str().to_str();

        let waterlogged = block_state.is_waterlogged();

        let block_name = block_state.get_name().to_str();

        let Some(variants) = self.get_variants_for_block(&block_name) else {
            eprintln!("No variants for {block_name}");
            return None;
        };
        let Some(mapped_state_spur) = self.resources.interner.get(&mapped_state_str) else {
            eprintln!("No entry in interner for {mapped_state_str}");
            return None;
        };

        match variants.get_model_properties_for_mapped_state(
            mapped_state_spur,
            &mapped_state_str,
            &self.resources.interner,
        )? {
            VariantModelType::SingleModel(items) => {
                //TODO this would be a random model every instance of the block. might not
                //implement this

                let model_properties = items
                    .get(0)
                    .expect("There should always be at least one model");
                let model_location_spur = model_properties.get_model_location_spur();

                let mut model: &FinalizedBlockModel;
                loop {
                    if let Some(model_opt) = self.cache.get(&model_location_spur) {
                        model = model_opt.as_ref()?;
                    } else {
                        let model_location_str =
                            self.resources.interner.resolve(&model_location_spur);
                        if let Some(model) = self.get_block_model(model_location_str) {
                            self.cache_model_and_parents(model_location_spur, model.clone());
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

                let cuboids = elements
                    .iter()
                    .map(|element| {
                        let from = element.from();
                        let to = element.to();
                        let faces = element.faces();
                        let rotation = element.rotation();

                        if let Some(rotation) = rotation {
                            let origin = rotation.origin();
                        }

                        Cuboid {
                            flags: todo!(),
                            matrix_id: todo!(),
                            material_ids: todo!(),
                            uvs: todo!(),
                        };
                    })
                    .collect::<Vec<_>>();
            }
            VariantModelType::Multipart(items) => {
                let model_properties = items
                    .iter()
                    .map(|slice_of_models| {
                        slice_of_models
                            .get(0)
                            .expect("The should always be at least one model here")
                    })
                    .collect::<Vec<_>>();
            }
        }

        todo!()
    }

    pub fn get_variants_for_block(&self, block_name: &str) -> Option<&InternedBlockVariants> {
        self.resources.variants.get(block_name)
    }
    pub fn load_texture(&self, resource_location: &str) -> Option<Texture> {
        self.resources.textures.get(resource_location).map(|data| {
            let image = RTWImage::load_from_memory(data)
                .map_err(|err| eprintln!("error loading image {resource_location}: {err}"))
                .ok()?;
            let image_arc = Arc::new(image);
            Some(Texture::Image(image_arc))
        })?
    }
    pub fn load_texture_by_spur(&self, resource_location_spur: Spur) -> Option<Texture> {
        let resource_location = self.resources.interner.resolve(&resource_location_spur);
        self.load_texture(resource_location)
    }
}

pub enum ModelType {
    Simple,
    Complex,
}

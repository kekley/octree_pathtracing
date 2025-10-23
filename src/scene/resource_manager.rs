use crate::gpu_structs::{
    cuboid::{Cuboid, CuboidFlags},
    gpu_material::GPUMaterial,
    model::{Model, ModelFlags},
};
use std::{hash::Hash, sync::Arc, u16};

use glam::Mat4;
use glam::{Quat, Vec2, Vec3A};
use hashbrown::HashMap;
use spider_eye::{
    block_model::borrow::BlockModel,
    block_state::{
        borrow::{BlockModelInfo, BlockstateType},
        common::BlockRotation,
    },
    element::borrow::Element,
    face::common::{face_name::FaceName, rotation::Axis},
    resource_loader::{BlockstateLookupError, ResourceLoader},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelLoadingError {
    #[error("Error when looking up blockstate: {0}")]
    BlockStateLookupError(#[from] BlockstateLookupError),
    #[error("Model fetch returned None. Location:{0}")]
    ModelNotFound(String),
}

///Struct for getting around the fact that floats do not implement ``Hash`` or ``Eq``
#[derive(Debug, Clone)]
struct Mat4Wrapper {
    data: Mat4,
}

impl From<&Mat4Wrapper> for Mat4Wrapper {
    fn from(value: &Mat4Wrapper) -> Self {
        value.clone()
    }
}

impl Mat4Wrapper {
    pub fn from_mat4(mat4: Mat4) -> Self {
        unsafe { std::mem::transmute(mat4) }
    }
    pub fn from_mat4_ref(mat4: &Mat4) -> &Self {
        unsafe { std::mem::transmute(mat4) }
    }
    pub fn to_mat4(self) -> Mat4 {
        unsafe { std::mem::transmute(self) }
    }
    pub fn to_mat4_ref(&self) -> &Mat4 {
        unsafe { std::mem::transmute(self) }
    }
}

impl PartialEq for Mat4Wrapper {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let as_bytes: [u8; 16 * 4] = unsafe { std::mem::transmute(self.clone()) };
        let as_bytes_other: [u8; 16 * 4] = unsafe { std::mem::transmute(other.clone()) };
        as_bytes == as_bytes_other
    }
}

impl Eq for Mat4Wrapper {}

impl Hash for Mat4Wrapper {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let as_bytes: [u8; 16 * 4] = unsafe { std::mem::transmute(self.clone()) };
        as_bytes.hash(state);
    }
}

use crate::textures::{material::Material, rtw_image::RTWImage, texture::Texture};

pub type TextureID = u32;
pub type CuboidID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

pub const UNIT_BLOCK_MIN: Vec3A = Vec3A::splat(-0.5);
pub const UNIT_BLOCK_MAX: Vec3A = Vec3A::splat(0.5);

#[derive(Debug)]
pub struct FinalizedBlockModel<'a> {
    textures: HashMap<&'a str, &'a str>,
    elements: Vec<Element<'a>>,
}

impl<'strings> FinalizedBlockModel<'strings> {
    pub fn get_elements(&self) -> &[Element<'strings>] {
        &self.elements
    }
}

///Model data that can be serialized to send to the gpu
pub struct BuiltModels {
    models: Vec<Model>,
    cuboids: Vec<Cuboid>,
    matrices: Vec<Mat4>,
    materials: Vec<GPUMaterial>,
    textures: Vec<Texture>,
}

pub struct ModelBuilder {
    model_data: Vec<ModelData>,
    textures: HashMap<String, Texture>,
    resources: ResourceLoader,
}

///A handle for accessing a model's data before calling .build() on the builder
#[derive(Debug, Clone, Copy)]
pub struct ModelHandle(pub(crate) usize);

impl ModelBuilder {
    pub fn new(resources: ResourceLoader) -> Self {
        Self {
            resources,
            model_data: Default::default(),
            textures: Default::default(),
        }
    }
    pub fn try_add_model_from_mapped_state(
        &mut self,
        mapped_state_str: &str,
    ) -> Result<ModelHandle, ModelLoadingError> {
        self.load_model_for_mapped_state(mapped_state_str)
            .map(ModelHandle)
    }

    pub fn get_intermediate_model_data(&self, handle: ModelHandle) -> &ModelData {
        self.model_data.get(handle.0).unwrap()
    }

    pub fn build(&self) -> BuiltModels {
        let mut unique_matrices: HashMap<Mat4Wrapper, u32> =
            HashMap::with_capacity(self.model_data.len());
        let mut cuboids = Vec::with_capacity(self.model_data.len());
        let mut unique_materials: HashMap<GPUMaterial, u32> =
            HashMap::with_capacity(self.model_data.len());
        let mut models = Vec::with_capacity(self.model_data.len());

        let num_textures = self.textures.len();
        let texture_index_map = self
            .textures
            .values()
            .cloned()
            .zip(0..num_textures)
            .collect::<HashMap<Texture, usize>>();

        self.model_data
            .iter()
            .for_each(|model_data| match model_data {
                ModelData::SimpleAABB { uvs, materials } => {
                    let material_ids = materials
                        .iter()
                        .map(|mat| {
                            let Material {
                                index_of_refraction,
                                material_flags,
                                specular,
                                emittance,
                                roughness,
                                metalness,
                                texture,
                                tint_index,
                            } = mat;
                            let texture_index = *texture_index_map
                                .get(texture)
                                .expect("Texture hash lookup failed")
                                as u32;

                            let gpu_material = GPUMaterial {
                                ior: *index_of_refraction,
                                specular: *specular,
                                emittance: *emittance,
                                roughness: *roughness,
                                metalness: *metalness,
                                texture_index,
                                tint_index: *tint_index,
                                flags: 0,
                            };

                            let materials_len = unique_materials.len();

                            *unique_materials
                                .entry(gpu_material)
                                .or_insert_with(|| materials_len as u32)
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap();

                    let cuboid = Cuboid {
                        flags: CuboidFlags::ALL_FACES.bits(),
                        matrix_id: 0,
                        material_ids,
                        uvs: unsafe { std::mem::transmute_copy(uvs) },
                    };

                    let cuboid_index = cuboids.len() as u32;

                    cuboids.push(cuboid);

                    let model = Model {
                        model_flags: ModelFlags::SIMPLE_AABB,
                        cuboid_start_index: cuboid_index,
                        length: 1,
                        padding: 0,
                    };

                    models.push(model);
                }
                ModelData::Cuboids(cuboid_datas) => {
                    let cuboid_start_index = cuboids.len() as u32;
                    let cuboid_len = cuboid_datas.len() as u32;

                    cuboid_datas.iter().for_each(|cuboid| {
                        let material_ids = cuboid
                            .materials
                            .iter()
                            .map(|mat| {
                                let Material {
                                    index_of_refraction,
                                    material_flags,
                                    specular,
                                    emittance,
                                    roughness,
                                    metalness,
                                    texture,
                                    tint_index,
                                } = mat;
                                let texture_index = *texture_index_map
                                    .get(texture)
                                    .expect("Texture hash lookup failed")
                                    as u32;

                                let gpu_material = GPUMaterial {
                                    ior: *index_of_refraction,
                                    specular: *specular,
                                    emittance: *emittance,
                                    roughness: *roughness,
                                    metalness: *metalness,
                                    texture_index,
                                    tint_index: *tint_index,
                                    flags: 0,
                                };

                                let materials_len = unique_materials.len();

                                *unique_materials
                                    .entry(gpu_material)
                                    .or_insert_with(|| materials_len as u32)
                            })
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap();

                        let CuboidData {
                            matrix,
                            flags,
                            uvs,
                            materials,
                        } = cuboid;
                        let matrix_id = if let Some(matrix) = matrix {
                            let matrix_len = unique_matrices.len();

                            *unique_matrices
                                .entry_ref(Mat4Wrapper::from_mat4_ref(matrix))
                                .or_insert_with(|| matrix_len as u32)
                        } else {
                            0
                        };

                        let cuboid = Cuboid {
                            flags: flags.bits(),
                            matrix_id,
                            material_ids,
                            uvs: Self::uvs_to_f16_bits(uvs),
                        };

                        cuboids.push(cuboid);
                    });

                    let model = Model {
                        model_flags: ModelFlags::empty(),
                        cuboid_start_index,
                        length: cuboid_len,
                        padding: 0,
                    };

                    models.push(model);
                }
            });

        let mut unsorted_matrices = unique_matrices.into_iter().collect::<Vec<_>>();
        unsorted_matrices.sort_by_key(|(_, i)| *i);

        let matrices = unsorted_matrices
            .into_iter()
            .map(|(mat, _)| mat.to_mat4())
            .collect::<Vec<_>>();

        let mut unsorted_materials = unique_materials.into_iter().collect::<Vec<_>>();
        unsorted_materials.sort_by_key(|(_, i)| *i);

        let materials: Vec<GPUMaterial> = unsorted_materials
            .into_iter()
            .map(|(material, _)| material)
            .collect::<Vec<_>>();

        let mut unsorted_textures = texture_index_map.into_iter().collect::<Vec<_>>();
        unsorted_textures.sort_by_key(|(_, i)| *i);

        let textures = unsorted_textures
            .into_iter()
            .map(|(texture, _)| texture)
            .collect::<Vec<_>>();

        BuiltModels {
            models,
            cuboids,
            matrices,
            materials,
            textures,
        }
    }

    fn uvs_to_f16_bits(uvs: &[Vec2; 12]) -> [u32; 12] {
        uvs.map(|vec2| {
            let x = vec2.x;
            let y = vec2.y;
            let x_as_u16 = (u16::MAX as f32 * x) as u16;
            let y_as_u16 = (u16::MAX as f32 * y) as u16;

            (x_as_u16 as u32) << 16 | (y_as_u16 as u32)
        })
    }

    fn apply_uv_lock(model_data: &mut ModelData, block_model_info: &BlockModelInfo) {
        if !block_model_info.get_uvlock() {
            return;
        }

        let x_rotation = block_model_info.get_block_rotation_x();

        let y_rotation = block_model_info.get_block_rotation_x();

        if x_rotation == BlockRotation::Zero && y_rotation == BlockRotation::Zero {
            return;
        }

        match model_data {
            ModelData::SimpleAABB { uvs, .. } => {
                Self::apply_uv_counter_rotation(uvs, x_rotation, y_rotation);
            }
            ModelData::Cuboids(cuboid_datas) => {
                cuboid_datas.iter_mut().for_each(|cuboid| {
                    Self::apply_uv_counter_rotation(&mut cuboid.uvs, x_rotation, y_rotation);
                });
            }
        }
    }

    /// Rotates a UV rectangle `[u1, v1, u2, v2]` by a 90-degree increment.
    fn rotate_uv_pair(uv_quad: &mut [Vec2; 2], angle_degrees: i16) {
        let [u1, v1] = uv_quad[0].to_array();
        let [u2, v2] = uv_quad[1].to_array();

        // Based on the rotation, we just swap and flip the coordinates.
        let new_uvs = match angle_degrees {
            90 => [Vec2::new(u1, v2), Vec2::new(u2, v1)], // New coords are (u1, v2) -> (u2, v1)
            180 => [Vec2::new(u2, v2), Vec2::new(u1, v1)], // New coords are (u2, v2) -> (u1, v1)
            270 => [Vec2::new(u2, v1), Vec2::new(u1, v2)], // New coords are (u2, v1) -> (u1, v2)
            _ => return,
        };
        *uv_quad = new_uvs;
    }

    fn apply_uv_counter_rotation(
        uvs: &mut [Vec2; 12],
        x_rotation: BlockRotation,
        y_rotation: BlockRotation,
    ) {
        for face in FaceName::iter_faces() {
            let face_index = face as usize;

            let uv_pair: &mut [Vec2; 2] = {
                let this = &mut uvs[face_index..(face_index + 2)];
                if this.len() == 2 {
                    let ptr = {
                        let this = this.as_mut_ptr();
                        this.cast()
                    };

                    // SAFETY: The underlying array of a slice can be reinterpreted as an actual array `[T; N]` if `N` is not greater than the slice's length.
                    let me = unsafe { &mut *ptr };
                    Some(me)
                } else {
                    None
                }
            }
            .unwrap();

            // This logic is derived from observing Minecraft's behavior for all combinations
            // of X and Y rotation on `uvlock`ed blocks like Observers and Dispensers.
            let y_rot_deg = y_rotation.to_degrees() as i16;
            let x_rot_deg = x_rotation.to_degrees() as i16;

            let uv_rotation = match face {
                FaceName::Up => -y_rot_deg,
                FaceName::Down => y_rot_deg,
                _ => {
                    // For North, South, East, West
                    if x_rot_deg == 0 {
                        -y_rot_deg
                    } else {
                        // When pitched on the X-axis, the vertical faces behave differently.
                        let transformed_face = face.rotate_x(x_rotation);
                        match transformed_face {
                            FaceName::Up => -y_rot_deg,
                            FaceName::Down => y_rot_deg,
                            _ => 0, // No secondary rotation needed
                        }
                    }
                }
            };

            let final_rotation = (uv_rotation + 360) % 360;

            if final_rotation != 0 {
                Self::rotate_uv_pair(uv_pair, final_rotation);
            }
        }
    }
    fn create_model_matrix(block_model_info: &BlockModelInfo) -> Option<Mat4> {
        let block_x_rotation = block_model_info.get_block_rotation_x();
        let block_y_rotation = block_model_info.get_block_rotation_y();

        if block_y_rotation == BlockRotation::Zero && block_x_rotation == BlockRotation::Zero {
            return None;
        }

        let x_rotation = Mat4::from_rotation_x(block_x_rotation.to_degrees().to_radians());

        let y_rotation = Mat4::from_rotation_y(block_y_rotation.to_degrees().to_radians());

        let block_matrix = x_rotation * y_rotation;

        Some(block_matrix)
    }

    fn try_finalize_block_model<'a>(
        model: &'a BlockModel<'a>,
        resources: &'a ResourceLoader,
    ) -> Result<FinalizedBlockModel<'a>, ModelLoadingError> {
        let mut current_model = model;
        let mut elements = if model.get_elements().is_empty() {
            None
        } else {
            Some(model.get_elements())
        };
        println!("elements: {elements:?}");
        let mut final_texture_map = current_model.get_textures().clone();

        let finalized_model: FinalizedBlockModel;

        loop {
            if let Some(parent_model_location) = current_model.get_parent() {
                let parent_model = if parent_model_location.split_once(":").is_some() {
                    resources.get_block_model(parent_model_location).unwrap()
                } else {
                    let mut namespace_added = String::new();
                    namespace_added.push_str("minecraft:");
                    namespace_added.push_str(parent_model_location);

                    resources.get_block_model(&namespace_added).unwrap()
                };

                current_model = parent_model;
                if elements.is_none() && !current_model.get_elements().is_empty() {
                    elements = Some(current_model.get_elements());
                }

                final_texture_map.extend(current_model.get_textures());
            } else {
                finalized_model = FinalizedBlockModel {
                    textures: final_texture_map,
                    elements: elements?.to_vec(),
                };
                break;
            };
        }
        Some(finalized_model)
    }

    fn load_model_for_mapped_state(
        &mut self,
        mapped_state_str: &str,
    ) -> Result<usize, ModelLoadingError> {
        let resources = &self.resources;
        let blockstate_type = resources.get_blockstates_for_mapped_state(mapped_state_str)?;

        println!("variant found");

        match blockstate_type {
            BlockstateType::SingleModel(items) => {
                println!("single model");
                //TODO this would be a random model every instance of the block. might not
                //implement this

                let block_model_info = items
                    .first()
                    .expect("There should always be at least one model");

                let model_location = block_model_info.get_resource_path();

                println!("looking for model: {model_location}");
                let block_model = resources
                    .get_block_model(model_location)
                    .ok_or(ModelLoadingError::ModelNotFound(model_location.to_string()))?;
                println!("got block model");

                let finalized_model = Self::try_finalize_block_model(block_model, resources)?;

                println!("got finalized model: {finalized_model:?}");

                let elements = finalized_model.get_elements();

                assert!(!elements.is_empty());

                let mut model_data = Self::finalized_block_model_to_model_data(
                    &finalized_model,
                    &mut self.textures,
                    resources,
                )?;

                Self::apply_uv_lock(&mut model_data, block_model_info);

                Self::apply_block_rotation(&mut model_data, block_model_info);

                let index = self.model_data.len();
                self.model_data.push(model_data);
                println!("returning from model loading!");
                Some(index)
            }

            BlockstateType::Multipart(model_infos) => {
                let first_entries = model_infos
                    .iter()
                    .map(|slice_of_models| {
                        slice_of_models
                            .first()
                            .expect("The should always be at least one model here")
                    })
                    .collect::<Vec<_>>();
                let cuboids = first_entries
                    .into_iter()
                    .flat_map(|block_model_info| {
                        let model_location = block_model_info.get_resource_path();

                        let block_model = resources.get_block_model(model_location)?;

                        let finalized_model =
                            Self::try_finalize_block_model(block_model, resources)?;
                        let elements = finalized_model.get_elements();

                        assert!(!elements.is_empty());

                        let cuboids = Self::finalized_model_to_cuboids_only(
                            &finalized_model,
                            &mut self.textures,
                            resources,
                        );

                        let mut model_data = ModelData::Cuboids(cuboids);

                        Self::apply_uv_lock(&mut model_data, block_model_info);
                        let ModelData::Cuboids(mut cuboids) = model_data else {
                            unreachable!()
                        };
                        if let Some(block_matrix) = Self::create_model_matrix(block_model_info) {
                            Self::apply_block_level_matrix_to_cuboids(&mut cuboids, block_matrix);
                        }

                        Some(cuboids)
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                let index = self.model_data.len();
                self.model_data.push(ModelData::Cuboids(cuboids));
                Some(index)
            }
        }
    }

    fn apply_block_rotation(model_data: &mut ModelData, block_model_info: &BlockModelInfo<'_>) {
        match model_data {
            ModelData::SimpleAABB { uvs, materials } => {
                let block_x_rotation = block_model_info.get_block_rotation_x();
                let block_y_rotation = block_model_info.get_block_rotation_y();

                const WEST: usize = 0;
                const EAST: usize = 1;
                const DOWN: usize = 2;
                const UP: usize = 3;
                const NORTH: usize = 4;
                const SOUTH: usize = 5;

                // This swaps the horizontal faces: North, East, South, West.
                match block_y_rotation {
                    BlockRotation::Ninety => {
                        // 90 degrees clockwise: N -> E, E -> S, S -> W, W -> N
                        let north_mat = materials[NORTH].clone();
                        let north_uvs = uvs[NORTH * 2..NORTH * 2 + 2].to_owned();

                        materials[NORTH] = materials[WEST].clone();
                        uvs.copy_within(WEST * 2..WEST * 2 + 2, NORTH * 2);

                        materials[WEST] = materials[SOUTH].clone();
                        uvs.copy_within(SOUTH * 2..SOUTH * 2 + 2, WEST * 2);

                        materials[SOUTH] = materials[EAST].clone();
                        uvs.copy_within(EAST * 2..EAST * 2 + 2, SOUTH * 2);

                        materials[EAST] = north_mat;
                        uvs[EAST * 2..EAST * 2 + 2].copy_from_slice(&north_uvs);
                    }
                    BlockRotation::OneEighty => {
                        // 180 degrees: N <-> S, E <-> W
                        materials.swap(NORTH, SOUTH);
                        materials.swap(EAST, WEST);
                        uvs.swap(NORTH * 2, SOUTH * 2);
                        uvs.swap(NORTH * 2 + 1, SOUTH * 2 + 1);
                        uvs.swap(EAST * 2, WEST * 2);
                        uvs.swap(EAST * 2 + 1, WEST * 2 + 1);
                    }
                    BlockRotation::TwoSeventy => {
                        // 270 degrees clockwise: N -> W, W -> S, S -> E, E -> N
                        let north_mat = materials[NORTH].clone();
                        let north_uvs = uvs[NORTH * 2..NORTH * 2 + 2].to_owned();

                        materials[NORTH] = materials[EAST].clone();
                        uvs.copy_within(EAST * 2..EAST * 2 + 2, NORTH * 2);

                        materials[EAST] = materials[SOUTH].clone();
                        uvs.copy_within(SOUTH * 2..SOUTH * 2 + 2, EAST * 2);

                        materials[SOUTH] = materials[WEST].clone();
                        uvs.copy_within(WEST * 2..WEST * 2 + 2, SOUTH * 2);

                        materials[WEST] = north_mat;
                        uvs[WEST * 2..WEST * 2 + 2].copy_from_slice(&north_uvs);
                    }
                    BlockRotation::Zero => { /* No rotation */ }
                }

                // This swaps the vertical faces: Up, North, Down, South.
                match block_x_rotation {
                    BlockRotation::Ninety => {
                        // 90 degrees pitch down: U -> N, N -> D, D -> S, S -> U
                        let up_mat = materials[UP].clone();
                        let up_uvs = uvs[UP * 2..UP * 2 + 2].to_owned();

                        materials[UP] = materials[SOUTH].clone();
                        uvs.copy_within(SOUTH * 2..SOUTH * 2 + 2, UP * 2);

                        materials[SOUTH] = materials[DOWN].clone();
                        uvs.copy_within(DOWN * 2..DOWN * 2 + 2, SOUTH * 2);

                        materials[DOWN] = materials[NORTH].clone();
                        uvs.copy_within(NORTH * 2..NORTH * 2 + 2, DOWN * 2);

                        materials[NORTH] = up_mat;
                        uvs[NORTH * 2..NORTH * 2 + 2].copy_from_slice(&up_uvs);
                    }
                    BlockRotation::OneEighty => {
                        // 180 degrees: U <-> D, N <-> S
                        materials.swap(UP, DOWN);
                        materials.swap(NORTH, SOUTH);
                        uvs.swap(UP * 2, DOWN * 2);
                        uvs.swap(UP * 2 + 1, DOWN * 2 + 1);
                        uvs.swap(NORTH * 2, SOUTH * 2);
                        uvs.swap(NORTH * 2 + 1, SOUTH * 2 + 1);
                    }
                    BlockRotation::TwoSeventy => {
                        // 270 degrees pitch down (or 90 up): U -> S, S -> D, D -> N, N -> U
                        let up_mat = materials[UP].clone();
                        let up_uvs = uvs[UP * 2..UP * 2 + 2].to_owned();

                        materials[UP] = materials[NORTH].clone();
                        uvs.copy_within(NORTH * 2..NORTH * 2 + 2, UP * 2);

                        materials[NORTH] = materials[DOWN].clone();
                        uvs.copy_within(DOWN * 2..DOWN * 2 + 2, NORTH * 2);

                        materials[DOWN] = materials[SOUTH].clone();
                        uvs.copy_within(SOUTH * 2..SOUTH * 2 + 2, DOWN * 2);

                        materials[SOUTH] = up_mat;
                        uvs[SOUTH * 2..SOUTH * 2 + 2].copy_from_slice(&up_uvs);
                    }
                    BlockRotation::Zero => { /* No rotation */ }
                }
            }

            ModelData::Cuboids(cuboids) => {
                if let Some(matrix) = Self::create_model_matrix(block_model_info) {
                    Self::apply_block_level_matrix_to_cuboids(cuboids, matrix);
                }
            }
        }
    }

    fn apply_block_level_matrix_to_cuboids(cuboids: &mut [CuboidData], matrix: Mat4) {
        cuboids
            .iter_mut()
            .filter(|cuboid| cuboid.matrix.is_some())
            .for_each(|cuboid| *cuboid.matrix.as_mut().unwrap() *= matrix);
    }

    fn finalized_model_to_cuboids_only(
        model: &FinalizedBlockModel,
        loaded_textures: &mut HashMap<String, Texture>,
        resources: &ResourceLoader,
    ) -> Vec<CuboidData> {
        let FinalizedBlockModel {
            textures: texture_map,
            elements,
        } = model;

        elements
            .iter()
            .map(|element| {
                Self::block_element_to_cuboid(element, texture_map, loaded_textures, resources)
            })
            .collect::<Vec<_>>()
    }

    fn finalized_block_model_to_model_data(
        model: &FinalizedBlockModel,
        loaded_textures: &mut HashMap<String, Texture>,
        resources: &ResourceLoader,
    ) -> Option<ModelData> {
        let FinalizedBlockModel {
            textures: texture_map,
            elements,
        } = model;

        let element_count = elements.len();

        fn element_is_aabb(element: &Element) -> bool {
            element.get_from() == [0.0, 0.0, 0.0]
                && element.get_to() == [16.0, 16.0, 16.0]
                && element.get_faces().count() == 6
        }

        if element_count > 1 || !element_is_aabb(&elements[0]) {
            let cuboids = elements
                .iter()
                .map(|element| {
                    Self::block_element_to_cuboid(element, texture_map, loaded_textures, resources)
                })
                .collect::<Vec<_>>();
            Some(ModelData::Cuboids(cuboids))
        } else {
            let element = &elements[0];

            Some(ModelData::SimpleAABB {
                uvs: Self::get_uvs_from_element(element),
                materials: Self::get_materials_from_element(
                    element,
                    texture_map,
                    loaded_textures,
                    resources,
                )
                .into(),
            })
        }
    }

    fn get_uvs_from_element(element: &Element<'_>) -> [Vec2; 12] {
        let mut uvs = [Vec2::default(); 12];

        for face in element.get_faces() {
            let name = face.get_name();
            let index = name as usize;
            let uv = face.get_uv();
            let x_uv = Vec2::new(uv[0], uv[1]);
            let y_uv = Vec2::new(uv[2], uv[3]);
            uvs[index * 2] = x_uv;
            uvs[index * 2 + 1] = y_uv;
        }

        uvs
    }

    fn get_flags_from_element(element: &Element<'_>) -> CuboidFlags {
        let mut flags = CuboidFlags::empty();
        for face in element.get_faces() {
            let name = face.get_name();
            flags |= CuboidFlags::from(name);
        }
        flags
    }

    fn get_materials_from_element(
        element: &Element<'_>,
        texture_map: &HashMap<&str, &str>,
        loaded_textures: &mut HashMap<String, Texture>,
        resources: &ResourceLoader,
    ) -> [Material; 6] {
        let mut materials = [const { Material::AIR }; 6];

        for face in element.get_faces() {
            let name = face.get_name();
            let index = name as usize;

            let texture_variable = face.get_texture();
            let texture_path =
                Self::resolve_texture_variable(texture_map, texture_variable).unwrap();

            let texture = loaded_textures.entry_ref(texture_path).or_insert_with(|| {
                let texture_data = resources
                    .get_texture_data(texture_path)
                    .expect("Texture data not found");
                let image = Arc::new(
                    RTWImage::load_from_memory(texture_data).expect("Faild to create RTWImage"),
                );
                Texture::Image(image)
            });

            let new_material = Material::builder().albedo(texture.clone()).build();

            materials[index] = new_material;
        }
        materials
    }

    fn get_matrix_from_element(element: &Element<'_>) -> Option<Mat4> {
        let min: Vec3A = Vec3A::from_array(element.get_from());
        let max: Vec3A = Vec3A::from_array(element.get_to());
        let scaled_shifted_from = (min / 16.0) - 0.5;
        let scaled_shifted_to = (max / 16.0) - 0.5;

        let center_point = (scaled_shifted_from + scaled_shifted_to) / 2.0;
        let scale_vector = scaled_shifted_to - scaled_shifted_from;

        let scale_matrix = Mat4::from_scale(scale_vector.into());

        let translation_matrix = Mat4::from_translation(center_point.into());

        let element_rotation = element.get_rotation();

        let rotation_matrix = if let Some(element_rotation) = element_rotation {
            let _origin = Vec3A::from_array(*element_rotation.origin());
            let scaled_origin = (_origin / 16.0) - 0.5;
            let axis = element_rotation.axis();
            let angle = element_rotation.angle().to_radians();
            let quat = match axis {
                Axis::X => Quat::from_rotation_x(angle),
                Axis::Y => Quat::from_rotation_y(angle),
                Axis::Z => Quat::from_rotation_z(angle),
            };

            let rotation = Mat4::from_quat(quat);

            let to_origin = Mat4::from_translation((-scaled_origin).into());
            let from_origin = Mat4::from_translation(scaled_origin.into());

            from_origin * rotation * to_origin
        } else {
            Mat4::IDENTITY
        };
        let final_matrix = translation_matrix * rotation_matrix * scale_matrix;

        if final_matrix == Mat4::IDENTITY {
            None
        } else {
            Some(final_matrix)
        }
    }

    fn block_element_to_cuboid(
        element: &Element<'_>,
        texture_map: &HashMap<&str, &str>,
        loaded_textures: &mut HashMap<String, Texture>,
        resources: &ResourceLoader,
    ) -> CuboidData {
        CuboidData {
            matrix: Self::get_matrix_from_element(element),
            flags: Self::get_flags_from_element(element),
            uvs: Self::get_uvs_from_element(element),
            materials: Self::get_materials_from_element(
                element,
                texture_map,
                loaded_textures,
                resources,
            ),
        }
    }

    fn resolve_texture_variable<'a>(
        texture_map: &'a HashMap<&str, &str>,
        variable: &'a str,
    ) -> Option<&'a str> {
        println!("looking up texture: {variable}");
        let mut current_var = variable.strip_prefix("#").unwrap();

        //Texture variables can point to other texture variables, return when we get to the actual
        //resource path
        loop {
            let result = texture_map.get(&current_var).unwrap();

            if result.starts_with("#") {
                //variable
                current_var = result.strip_prefix("#").unwrap();
            } else {
                return Some(result);
            }
        }
    }
}

pub enum ModelType {
    Simple,
    Complex,
}

#[derive(Debug)]
pub enum ModelData {
    SimpleAABB {
        uvs: [Vec2; 12],
        materials: Box<[Material; 6]>,
    },
    Cuboids(Vec<CuboidData>),
}

#[derive(Debug)]
struct CuboidData {
    matrix: Option<Mat4>,
    flags: CuboidFlags,
    uvs: [Vec2; 12],
    materials: [Material; 6],
}

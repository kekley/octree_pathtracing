use std::sync::Arc;

use glam::{Affine3A, Quat, Vec3, Vec3A};
use spider_eye::{
    block_element::ElementRotation, block_face::FaceName, block_models::BlockRotation, chunk_new::section::BlockState, variant::ModelVariant,
    MCResourceLoader,
};

use crate::geometry::quad::Quad;

use super::resource_model::{BlockModel, QuadModel, SingleBlockModel};

pub type TextureID = u32;
pub type QuadID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

pub struct ModelManager {
    pub resource_loader: MCResourceLoader,
    pub(crate) quads: Arc<parking_lot::RwLock<Vec<Quad>>>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self {
            resource_loader: MCResourceLoader::new(),
            quads: Default::default(),
        }
    }
}

impl ModelManager {
    pub fn model_count(&self) -> usize {
        todo!()
    }
    pub fn seen_materials(&self) -> usize {
        todo!()
    }
    pub fn load_resource(&self, block: &BlockState) -> Option<ResourceModel> {
        todo!()
    }
    pub fn new() -> Self {
        todo!()
    }

    pub fn build_variant(&self, variants: Vec<ModelVariant>) -> Option<ResourceModel> {
        todo!()
    }

    fn get_materials(&self, block_model: &BlockState) -> () {
        todo!()
    }
    fn make_quads(
        &self,
        block_model: &BlockModel,
        rotation_x: Option<BlockRotation>,
        rotation_y: Option<BlockRotation>,
    ) -> Vec<Quad> {
        todo!()
    }

    fn build_model(
        &self,

        block_model: &BlockModel,
        model_type: ModelType,
        rotation_x: Option<BlockRotation>,
        rotation_y: Option<BlockRotation>,
    ) -> Option<ResourceModel> {
        todo!()
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
fn matrix_from_rotation(rotation: &ElementRotation) -> Affine3A {
    let axis = match rotation.axis {
        spider_eye::block_element::ElementAxis::X => Vec3::X,
        spider_eye::block_element::ElementAxis::Y => Vec3::Y,
        spider_eye::block_element::ElementAxis::Z => Vec3::Z,
    };
    let angle: f32 = rotation.angle.into();
    let quat = Quat::from_axis_angle(axis, angle.to_radians());
    Affine3A::from_rotation_translation(quat, Vec3::from_slice(&rotation.origin))
}

fn matrix_from_block_rotation_x(rotation: &BlockRotation) -> Affine3A {
    match rotation {
        BlockRotation::Zero => Affine3A::IDENTITY,
        BlockRotation::Ninety => Affine3A::from_axis_angle(Vec3::X, 90.0f32.to_radians()),
        BlockRotation::OneEighty => Affine3A::from_axis_angle(Vec3::X, 180.0f32.to_radians()),
        BlockRotation::TwoSeventy => Affine3A::from_axis_angle(Vec3::X, 270.0f32.to_radians()),
    }
}
fn matrix_from_block_rotation_y(rotation: &BlockRotation) -> Affine3A {
    match rotation {
        BlockRotation::Zero => Affine3A::IDENTITY,
        BlockRotation::Ninety => Affine3A::from_axis_angle(Vec3::Y, 90.0f32.to_radians()),
        BlockRotation::OneEighty => Affine3A::from_axis_angle(Vec3::Y, 180.0f32.to_radians()),
        BlockRotation::TwoSeventy => Affine3A::from_axis_angle(Vec3::Y, 270.0f32.to_radians()),
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

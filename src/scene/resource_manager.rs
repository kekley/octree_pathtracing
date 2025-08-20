use std::sync::Arc;

use glam::{Affine3A, Quat, Vec3, Vec3A};
use spider_eye::{
    blockstate::borrow::BlockState, interned::blockstate::InternedVariantType,
    resource_loader::LoadedResources, serde::block_model::FaceName,
};

use crate::geometry::quad::Quad;

use super::resource_model::{BlockModel, QuadModel, SingleBlockModel};

pub type TextureID = u32;
pub type QuadID = u32;
pub type MaterialID = u32;
pub type ModelID = u32;

#[derive(Default)]
pub struct ModelManager {
    resources: Option<LoadedResources>,
    pub(crate) quads: Arc<parking_lot::RwLock<Vec<Quad>>>,
}

impl ModelManager {
    pub fn model_count(&self) -> usize {
        todo!()
    }
    pub fn seen_materials(&self) -> usize {
        todo!()
    }
    pub fn load_resource(&self, block: &BlockState<'_, '_>) -> Option<ResourceModel> {
        todo!()
    }
    pub fn new() -> Self {
        todo!()
    }

    pub fn build_variant(&self, variants: Vec<InternedVariantType>) -> Option<ResourceModel> {
        todo!()
    }

    fn get_materials(&self, block_model: &BlockState) -> () {
        todo!()
    }
    fn make_quads(
        &self,
        block_model: &BlockModel,
        rotation_x: Option<i32>,
        rotation_y: Option<i32>,
    ) -> Vec<Quad> {
        todo!()
    }

    fn build_model(
        &self,

        block_model: &BlockModel,
        model_type: ModelType,
        rotation_x: Option<i32>,
        rotation_y: Option<i32>,
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
fn matrix_from_rotation(rotation: &[f32; 3]) -> Affine3A {
    todo!()
}

fn matrix_from_block_rotation_x(rotation: i32) -> Affine3A {
    todo!()
}
fn matrix_from_block_rotation_y(rotation: i32) -> Affine3A {
    todo!()
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

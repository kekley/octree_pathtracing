use std::{array::from_fn, f32::INFINITY, usize};

use glam::{Vec3, Vec3A, Vec4};
use rayon::array;
use spider_eye::{
    block_models::{BlockModel, BlockRotation, IntermediateBlockModel},
    block_texture::{BlockTextures, TextureVariable},
};

use crate::{material, Material, MaterialBuilder, Quad, RTWImage, Ray, Texture};

pub struct SingleBlockModel {
    materials: [Material; 6],
    rotation: BlockRotation,
}
impl SingleBlockModel {}

pub struct MultiBlockModel {
    blocks: Vec<SingleBlockModel>,
}

pub struct QuadModel {
    quads: Vec<Quad>,
}
impl QuadModel {
    const E0: Vec3A = Vec3A::splat(-Ray::EPSILON);
    const E1: Vec3A = Vec3A::splat(1.0 + Ray::EPSILON);
    pub fn hit(&self, ray: &mut Ray) -> bool {
        let mut hit = false;

        ray.hit.t = INFINITY;
        let mut color = Vec4::ZERO;
        self.quads.iter().for_each(|quad| {
            if quad.hit(ray) {
                let c = quad
                    .material
                    .albedo
                    .value(ray.hit.u, ray.hit.v, &Vec3A::ZERO);
                if c.w > Ray::EPSILON {
                    color = c;
                    ray.hit.t = ray.hit.t_next;
                    ray.orient_normal(quad.normal);
                    hit = true
                }
            }
        });

        if hit {
            let p = ray.origin - (ray.at(Ray::OFFSET)).floor() + ray.direction * ray.hit.t_next;
            let gt = p.cmpgt(Self::E1);
            let lt = p.cmplt(Self::E0);

            if gt.any() || lt.any() {
                return false;
            }

            ray.hit.color = color;
            ray.distance_travelled += ray.hit.t;
            ray.origin = ray.at(ray.hit.t);
        }
        hit
    }
}

pub enum MinecraftModel {
    SingleBlock(SingleBlockModel),
    Quad(QuadModel),
}
pub enum ModelType {
    SingleAABB,
    MultiAABB,
    Quads,
}

impl MinecraftModel {
    fn from_variant(variant: VariantModel) -> MinecraftModel {
        MinecraftModel::from_block_model(BlockModel::load("s"))
    }
    fn from_block_model(block_model: BlockModel) -> MinecraftModel {
        match Self::determine_model_type(&block_model) {
            ModelType::SingleAABB => todo!(),
            ModelType::MultiAABB => todo!(),
            ModelType::Quads => todo!(),
        }
    }
    fn determine_model_type(block_model: &BlockModel) -> ModelType {
        match block_model.elements.len() {
            0 => {
                panic!("no elements")
            }
            1 => {
                if block_model.elements[0].is_axis_aligned() && block_model.elements[0].is_cube() {
                    return ModelType::SingleAABB;
                } else {
                    return ModelType::Quads;
                }
            }
            2..=usize::MAX => {
                let is_aabb = block_model
                    .elements
                    .iter()
                    .any(|element| element.is_axis_aligned() && element.is_cube());
                if is_aabb {
                    return ModelType::MultiAABB;
                } else {
                    return ModelType::Quads;
                }
            }
            _ => {
                panic!("")
            }
        }
    }
}

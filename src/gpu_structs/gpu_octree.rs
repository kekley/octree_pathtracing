use std::{fmt::Debug, u32};

use crate::{
    ray_tracing::resource_manager::ResourceModel,
    util,
    voxels::octree::{Octant, Octree},
};
use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable, Default, Debug)]
pub struct TraversalContext {
    pub octree_scale: f32,
    pub root: u32,
    pub scale: u32,
    pub octant_stack: [u32; 23 + 1],
    pub time_stack: [f32; 23 + 1],
    pub padding: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeNode {
    data: [u32; 8],
}

impl From<&Octant<ResourceModel>> for GPUOctreeNode {
    fn from(value: &Octant<ResourceModel>) -> Self {
        let Octant {
            parent,
            child_count,
            children,
        } = value;
        let mut data = [0u32; 8];
        children
            .iter()
            .enumerate()
            .for_each(|(i, child)| match child {
                crate::voxels::octree::Child::None => {}
                crate::voxels::octree::Child::Octant(ind) => {
                    let child_presence_flag: u32 = (1u32 << i) << 16;
                    data[0] |= child_presence_flag;
                    util::write_u30_to_u32_arr(16 + i * 30, *ind, &mut data);
                }
                crate::voxels::octree::Child::Leaf(val) => {
                    let bits: u32 = ((1 << i) | (1 << (i + 8))) << 16;
                    data[0] |= bits as u32;
                    let index = val.get_first_index();
                    util::write_u30_to_u32_arr(16 + i * 30, index, &mut data);
                }
            });
        children.iter().enumerate().for_each(|(i, child)| {
            let is_child = (data[0] >> 16) & (1 << i);
            let is_leaf = (data[0] >> 16) & (1 << 8 + i);

            assert!(!child.is_none() == (is_child != 0));
            assert!(child.is_leaf() == (is_leaf != 0));
            if is_child != 0 {
                if is_leaf != 0 {
                    assert_eq!(
                        child.get_leaf_value().unwrap().get_first_index(),
                        util::extract_u30_from_u32_arr(&data, 16 + i * 30)
                    );
                } else {
                    assert_eq!(
                        child.get_octant_value().unwrap(),
                        util::extract_u30_from_u32_arr(&data, 16 + i * 30)
                    );
                }
            }
        });
        GPUOctreeNode { data }
    }
}

pub struct GPUOctree {
    pub octree_scale: f32,
    pub octants: Vec<GPUOctreeNode>,
    pub depth: u8,
}

impl From<&Octree<ResourceModel>> for GPUOctree {
    fn from(value: &Octree<ResourceModel>) -> Self {
        let vec = value
            .octants
            .iter()
            .map(|octant| GPUOctreeNode::from(octant))
            .collect::<Vec<_>>();

        GPUOctree {
            octree_scale: value.octree_scale,
            octants: vec,
            depth: value.depth,
        }
    }
}

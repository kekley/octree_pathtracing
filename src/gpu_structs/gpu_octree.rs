use std::{fmt::Debug, u16, u32};

use crate::{
    octree::octree::{Child, Octant, Octree},
    scene::resource_manager::ResourceModel,
};
use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable, Default, Debug)]
pub struct TraversalContext {
    pub octree_scale: f32,
    pub root: u32,
    pub scale: u32,
    pub padding: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeNode {
    data: [u32; 12],
}

impl From<&Octant<ResourceModel>> for GPUOctreeNode {
    fn from(value: &Octant<ResourceModel>) -> Self {
        let Octant {
            parent,
            child_count,
            children,
        } = value;
        let mut data = [0u32; 12];
        children
            .iter()
            .enumerate()
            .for_each(|(i, child)| match child {
                Child::None => {}
                Child::Octant(ind) => {
                    let child_mask: u32 = u8::MAX as u32;
                    data[i / 2] |= child_mask << (16 * (i % 2));
                    data[4 + i] = *ind;
                }
                Child::Leaf(val) => {
                    let bits: u32 = u16::MAX as u32;
                    data[i / 2] |= bits << (16 * (i % 2));
                    data[4 + i] = val.get_first_index();
                }
            });
        children.iter().enumerate().for_each(|(i, child)| {
            let is_child = ((data[i / 2] >> (16 * (i % 2))) & 0x0000FFFF) & u8::MAX as u32 != 0;
            let is_leaf = ((data[i / 2] >> (16 * (i % 2))) & 0x0000FFFF) ^ u16::MAX as u32 == 0;

            assert!(!child.is_none() == is_child);
            assert!(child.is_leaf() == is_leaf);
            if is_child {
                if is_leaf {
                    assert_eq!(
                        child.get_leaf_value().unwrap().get_first_index(),
                        data[4 + i]
                    );
                } else {
                    assert_eq!(child.get_octant_value().unwrap(), data[4 + i]);
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

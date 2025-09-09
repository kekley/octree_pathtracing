use std::{fmt::Debug, u32};

use crate::octree::new_octree::{ChildType, Octree};
use anyhow::Chain;
use bytemuck::{Pod, Zeroable};

///The first four words are header data, that leaves 128 bits for metadata about the octants,
///which is 16 bits per octant
///The next 8 words are the actual data for each octant. Either an index to the next child or the
///leaf value
#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeNode {
    data: [u32; 12],
}
#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GPUOctreeUniform {
    pub octree_scale: f32,
    pub depth: u32,
    pub root: u32,
    pub padding: u32,
}

const LEAF_BIT: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0001;
const CHILD_BIT: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0010;
const LOD_BIT: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0100;

pub fn octree_to_gpu_data(tree: &Octree) -> (GPUOctreeUniform, Vec<GPUOctreeNode>) {
    let mut octant_data = [0u32; 12];
    let gpu_octants = tree
        .octants_slice()
        .iter()
        .map(|octant| {
            octant
                .iter_children()
                .enumerate()
                .for_each(|(index, child)| todo!());

            //verify
            octant
                .iter_children()
                .enumerate()
                .for_each(|(index, (child_type, data))| {
                    let header_index = index / 2;
                    let data_index = 4 + index;
                    let header_shift = 16 * (index % 2);

                    let is_child = ((octant_data[header_index] >> header_shift) & CHILD_BIT) != 0;
                    let is_leaf = ((octant_data[header_index] >> header_shift) & LEAF_BIT) != 0;

                    assert!(is_child == matches!(child_type, ChildType::Octant | ChildType::Leaf));

                    assert!(is_leaf == matches!(child_type, ChildType::Leaf));

                    if is_child {
                        assert_eq!(*data, octant_data[data_index]);
                    }
                });
            let node = GPUOctreeNode { data: octant_data };

            octant_data.iter_mut().for_each(|val| {
                *val = 0;
            });
            node
        })
        .collect::<Vec<_>>();

    let uniform = GPUOctreeUniform {
        octree_scale: tree.scale(),
        depth: tree.depth() as u32,
        root: tree.root().unwrap(),
        padding: 0,
    };

    (uniform, gpu_octants)
}

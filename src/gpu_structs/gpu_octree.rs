use std::{fmt::Debug, u32};

use crate::octree::new_octree::{Child, Octree};
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

pub fn octree_to_gpu_data(tree: &Octree<u32>) -> (GPUOctreeUniform, Vec<GPUOctreeNode>) {
    let mut octant_data = [0u32; 12];
    let gpu_octants = tree
        .octants_slice()
        .iter()
        .map(|octant| {
            octant
                .children()
                .iter()
                .enumerate()
                .for_each(|(index, child)| match child {
                    Child::None => {}
                    Child::Lod(data) => {
                        let header_index = index / 2; //first four words are headers
                        let data_index = 4 + index; //actual data is in the next 8 words
                        let header_shift = 16 * (index % 2); //each header word is split in
                                                             //two for each octant

                        octant_data[header_index] |= (LOD_BIT | CHILD_BIT) << header_shift;
                        octant_data[data_index] = *data;
                    }
                    Child::Octant(octant_id) => {
                        let header_index = index / 2;
                        let data_index = 4 + index;
                        let header_shift = 16 * (index % 2);

                        octant_data[header_index] |= CHILD_BIT << header_shift;
                        octant_data[data_index] = *octant_id;
                    }
                    Child::Leaf(data) => {
                        let header_index = index / 2;
                        let data_index = 4 + index;
                        let header_shift = 16 * (index % 2);

                        octant_data[header_index] |= (LEAF_BIT | CHILD_BIT) << header_shift;
                        octant_data[data_index] = *data;
                    }
                });

            //verify
            octant
                .children()
                .iter()
                .enumerate()
                .for_each(|(index, child)| {
                    let header_index = index / 2;
                    let data_index = 4 + index;
                    let header_shift = 16 * (index % 2);

                    let is_child = ((octant_data[header_index] >> header_shift) & CHILD_BIT) != 0;
                    let is_leaf = ((octant_data[header_index] >> header_shift) & LEAF_BIT) != 0;

                    assert!(!child.is_none() == is_child);
                    assert!(child.is_leaf() == is_leaf);
                    if is_child {
                        if is_leaf {
                            assert_eq!(*child.get_leaf_value().unwrap(), octant_data[data_index]);
                        } else {
                            assert_eq!(child.get_octant_id().unwrap(), octant_data[data_index]);
                        }
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

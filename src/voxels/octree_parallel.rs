use std::array;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::ray_tracing::resource_manager::ResourceModel;

use super::octree::{Child, Octant, OctantId, Octree, Position};

impl Octree<ResourceModel> {
    pub fn construct_parallel<P: Position, F: Fn(P) -> Option<ResourceModel> + Sync>(
        depth: u8,
        f: F,
    ) -> Self {
        let size = 2f32.powi((depth - 1) as i32) as u32;
        let a = (0u8..8)
            .into_iter()
            .map(|i: u8| {
                let child_pos = P::construct(
                    0 + size * ((i as u32) & 1),
                    0 + size * ((i as u32 >> 1) & 1),
                    0 + size * ((i as u32 >> 2) & 1),
                );
                let mut subtree: Octree<ResourceModel> = Octree::with_capacity(5000);
                if let Some(result) = subtree.construct_octants_with_impl(size, child_pos, &f) {
                    subtree.root = Some(result);
                    subtree.depth = depth - 1;
                }
                subtree
            })
            .collect::<Vec<Octree<ResourceModel>>>();
        let array: [Octree<ResourceModel>; 8] = a.try_into().unwrap();

        Octree::consume_octrees_of_depth(array, depth - 1).unwrap()
    }

    pub fn consume_octrees_of_depth(
        trees: [Octree<ResourceModel>; 8],
        subtree_depth: u8,
    ) -> Result<Octree<ResourceModel>, ()> {
        let child_array: [Child<ResourceModel>; 8] = array::from_fn(|_| Default::default());
        let root_octant = Octant {
            parent: None,
            child_count: 0,
            children: child_array,
        };
        let mut roots: [Option<OctantId>; 8] = [None; 8];
        let mut new_octant_vec: Vec<Octant<ResourceModel>> = Vec::new();
        new_octant_vec.push(root_octant);
        let mut new_free_list: Vec<u32> = Vec::new();
        trees.into_iter().enumerate().for_each(|(i, tree)| {
            let Octree::<ResourceModel> {
                root,
                mut octants,
                mut free_list,
                depth,
            } = tree;

            if root.is_some() {
                let root = root.unwrap();
                if depth != subtree_depth {
                    return;
                }
                octants.iter_mut().for_each(|octant| {
                    if let Some(parent_value) = octant.parent {
                        octant.parent = Some(parent_value + new_octant_vec.len() as u32);
                    }
                    octant.children.iter_mut().for_each(|child| match child {
                        Child::Octant(id) => *id = *id + new_octant_vec.len() as u32,
                        _ => {}
                    });
                });

                free_list.iter_mut().for_each(|id| {
                    *id += new_octant_vec.len() as u32;
                });
                //println!("{:?}", octants[root as usize]);
                roots[i] = Some(root + new_octant_vec.len() as u32);
                new_octant_vec.extend(octants);
                new_free_list.extend(free_list);
            }
        });
        let root_octant = new_octant_vec.get_mut(0).unwrap();
        let mut child_count = 0;
        root_octant
            .children
            .iter_mut()
            .enumerate()
            .for_each(|(i, child)| {
                *child = {
                    match roots[i] {
                        Some(root) => {
                            child_count += 1;
                            Child::Octant(root)
                        }
                        None => Child::None,
                    }
                }
            });
        root_octant.child_count = child_count;
        let mut new_octree: Octree<ResourceModel> = Octree {
            root: Some(0),
            octants: new_octant_vec,
            free_list: new_free_list,
            depth: subtree_depth + 1,
        };
        new_octree.compact();
        return Ok(new_octree);
    }
}

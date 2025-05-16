use std::{array, u32};

use aovec::Aovec;
use glam::{I64Vec3, IVec3};
use lasso::ThreadedRodeo;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use spider_eye::{
    chunk::{self, Chunk},
    loaded_world::{ChunkCoords, RegionCoords, World, WorldCoords},
    region::{self, LoadedRegion},
};

use crate::ray_tracing::resource_manager::{ModelManager, ResourceModel};

use super::octree::{Child, Octant, OctantId, Octree, Position};

impl Octree<ResourceModel> {
    pub fn load_mc_world<P: Position>(
        origin: WorldCoords,
        depth: u8,
        world: World,
        model_manager: &ModelManager,
    ) -> Self {
        let size = 2u32.pow(depth as u32);
        let mut octree = Octree::<ResourceModel>::new();
        let offset = I64Vec3::new(origin.x, origin.y, origin.z);
        let pos = P::construct(0, 0, 0);
        let result = match size {
            //single chunk
            1..=2 => {
                let region = world.load_region(origin.into());
                if let Some(region) = region {
                    let chunk_coords: ChunkCoords = origin.into();
                    if let Some(chunk) = region.get_chunk(
                        (chunk_coords.x.abs() % 32) as u32,
                        (chunk_coords.z.abs() % 32) as u32,
                    ) {
                        octree.construct_block_level(chunk, model_manager, offset, pos)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            3..=16 => {
                let region = world.load_region(origin.into());
                if let Some(region) = region {
                    let chunk_coords: ChunkCoords = origin.into();
                    if let Some(chunk) = region.get_chunk(
                        (chunk_coords.x.abs() % 32) as u32,
                        (chunk_coords.z.abs() % 32) as u32,
                    ) {
                        octree.construct_chunk_level(size, chunk, model_manager, offset, pos)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            17..=1024 => {
                let region = world.load_region(origin.into());
                if let Some(region) = region {
                    octree.construct_region_level(size, &region, model_manager, offset, pos)
                } else {
                    None
                }
            }
            1025..=u32::MAX => {
                octree.construct_world_level(size, &world, model_manager, offset, pos)
            }
            _ => {
                unreachable!()
            }
        };
        if let Some(result) = result {
            octree.root = Some(result);
            octree.depth = depth;
            octree.octree_scale = f32::exp2(-(depth as f32));
            return octree;
        }
        panic!()
    }
    //call when size is 2049-inf
    fn construct_world_level<P: Position>(
        &mut self,
        size: u32,
        world: &World,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let mut new_parent = None;

        (0..8).for_each(|i| {
            let child_pos = P::construct(
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32 >> 1) & 1),
                pos.z() + size * ((i as u32 >> 2) & 1),
            );

            if size > 1024 {
                let child_id =
                    self.construct_world_level(size, world, model_manager, offset, child_pos);
                let Some(child_id) = child_id else {
                    return;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Octant(child_id));
                let child = &mut self.octants[child_id as usize];
                child.parent = Some(*parent_id);
                return;
            }
            let region_coords: RegionCoords = WorldCoords {
                x: child_pos.x() as i64 + offset.x,
                y: child_pos.y() as i64 + offset.y,
                z: child_pos.z() as i64 + offset.z,
            }
            .into();

            if let Some(region) = world.load_region(region_coords) {
                if let Some(value) =
                    self.construct_region_level(size, &region, model_manager, offset, child_pos)
                {
                    let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                    self.octants[*parent_id as usize].set_child(i, Child::Octant(value));
                }
            }
        });
        new_parent
    }
    //call when size is between 17-1024
    fn construct_region_level<P: Position>(
        &mut self,
        size: u32,
        region: &LoadedRegion,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let mut new_parent = None;

        (0..8).for_each(|i| {
            let child_pos = P::construct(
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32 >> 1) & 1),
                pos.z() + size * ((i as u32 >> 2) & 1),
            );
            if size > 16 {
                let child_id =
                    self.construct_region_level(size, region, model_manager, offset, child_pos);
                let Some(child_id) = child_id else {
                    return;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Octant(child_id));
                let child = &mut self.octants[child_id as usize];
                child.parent = Some(*parent_id);
                return;
            }
            let chunk_coords: ChunkCoords = WorldCoords {
                x: child_pos.x() as i64 + offset.x,
                y: child_pos.y() as i64 + offset.y,
                z: child_pos.z() as i64 + offset.z,
            }
            .into();

            if let Some(chunk) = region.get_chunk(
                (chunk_coords.x.abs() % 32) as u32,
                (chunk_coords.z.abs() % 32) as u32,
            ) {
                if let Some(value) =
                    self.construct_chunk_level(size, chunk, model_manager, offset, child_pos)
                {
                    let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                    self.octants[*parent_id as usize].set_child(i, Child::Octant(value));
                }
            }
        });
        new_parent
    }
    //call this when size is between 3-16
    fn construct_chunk_level<P: Position>(
        &mut self,
        size: u32,
        chunk: &Chunk,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let mut new_parent = None;

        (0..8).for_each(|i| {
            let child_pos = P::construct(
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32 >> 1) & 1),
                pos.z() + size * ((i as u32 >> 2) & 1),
            );
            if size > 2 {
                let child_id =
                    self.construct_chunk_level(size, chunk, model_manager, offset, child_pos);
                let Some(child_id) = child_id else {
                    return;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Octant(child_id));
                let child = &mut self.octants[child_id as usize];
                child.parent = Some(*parent_id);
                return;
            }
            if let Some(value) = self.construct_block_level(chunk, model_manager, offset, child_pos)
            {
                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Octant(value));
            }
        });
        new_parent
    }
    //returns an octant right above the leaf level. call this when size is 2
    fn construct_block_level<P: Position>(
        &mut self,
        chunk: &Chunk,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let mut new_parent: Option<OctantId> = None;
        (0..8).for_each(|child_idx| {
            let child_pos = WorldCoords {
                x: ((pos.x() + ((child_idx as u32) & 1)) as i64) + offset.x,
                y: ((pos.y() + ((child_idx as u32 >> 1) & 1)) as i64) + offset.y,
                z: ((pos.z() + ((child_idx as u32 >> 2) & 1)) as i64) + offset.z,
            };
            if let Some(block) = chunk.get_world_block(child_pos) {
                if let Some(model) = model_manager.load_resource(block) {
                    let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                    self.octants[*parent_id as usize].set_child(child_idx, Child::Leaf(model));
                }
            }
        });
        new_parent
    }
}

impl Octree<ResourceModel> {
    pub fn construct_parallel<P: Position, F: Fn(P) -> Option<ResourceModel> + Sync>(
        depth: u8,
        f: F,
    ) -> Self {
        let size = 2f32.powi((depth - 1) as i32) as u32;
        let a = (0u8..8)
            .into_par_iter()
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
                octree_scale,
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
            octree_scale: f32::exp2(-((subtree_depth + 1) as f32)),
        };
        new_octree.compact();
        return Ok(new_octree);
    }
}

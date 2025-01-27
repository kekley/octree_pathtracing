use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use glam::UVec3;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use spider_eye::{
    chunk::Chunk,
    loaded_world::{ChunkCoords, World, WorldCoords},
};

use crate::{octree, Octree};
const WORLD_SIZE: usize = 10;
#[derive(PartialEq, Eq, Hash)]
pub struct OctreeChunkPos {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}
impl OctreeChunkPos {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }
}

impl Octree<Octree<u32>> {
    pub fn minecraft_world(at: WorldCoords, world: World) -> Self {
        let x_offset = (at.x - (WORLD_SIZE / 2) as i64 * 32).abs();
        let z_offset = (at.z - (WORLD_SIZE / 2) as i64 * 32).abs();
        let chunk_list: DashMap<OctreeChunkPos, Octree<u32>> = DashMap::new();
        (0..WORLD_SIZE).into_par_iter().for_each(|x| {
            (0..WORLD_SIZE).into_par_iter().for_each(|z| {
                (0..WORLD_SIZE).for_each(|y| {
                    let chunk_pos = OctreeChunkPos::new(x, y, z);
                    let mc_chunk_pos0: ChunkCoords = WorldCoords {
                        x: (x as i64 * 32) - x_offset,
                        y: (y as i64 * 32),
                        z: (z as i64 * 32) - z_offset,
                    }
                    .into();
                    let mc_chunk_pos1: ChunkCoords = WorldCoords {
                        x: (x as i64 * 32) - x_offset + 1 * 16,
                        y: (y as i64 * 32),
                        z: (z as i64 * 32) - z_offset,
                    }
                    .into();
                    let mc_chunk_pos2: ChunkCoords = WorldCoords {
                        x: (x as i64 * 32) - x_offset,
                        y: (y as i64 * 32),
                        z: (z as i64 * 32) - z_offset + 1 * 16,
                    }
                    .into();

                    let mc_chunk_pos3: ChunkCoords = WorldCoords {
                        x: (x as i64 * 32) - x_offset + 1 * 16,
                        y: (y as i64 * 32),
                        z: (z as i64 * 32) - z_offset + 1 * 16,
                    }
                    .into();

                    let chunks: [Option<Arc<Chunk>>; 4] = [
                        world.get_chunk_cached(mc_chunk_pos0),
                        world.get_chunk_cached(mc_chunk_pos1),
                        world.get_chunk_cached(mc_chunk_pos2),
                        world.get_chunk_cached(mc_chunk_pos3),
                    ];
                    for chunk in &chunks {
                        if chunk.as_ref().is_some() {
                            //println!("{:?}", chunk.as_ref().unwrap().coords);
                        }
                    }
                    let f = |position: UVec3| -> Option<u32> {
                        let chunk_pos = &chunk_pos;
                        let chunk = &chunks[(position.x / 16 + (position.z / 16) * 2) as usize];
                        let chunk = chunk.as_ref();
                        let world_coords = WorldCoords {
                            x: (position.x as i64 - x_offset) + chunk_pos.x as i64 * 32,
                            y: position.y as i64 - 64 + chunk_pos.y as i64 * 32,
                            z: (position.z as i64 - z_offset) + chunk_pos.z as i64 * 32,
                        };

                        let block = chunk?.get_world_block(world_coords);
                        if block.is_some() {
                            if block.unwrap() == 0 {
                                return None;
                            }
                        }
                        block
                    };
                    let mut oct_chunk: Octree<u32> = Octree::with_capacity(5000);
                    oct_chunk.construct_octants_with(5, f);
                    chunk_list.insert(chunk_pos, oct_chunk);
                });
            });
        });

        let mut world: Octree<Octree<u32>> = Octree::with_capacity(1000);
        let world_f = |pos: UVec3| -> Option<Octree<u32>> {
            let chunk_pos = OctreeChunkPos::new(pos.x as usize, pos.y as usize, pos.z as usize);
            let mut res = chunk_list.remove(&chunk_pos);
            if res.is_some() {
                let res = res.unwrap().1;
                if res.depth() != 0 {
                    return Some(res);
                } else {
                    return None;
                }
            } else {
                None
            }
        };
        world.construct_octants_with(4, world_f);
        world
    }
}

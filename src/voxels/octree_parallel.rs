use std::{
    array,
    cell::Cell,
    sync::{Arc, LazyLock, Mutex, OnceLock},
    u32,
};

use aovec::Aovec;
use dashmap::RwLock;
use glam::{I64Vec3, IVec3};
use lasso::ThreadedRodeo;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use spider_eye::{
    chunk::{self, Chunk},
    loaded_world::{ChunkCoords, RegionCoords, World, WorldCoords},
    region::{self, LazyRegion, LoadedRegion},
};

use crate::{
    ray_tracing::resource_manager::{ModelManager, ResourceModel},
    voxels::octree::Child,
};

use super::octree::{Octant, OctantId, Position};
use std::{fmt::Debug, hash::Hash, mem};

use glam::UVec3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LeafId {
    pub parent: OctantId,
    pub idx: u8,
}

pub struct ParallelOctree<T: Copy> {
    pub octree_scale: f32,
    pub root: Option<OctantId>,
    pub octants: Aovec<Cell<Octant<T>>>,
    pub free_list: Arc<Mutex<Vec<OctantId>>>,
    pub depth: u8,
}

impl<T: Copy> ParallelOctree<T> {
    pub fn new() -> Self {
        Self::new_in()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity)
    }
}

impl<T: PartialEq + Copy> PartialEq for ParallelOctree<T> {
    fn eq(&self, other: &Self) -> bool {
        let self_lock = self.free_list.lock().unwrap();
        let self_free_list: &Vec<_> = self_lock.as_ref();
        let other_lock = other.free_list.lock().unwrap();
        let other_free_list: &Vec<_> = other_lock.as_ref();

        let self_octant_count = self.octants.len();
        let other_octant_count = other.octants.len();

        let octants_eq = if self_octant_count == other_octant_count {
            (0..self_octant_count).all(|i| self.octants[i] == other.octants[i])
        } else {
            false
        };

        self.root.eq(&other.root)
            && octants_eq
            && self_free_list.eq(other_free_list)
            && self.depth.eq(&other.depth)
    }
}

impl<T: Copy> ParallelOctree<T> {
    fn new_in() -> Self {
        Self::with_capacity_in(5000000)
    }

    fn with_capacity_in(capacity: usize) -> Self {
        Self {
            octree_scale: f32::exp2(-0.0),
            root: None,
            octants: Aovec::new(capacity),
            free_list: Arc::new(Mutex::new(vec![])),
            depth: 0,
        }
    }

    pub fn set_leaf(&mut self, pos: impl Position, leaf: T) -> (LeafId, Option<T>) {
        self.expand(pos.required_depth());

        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.powi(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;

            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            if size == 1 {
                let mut octant = self.octants[it as usize].get();
                octant.set_child(idx, Child::Leaf(leaf));
                let prev = self.octants[it as usize].replace(octant).children[it as usize];
                return (LeafId { parent: it, idx }, prev.into_leaf_value());
            }
            it = self.step_into_or_create_octant_at(it, idx);
        }
        unreachable!()
    }

    pub fn get_leaf(&self, pos: impl Position) -> Option<T> {
        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.powi(self.depth as i32) as u32;

        while size > 0 {
            size /= 2;
            let idx = pos.div(size).idx();
            //println!("it: {}, pos: {:?} idx: {},size: {}", it, pos, idx, size);
            pos.rem_assign(size);

            let child = &self.octants[it as usize].get().children[idx as usize];
            if child.is_none() {
                //println!("is none! size:{}", size);
                break;
            }

            match &self.octants[it as usize].get().children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(val) => return Some(*val),
            }
        }

        None
    }

    pub fn construct_octants_with<P: Position, F: Fn(P) -> Option<T>>(&mut self, depth: u8, f: F) {
        self.reset();

        let size = 2f32.powi(depth as i32) as u32;

        if let Some(result) = self.construct_octants_with_impl(size, P::construct(0, 0, 0), &f) {
            self.root = Some(result);
            self.depth = depth;
        }
    }
    pub(super) fn construct_octants_with_impl<P: Position, F: Fn(P) -> Option<T>>(
        &self,
        size: u32,
        pos: P,
        f: &F,
    ) -> Option<OctantId> {
        let size = size / 2;

        let mut new_parent = None;

        (0u8..8).for_each(|i| {
            let child_pos = P::construct(
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32 >> 1) & 1),
                pos.z() + size * ((i as u32 >> 2) & 1),
            );

            if size > 1 {
                let child_id = self.construct_octants_with_impl(size, child_pos, f);
                let Some(child_id) = child_id else {
                    return;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                let mut parent_mut = self.octants[*parent_id as usize].get();
                parent_mut.set_child(i, Child::Octant(child_id));
                self.octants[*parent_id as usize].set(parent_mut);
                let mut child = self.octants[child_id as usize].get();
                child.parent = Some(*parent_id);
                self.octants[child_id as usize].set(child);
                return;
            }

            if let Some(value) = f(child_pos) {
                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                let mut parent_mut = self.octants[*parent_id as usize].get();
                parent_mut.set_child(i, Child::Leaf(value));
                self.octants[*parent_id as usize].set(parent_mut);
            }
        });

        new_parent
    }

    pub fn move_leaf(&mut self, leaf_id: LeafId, to_pos: impl Position) -> (LeafId, Option<T>) {
        self.expand_to(to_pos.required_depth());

        let mut it = self.root.unwrap();
        let mut pos = to_pos;
        let mut size = 2f32.powi(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            if size == 1 {
                //leaf replaced with itself
                if it == leaf_id.parent && idx == leaf_id.idx {
                    return (leaf_id, None);
                }
                let mut octant = self.octants[it as usize].get();
                let old_leaf = octant.set_child(leaf_id.idx, Child::None);
                self.octants[it as usize].set(octant);

                let mut octant = self.octants[leaf_id.parent as usize].get();
                let new_leaf = octant.set_child(leaf_id.idx, Child::None);
                self.octants[leaf_id.parent as usize].set(octant);
                if new_leaf.get_leaf_value().is_some() {
                    let mut octant = self.octants[it as usize].get();
                    octant.set_child(idx, new_leaf);
                    self.octants[it as usize].set(octant);
                }

                let new_leaf_id = LeafId { parent: it, idx };

                match old_leaf {
                    Child::None => return (new_leaf_id, None),
                    Child::Octant(_) => unreachable!("found unexpected octant"),
                    Child::Leaf(val) => return (new_leaf_id, Some(val)),
                }
            }
            it = self.step_into_or_create_octant_at(it, idx);
        }
        unreachable!("could not reach end of tree")
    }

    pub fn remove_leaf(&mut self, pos: impl Position) -> (Option<T>, Option<LeafId>) {
        if pos.required_depth() > self.depth {
            return (None, None);
        }

        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.powi(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            match &self.octants[it as usize].get().children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(_) => {
                    let mut octant = self.octants[it as usize].get();
                    match octant.set_child(idx, Child::None) {
                        Child::None => {
                            self.octants[it as usize].set(octant);
                            return (None, None);
                        }
                        Child::Octant(_) => unreachable!(),
                        Child::Leaf(val) => {
                            self.octants[it as usize].set(octant);
                            return (
                                Some(val),
                                Some(LeafId {
                                    parent: it,
                                    idx: idx,
                                }),
                            );
                        }
                    }
                }
            }
        }
        (None, None)
    }

    pub fn remove_leaf_by_id(&mut self, leaf_id: LeafId) -> Option<T> {
        match self.octants[leaf_id.parent as usize].get().children[leaf_id.idx as usize] {
            Child::None | Child::Octant(_) => None,
            Child::Leaf(_) => {
                let mut octant = self.octants[leaf_id.parent as usize].get();
                match octant.set_child(leaf_id.idx, Child::None) {
                    Child::None => {
                        self.octants[leaf_id.parent as usize].set(octant);
                        None
                    }
                    Child::Octant(_) => unreachable!("found unexpected octant"),
                    Child::Leaf(val) => {
                        self.octants[leaf_id.parent as usize].set(octant);
                        Some(val)
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.root = None;
        self.octants = Aovec::new(1024);
        self.free_list = Arc::new(Mutex::new(vec![]));
        self.depth = 0;
    }

    pub fn expand_to(&mut self, to: u8) {
        if self.depth > to {
            return;
        }
        let diff = to - self.depth;
        if diff > 0 {
            self.expand(diff);
        }
    }

    pub fn expand(&mut self, by: u8) {
        for _ in 0..by {
            let new_root_id = self.new_octant(None);

            if let Some(root_id) = self.root {
                let mut root_octant = self.octants[root_id as usize].get();
                root_octant.parent = Some(new_root_id);
                let mut new_root = self.octants[new_root_id as usize].get();
                new_root.set_child(0, Child::Octant(root_id));
                self.octants[root_id as usize].set(root_octant);
                self.octants[new_root_id as usize].set(new_root);
            }
            self.root = Some(new_root_id)
        }
        self.depth += by
    }

    pub(crate) fn new_octant(&self, parent: Option<OctantId>) -> OctantId {
        let mut free_list = self.free_list.lock().unwrap();
        if let Some(free_id) = free_list.pop() {
            drop(free_list);
            let mut free_octant = self.octants[free_id as usize].get();
            free_octant.parent = parent;
            self.octants[free_id as usize].set(free_octant);
            return free_id;
        }
        let id = self.octants.push(
            Octant {
                parent,
                child_count: 0,
                children: Default::default(),
            }
            .into(),
        );
        id as u32
    }

    pub fn compact(&mut self) {
        if self.root.is_none() {
            return;
        }
        self.compact_octant(self.root.unwrap());

        if self.octants[self.root.unwrap() as usize].get().child_count != 0 {
            return;
        }

        self.reset();
    }

    fn compact_octant(&mut self, octant_id: OctantId) {
        for i in 0..8 {
            let id = {
                let octant = &self.octants[octant_id as usize].get();
                octant.children[i].get_octant_value()
            };
            if id.is_none() {
                continue;
            }
            let id = id.unwrap();

            self.compact_octant(id);

            let octant = &self.octants[id as usize].get();
            if octant.child_count == 0 {
                self.delete_octant(id);

                let mut octant = self.octants[octant_id as usize].get();
                octant.set_child(i as u8, Child::None);
                self.octants[octant_id as usize].set(octant);
            }
        }
    }

    fn delete_octant(&mut self, id: OctantId) {
        if let Some(parent) = self.octants[id as usize].get().parent {
            let children = &self.octants[parent as usize].get().children;
            let idx = children
                .iter()
                .position(|x| x.get_octant_value() == Some(id));

            if let Some(idx) = idx {
                let mut octant = self.octants[parent as usize].get();
                octant.set_child(idx as u8, Child::None);
                self.octants[parent as usize].set(octant);
            }
        }

        let mut octant = self.octants[id as usize].get();
        octant.parent = None;
        octant.child_count = 0;
        for child in &mut octant.children {
            *child = Child::None;
        }
        self.octants[id as usize].set(octant);

        let mut free_list = self.free_list.lock().unwrap();
        free_list.push(id);
    }

    fn step_into_or_create_octant_at(&mut self, it: OctantId, idx: u8) -> OctantId {
        match &self.octants[it as usize].get().children[idx as usize] {
            Child::None => {
                let prev_id = it;
                let next_id = self.new_octant(Some(prev_id));
                let mut prev_octant: Octant<T> = self.octants[prev_id as usize].get();
                prev_octant.set_child(idx, Child::Octant(next_id));
                self.octants[prev_id as usize].set(prev_octant);
                next_id
            }
            Child::Octant(id) => *id,
            Child::Leaf(_) => unreachable!("found unexpected leaf"),
        }
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }
}

impl ParallelOctree<ResourceModel> {
    pub fn load_mc_world<P: Position + Sync>(
        origin: WorldCoords,
        depth: u8,
        world: World,
        model_manager: &ModelManager,
    ) -> Self {
        let size = 2u32.pow(depth as u32);
        let mut octree = ParallelOctree::<ResourceModel>::new();
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
                let region = world.get_region_lazy(origin.into());
                if let Some(region) = region {
                    let chunk_coords: ChunkCoords = origin.into();
                    if let Some(chunk) = region.get_chunk(chunk_coords) {
                        octree.construct_chunk_level(size, &chunk, model_manager, offset, pos)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            17..=1024 => {
                let region = world.get_region_lazy(origin.into());
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
    #[inline(always)]

    fn construct_world_level<P: Position + Sync>(
        &self,
        size: u32,
        world: &World,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let new_parent: OnceLock<u32> = OnceLock::new();

        if size > 1024 {
            (0..8).into_iter().for_each(|i| {
                let child_pos = P::construct(
                    pos.x() + size * ((i as u32) & 1),
                    pos.y() + size * ((i as u32 >> 1) & 1),
                    pos.z() + size * ((i as u32 >> 2) & 1),
                );

                let child_id =
                    self.construct_world_level(size, world, model_manager, offset, child_pos);
                let Some(child_id) = child_id else {
                    return;
                };

                let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                let mut parent = self.octants[*parent_id as usize].get();
                parent.set_child(i, Child::Octant(child_id));
                self.octants[*parent_id as usize].set(parent);
                let mut child = self.octants[child_id as usize].get();
                child.parent = Some(*parent_id);
                self.octants[child_id as usize].set(child);
                return;
            });
        } else {
            let region_coords: RegionCoords = WorldCoords {
                x: pos.x() as i64 + offset.x,
                y: pos.y() as i64 + offset.y,
                z: pos.z() as i64 + offset.z,
            }
            .into();
            dbg!(region_coords);
            let mut regions: [Option<LazyRegion>; 4] = [const { None }; 4];
            regions.iter_mut().enumerate().for_each(|(i, region)| {
                let region_coords = RegionCoords {
                    x: region_coords.x + (i & 1) as i64,
                    z: region_coords.z + (i & 2) as i64,
                };
                *region = world.get_region_lazy(region_coords);
            });
            (0..8).into_iter().for_each(|i| {
                let x = i as usize & 1;
                let z = (i as usize >> 2) & 1;
                let child_pos = P::construct(
                    pos.x() + size * ((i as u32) & 1),
                    pos.y() + size * ((i as u32 >> 1) & 1),
                    pos.z() + size * ((i as u32 >> 2) & 1),
                );

                dbg!(&child_pos);

                if let Some(region) = &regions[x + z * 2] {
                    if let Some(value) =
                        self.construct_region_level(size, &region, model_manager, offset, child_pos)
                    {
                        let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                        let mut parent = self.octants[*parent_id as usize].get();

                        parent.set_child(i, Child::Octant(value));
                        self.octants[*parent_id as usize].set(parent);
                    }
                }
            });
        }
        new_parent.into_inner()
    }
    //call when size is between 17-1024
    #[inline(always)]

    fn construct_region_level<P: Position + Sync>(
        &self,
        size: u32,
        region: &LazyRegion,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let new_parent: OnceLock<u32> = OnceLock::new();

        (0..8).into_iter().for_each(|i| {
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

                let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                let mut parent = self.octants[*parent_id as usize].get();
                parent.set_child(i, Child::Octant(child_id));
                self.octants[*parent_id as usize].set(parent);
                let mut child = self.octants[child_id as usize].get();
                child.parent = Some(*parent_id);
                self.octants[child_id as usize].set(child);
                return;
            }
            let chunk_coords: ChunkCoords = WorldCoords {
                x: child_pos.x() as i64 + offset.x,
                y: child_pos.y() as i64 + offset.y,
                z: child_pos.z() as i64 + offset.z,
            }
            .into();

            if let Some(chunk) = region.get_chunk(chunk_coords) {
                if let Some(value) =
                    self.construct_chunk_level(size, &chunk, model_manager, offset, child_pos)
                {
                    let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                    let mut parent = self.octants[*parent_id as usize].get();
                    parent.set_child(i, Child::Octant(value));
                    self.octants[*parent_id as usize].set(parent);
                }
            }
        });
        new_parent.into_inner()
    }
    //call this when size is between 3-16
    #[inline(always)]

    fn construct_chunk_level<P: Position + Sync>(
        &self,
        size: u32,
        chunk: &Chunk,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let size = size / 2;
        let new_parent: OnceLock<u32> = OnceLock::new();

        (0..8).into_iter().for_each(|i| {
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

                let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                let mut parent = self.octants[*parent_id as usize].get();
                parent.set_child(i, Child::Octant(child_id));
                self.octants[*parent_id as usize].set(parent);
                let mut child = self.octants[child_id as usize].get();
                child.parent = Some(*parent_id);
                self.octants[child_id as usize].set(child);
                return;
            }
            if let Some(value) = self.construct_block_level(chunk, model_manager, offset, child_pos)
            {
                let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                let mut parent = self.octants[*parent_id as usize].get();
                parent.set_child(i, Child::Octant(value));
                self.octants[*parent_id as usize].set(parent);
            }
        });
        new_parent.into_inner()
    }
    //returns an octant right above the leaf level. call this when size is 2
    #[inline(always)]

    fn construct_block_level<P: Position + Sync>(
        &self,
        chunk: &Chunk,
        model_manager: &ModelManager,
        offset: I64Vec3,
        pos: P,
    ) -> Option<OctantId> {
        let new_parent: OnceLock<u32> = OnceLock::new();
        (0..8).into_iter().for_each(|child_idx| {
            let child_pos = WorldCoords {
                x: ((pos.x() + ((child_idx as u32) & 1)) as i64) + offset.x,
                y: ((pos.y() + ((child_idx as u32 >> 1) & 1)) as i64) + offset.y,
                z: ((pos.z() + ((child_idx as u32 >> 2) & 1)) as i64) + offset.z,
            };
            if let Some(block) = chunk.get_world_block(child_pos) {
                if let Some(model) = model_manager.load_resource(block) {
                    let parent_id = new_parent.get_or_init(|| self.new_octant(None));
                    let mut parent = self.octants[*parent_id as usize].get();
                    parent.set_child(child_idx, Child::Leaf(model));
                    self.octants[*parent_id as usize].set(parent);
                }
            }
        });
        new_parent.into_inner()
    }
}

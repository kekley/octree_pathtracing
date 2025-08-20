use std::{array, fmt::Debug, hash::Hash, mem};

use glam::{I64Vec3, UVec3};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use spider_eye::{
    chunk::Chunk,
    coords::{block::BlockCoords, chunk::ChunkCoords, region::RegionCoords},
    loaded_world::World,
    region::LazyRegion,
};

use crate::scene::resource_manager::{ModelManager, ResourceModel};

use super::octree_parallel::ParallelOctree;

pub trait Position: Copy + Clone + Debug + Sized {
    fn construct(x: u32, y: u32, z: u32) -> Self;
    fn idx(&self) -> u8;
    fn required_depth(&self) -> u8;
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn z(&self) -> u32;
    fn div(&self, rhs: u32) -> Self;
    fn rem_assign(&mut self, rhs: u32);
}

impl Position for UVec3 {
    fn idx(&self) -> u8 {
        (self.x + self.y * 2 + self.z * 4) as u8
    }
    fn required_depth(&self) -> u8 {
        let depth = self.max_element();
        (depth as f32).log2().floor() as u8 + 1
    }

    fn construct(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    fn x(&self) -> u32 {
        self.x
    }

    fn y(&self) -> u32 {
        self.y
    }

    fn z(&self) -> u32 {
        self.z
    }

    fn div(&self, rhs: u32) -> Self {
        *self / rhs
    }

    fn rem_assign(&mut self, rhs: u32) {
        *self %= rhs;
    }
}

pub type OctantId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LeafId {
    pub parent: OctantId,
    pub idx: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Child<T> {
    #[default]
    None,
    Octant(OctantId),
    Leaf(T),
}

impl<T> Child<T> {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_octant(&self) -> bool {
        matches!(self, Self::Octant(_))
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    pub fn get_leaf_value(&self) -> Option<&T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }
    pub fn get_octant_value(&self) -> Option<OctantId> {
        match self {
            Self::Octant(id) => Some(*id),
            _ => None,
        }
    }

    pub fn get_leaf_value_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }

    pub fn into_leaf_value(self) -> Option<T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }
}

impl<T: PartialEq> PartialEq for Child<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Child::None, Child::None) => true,
            (Child::Octant(l), Child::Octant(r)) => l == r,
            (Child::Leaf(l), Child::Leaf(r)) => l == r,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Octant<T> {
    pub parent: Option<OctantId>,
    pub child_count: u8,
    pub children: [Child<T>; 8],
}

impl<T: Copy> Octant<T> {
    pub fn set_child(&mut self, idx: u8, child: Child<T>) -> Child<T> {
        let idx = idx as usize;
        if self.children[idx].is_none() && !child.is_none() {
            self.child_count += 1;
        }
        if !self.children[idx].is_none() && child.is_none() {
            self.child_count -= 1;
        }
        let mut child = child;

        mem::swap(&mut child, &mut self.children[idx]);
        child
    }
}

#[derive(Debug, Clone, Default)]
pub struct Octree<T: Copy> {
    pub octree_scale: f32,
    pub root: Option<OctantId>,
    pub octants: Vec<Octant<T>>,
    pub free_list: Vec<OctantId>,
    pub depth: u8,
}

impl<T: Copy> Octree<T> {
    pub fn new() -> Self {
        Self::new_in()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity)
    }
}

impl<T: PartialEq + Copy> PartialEq for Octree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root.eq(&other.root)
            && self.octants.eq(&other.octants)
            && self.free_list.eq(&other.free_list)
            && self.depth.eq(&other.depth)
    }
}

impl<T: Copy> Octree<T> {
    fn new_in() -> Self {
        Self::with_capacity_in(0)
    }

    fn with_capacity_in(capacity: usize) -> Self {
        Self {
            octree_scale: f32::exp2(-0.0),
            root: None,
            octants: Vec::with_capacity(capacity),
            free_list: Vec::new(),
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
                let prev = self.octants[it as usize].set_child(idx, Child::Leaf(leaf));
                return (LeafId { parent: it, idx }, prev.into_leaf_value());
            }
            it = self.step_into_or_create_octant_at(it, idx);
        }
        unreachable!()
    }

    pub fn get_leaf(&self, pos: impl Position) -> Option<&T> {
        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.powi(self.depth as i32) as u32;

        while size > 0 {
            size /= 2;
            let idx = pos.div(size).idx();
            //println!("it: {}, pos: {:?} idx: {},size: {}", it, pos, idx, size);
            pos.rem_assign(size);

            let child = &self.octants[it as usize].children[idx as usize];
            if child.is_none() {
                //println!("is none! size:{}", size);
                break;
            }

            match &self.octants[it as usize].children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(val) => return Some(val),
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
        &mut self,
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
                self.octants[*parent_id as usize].set_child(i, Child::Octant(child_id));

                let child = &mut self.octants[child_id as usize];
                child.parent = Some(*parent_id);

                return;
            }

            if let Some(value) = f(child_pos) {
                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Leaf(value));
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

                let old_leaf = self.octants[it as usize].set_child(leaf_id.idx, Child::None);

                let new_leaf =
                    self.octants[leaf_id.parent as usize].set_child(leaf_id.idx, Child::None);

                if new_leaf.get_leaf_value().is_some() {
                    self.octants[it as usize].set_child(idx, new_leaf);
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

            match &self.octants[it as usize].children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(_) => match self.octants[it as usize].set_child(idx, Child::None) {
                    Child::None => return (None, None),
                    Child::Octant(_) => unreachable!(),
                    Child::Leaf(val) => return (Some(val), Some(LeafId { parent: it, idx })),
                },
            }
        }
        (None, None)
    }

    pub fn remove_leaf_by_id(&mut self, leaf_id: LeafId) -> Option<T> {
        match self.octants[leaf_id.parent as usize].children[leaf_id.idx as usize] {
            Child::None | Child::Octant(_) => None,
            Child::Leaf(_) => {
                match self.octants[leaf_id.parent as usize].set_child(leaf_id.idx, Child::None) {
                    Child::None => None,
                    Child::Octant(_) => unreachable!("found unexpected octant"),
                    Child::Leaf(val) => Some(val),
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.root = None;
        self.octants.clear();
        self.free_list.clear();
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
                self.octants[root_id as usize].parent = Some(new_root_id);
                self.octants[new_root_id as usize].set_child(0, Child::Octant(root_id));
            }
            self.root = Some(new_root_id)
        }
        self.depth += by
    }

    pub(crate) fn new_octant(&mut self, parent: Option<OctantId>) -> OctantId {
        if let Some(free_id) = self.free_list.pop() {
            self.octants[free_id as usize].parent = parent;
            return free_id;
        }
        let id = self.octants.len() as OctantId;
        self.octants.push(Octant {
            parent,
            child_count: 0,
            children: Default::default(),
        });
        id
    }

    pub fn compact(&mut self) {
        if self.root.is_none() {
            return;
        }
        self.compact_octant(self.root.unwrap());

        if self.octants[self.root.unwrap() as usize].child_count != 0 {
            return;
        }

        self.reset();
    }

    fn compact_octant(&mut self, octant_id: OctantId) {
        let children = self.octants[octant_id as usize].children.len();

        for i in 0..children {
            let id = {
                let octant = &self.octants[octant_id as usize];
                octant.children[i].get_octant_value()
            };
            if id.is_none() {
                continue;
            }
            let id = id.unwrap();

            self.compact_octant(id);

            let octant = &self.octants[id as usize];
            if octant.child_count == 0 {
                self.delete_octant(id);
                self.octants[octant_id as usize].set_child(i as u8, Child::None);
            }
        }
    }

    fn delete_octant(&mut self, id: OctantId) {
        if let Some(parent) = self.octants[id as usize].parent {
            let children = &self.octants[parent as usize].children;
            let idx = children
                .iter()
                .position(|x| x.get_octant_value() == Some(id));

            if let Some(idx) = idx {
                self.octants[parent as usize].set_child(idx as u8, Child::None);
            }
        }

        let octant = &mut self.octants[id as usize];
        octant.parent = None;
        octant.child_count = 0;

        for child in &mut octant.children {
            *child = Child::None;
        }

        self.free_list.push(id);
    }

    fn step_into_or_create_octant_at(&mut self, it: OctantId, idx: u8) -> OctantId {
        match &self.octants[it as usize].children[idx as usize] {
            Child::None => {
                let prev_id = it;
                let next_id = self.new_octant(Some(prev_id));
                let prev_octant: &mut Octant<T> = &mut self.octants[prev_id as usize];
                prev_octant.set_child(idx, Child::Octant(next_id));

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
impl Octree<ResourceModel> {
    pub fn load_mc_world<P: Position>(
        origin: BlockCoords,
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
            dbg!(&child_pos);
            let region_coords: RegionCoords = BlockCoords {
                x: child_pos.x() as i64 + offset.x,
                y: child_pos.y() as i64 + offset.y,
                z: child_pos.z() as i64 + offset.z,
            }
            .into();
            dbg!(region_coords);
            if let Some(region) = world.get_region_lazy(region_coords) {
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
        region: &LazyRegion,
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
            let chunk_coords: ChunkCoords = BlockCoords {
                x: child_pos.x() as i64 + offset.x,
                y: child_pos.y() as i64 + offset.y,
                z: child_pos.z() as i64 + offset.z,
            }
            .into();

            if let Some(chunk) = region.get_chunk(chunk_coords) {
                if let Some(value) =
                    self.construct_chunk_level(size, &chunk, model_manager, offset, child_pos)
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
            let child_pos = BlockCoords {
                x: ((pos.x() + ((child_idx as u32) & 1)) as i64) + offset.x,
                y: ((pos.y() + ((child_idx as u32 >> 1) & 1)) as i64) + offset.y,
                z: ((pos.z() + ((child_idx as u32 >> 2) & 1)) as i64) + offset.z,
            };
            if let Some(block) = todo!() {
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

impl<T: Copy> From<ParallelOctree<T>> for Octree<T> {
    fn from(value: ParallelOctree<T>) -> Self {
        let ParallelOctree {
            octree_scale,
            root,
            octants,
            free_list,
            depth,
        } = value;
        let lock = free_list.lock().unwrap();
        let octants_len = octants.len();
        dbg!(octants_len);
        let octants_vec: Vec<_> = (0..octants_len).map(|i| octants[i].get()).collect();
        Self {
            octree_scale: octree_scale,
            root: root,
            octants: octants_vec,
            free_list: lock.clone(),
            depth: depth,
        }
    }
}

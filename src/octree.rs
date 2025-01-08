use std::{
    fmt::Debug,
    hash::Hash,
    mem,
    ops::{Div, RemAssign},
};

use glam::{UVec3, Vec3A};
use rand_distr::num_traits::Pow;

use crate::Ray;

pub type OctantId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LeafId {
    pub parent: OctantId,
    pub idx: u8,
}

pub trait Position:
    Div<u32, Output = Self> + RemAssign<u32> + Copy + Clone + Eq + PartialEq + Debug + Hash
{
    fn construct(pos: [u32; 3]) -> Self;
    fn idx(&self) -> u8;
    fn required_depth(&self) -> u8;
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn z(&self) -> u32;
}

impl Position for UVec3 {
    fn idx(&self) -> u8 {
        (self.x + self.y * 2 + self.z * 4) as u8
    }
    fn required_depth(&self) -> u8 {
        let depth = self.max_element();
        (depth as f32).log2().floor() as u8 + 1
    }

    fn construct(pos: [u32; 3]) -> Self {
        Self::new(pos[0], pos[1], pos[2])
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
}

#[derive(Debug, Default)]
pub(super) enum Child<T> {
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

#[derive(Debug, PartialEq)]
pub(super) struct Octant<T> {
    parent: Option<OctantId>,
    child_count: u8,
    pub(super) children: [Child<T>; 8],
}

impl<T> Octant<T> {
    fn set_child(&mut self, idx: u8, child: Child<T>) -> Child<T> {
        let idx = idx as usize;
        if self.children[idx].is_none() && child.is_none() {
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

#[derive(Debug)]
pub struct Octree<T> {
    pub(super) root: Option<OctantId>,
    pub(super) octants: Vec<Octant<T>>,
    free_list: Vec<OctantId>,
    depth: u8,
}

impl<T> Octree<T> {
    pub fn new() -> Self {
        Self::new_in()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity)
    }
}

impl<T: PartialEq> PartialEq for Octree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root.eq(&other.root)
            && self.octants.eq(&other.octants)
            && self.free_list.eq(&other.free_list)
            && self.depth.eq(&other.depth)
    }
}

impl<T> Octree<T> {
    pub fn new_in() -> Self {
        Self::with_capacity_in(0)
    }

    pub fn with_capacity_in(capacity: usize) -> Self {
        Self {
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
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = (pos / size).idx();
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
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size > 0 {
            size /= 2;
            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            let child = &self.octants[it as usize].children[idx as usize];
            if child.is_none() {
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

        let size = 2f32.pow(depth as i32) as u32;

        if let Some(result) =
            self.construct_octants_with_impl(size, P::construct([0u32, 0u32, 0u32]), &f)
        {
            self.root = Some(result);
            self.depth = depth;
        }
    }

    fn construct_octants_with_impl<P: Position, F: Fn(P) -> Option<T>>(
        &mut self,
        size: u32,
        pos: P,
        f: &F,
    ) -> Option<OctantId> {
        let size = size / 2;

        let mut new_parent = None;

        for i in 0u8..8 {
            let child_pos = P::construct([
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32) & 1),
                pos.z() + size * ((i as u32) & 1),
            ]);
            if size > 1 {
                let child_id = self.construct_octants_with_impl(size, child_pos, f);
                let Some(child_id) = child_id else {
                    continue;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));

                let child = &mut self.octants[child_id as usize];

                child.parent = Some(*parent_id);
                continue;
            }
            if let Some(value) = f(child_pos) {
                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Leaf(value));
            }
        }

        new_parent
    }

    pub fn move_leaf(&mut self, leaf_id: LeafId, to_pos: impl Position) -> (LeafId, Option<T>) {
        self.expand_to(to_pos.required_depth());

        let mut it = self.root.unwrap();
        let mut pos = to_pos;
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = (pos / size).idx();
            pos %= size;

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
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = (pos / size).idx();
            pos %= size;

            match &self.octants[it as usize].children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(_) => match self.octants[it as usize].set_child(idx, Child::None) {
                    Child::None => return (None, None),
                    Child::Octant(_) => unreachable!(),
                    Child::Leaf(val) => {
                        return (
                            Some(val),
                            Some(LeafId {
                                parent: it,
                                idx: idx,
                            }),
                        )
                    }
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
            todo!()
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

    fn new_octant(&mut self, parent: Option<OctantId>) -> OctantId {
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
                let prev_octant = &mut self.octants[prev_id as usize];
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

    const MAX_STEPS: usize = 1000;
    const MAX_SCALE: usize = 23;
    const EPSILON: f32 = 0.00000011920929;

    pub fn intersect_octree(&self, ray: &mut Ray, max_dst: f32, do_translucensy: bool) {
        let ptr_stack: [u32; 24] = [Default::default(); Self::MAX_SCALE + 1];
        let parent_octant_idx_stack: [u32; 24] = [Default::default(); Self::MAX_SCALE + 1];
        let t_max_stack: [f32; 24] = [Default::default(); Self::MAX_SCALE + 1];
        let octree_scale: f32 = 2.0f32.pow(-(self.depth() as i32));

        let mut ro: Vec3A = ray.origin * octree_scale;
        let mut rd: Vec3A = ray.direction.normalize();
        let max_dst: f32 = max_dst * octree_scale;

        ro += 1.0;

        let mut ptr: u32 = 0;
        let mut parent_octant_idx: u32 = 0;

        let scale = Self::MAX_SCALE - 1;
        let scale_exp2: f32 = 0.5f32;

        let last_leaf_value: u32 = -1i32 as u32;
        let adjacent_leaf_count: u32 = 0;

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign = Self::EPSILON.to_bits() & !sign_mask;
        if rd.x.abs() < Self::EPSILON {
            rd.x = f32::from_bits(epsilon_bits_without_sign | (rd.x.to_bits() & sign_mask))
        }
        if rd.y.abs() < Self::EPSILON {
            rd.y = f32::from_bits(epsilon_bits_without_sign | (rd.y.to_bits() & sign_mask))
        }
        if rd.x.abs() < Self::EPSILON {
            rd.z = f32::from_bits(epsilon_bits_without_sign | (rd.z.to_bits() & sign_mask))
        }

        let t_coef: glam::Vec3A = 1.0 - rd.abs();
        let mut t_bias = t_coef * ro;

        let mut octant_mask: u32 = 0u32;
        if rd.x > 0.0 {
            octant_mask ^= 1;
            t_bias.x = 3.0 * t_coef.x - t_bias.x;
        }
        if rd.y > 0.0 {
            octant_mask ^= 1;
            t_bias.y = 3.0 * t_coef.y - t_bias.y;
        }
        if rd.z > 0.0 {
            octant_mask ^= 1;
            t_bias.z = 3.0 * t_coef.z - t_bias.z;
        }

        let mut t_min = (2.0 * t_coef.x - t_bias.x)
            .max(2.0 * t_coef.y - t_bias.y)
            .max(2.0 * t_coef.z - t_bias.z);

        t_min = 0.0f32.max(t_min);

        let t_max = (t_coef.x - t_bias.x)
            .min(t_coef.y - t_bias.y)
            .min(t_coef.z - t_bias.z);
        let h = t_max;

        let mut idx = 0;

        let mut pos = Vec3A::splat(1.0);

        if t_min < 1.5 * t_coef.x - t_bias.x {
            idx ^= 1;
            pos.x = 1.5;
        }
        if t_min < 1.5 * t_coef.y - t_bias.y {
            idx ^= 2;
            pos.y = 1.5;
        }
        if t_min < 1.5 * t_coef.z - t_bias.z {
            idx ^= 4;
            pos.z = 1.5;
        }

        let mut it = self.root.unwrap();
        for i in (0..Self::MAX_STEPS) {
            if max_dst >= 0.0 && t_min > max_dst {
                return;
            }

            let t_corner: Vec3A = pos * t_coef - t_bias;

            let tc_max: f32 = t_corner.min_element();

            let octant_idx: u32 = idx ^ octant_mask;
            let bit: u32 = 1 << octant_idx;

            let is_leaf = self.octants[it as usize].children[octant_idx as usize].is_leaf();
            let is_child = !self.octants[it as usize].children[octant_idx as usize].is_none();

            if is_child && t_min <= t_max {
                
            }
        }
    }
}
